use kkrieger_oxide::{
    bitmap::FontPage,
    collision::world::World,
    game::simulation::Simulation,
    kx::Document,
    random::Random,
    scene::Error,
    scene::{
        culling::Frustum,
        effects::{Content, Rendering},
        exec::{Execution, Job, RenderCommand, Renderer},
        light,
        overlay::{Pool, Viewport},
        paint::{self, DrawKind},
    },
    texture,
};
#[derive(Default, Debug)]
struct RenderStats {
    particle_packets: usize,
    particle_quads: usize,
    print_packets: usize,
    print_vertices: usize,
    prepared_lights: usize,
    max_lights: usize,
    mesh_paints: usize,
    effect_jobs: usize,
    scene_passes: usize,
    overlay_commands: usize,
}
struct AuthoredRenderer {
    assets: paint::Assets,
    pages: std::collections::BTreeMap<u8, FontPage>,
    stats: std::rc::Rc<std::cell::RefCell<RenderStats>>,
}
impl Renderer for AuthoredRenderer {
    fn render(
        &mut self,
        e: &mut Execution,
        d: &Document<'_>,
        command: RenderCommand,
    ) -> Result<(), Error> {
        let mut stats = self.stats.borrow_mut();
        let RenderCommand::Scene(index) = command else {
            if matches!(command, RenderCommand::Overlay(_)) {
                stats.overlay_commands += 1;
            }
            return Ok(());
        };
        stats.scene_passes += 1;
        let pass = e.render_passes[index].clone();
        let materials = &self.assets.materials;
        let [x0, y0, x1, y1] = pass.viewport.window;
        let lighting = light::Context::from_camera(&pass.camera, [x1 - x0, y1 - y0], true).unwrap();
        let mut visible_lights = 0;
        let mut paint_lights = Vec::new();
        for job in &pass.jobs {
            if !matches!(job, Job::Light { .. }) {
                continue;
            }
            if let Some(light) = lighting.prepare(job).unwrap() {
                visible_lights += 1;
                paint_lights.push(light.paint_light());
                assert!(light.importance.is_finite() && light.amplify.is_finite());
                assert!(light.rect.into_iter().all(f32::is_finite));
                for frustum in [&light.shadow, &light.zfail] {
                    assert!(
                        frustum.planes[..frustum.count]
                            .iter()
                            .all(|p| p.equation.into_iter().all(f32::is_finite))
                    );
                }
            }
        }
        stats.prepared_lights += visible_lights;
        stats.max_lights = stats.max_lights.max(visible_lights);
        let environment = pass
            .camera
            .material_environment([x1 - x0, y1 - y0])
            .unwrap();
        let frustum =
            Frustum::from_view_projection(environment.view_projection, [-1., -1., 1., 1.]);
        // Exercise every visible candidate here; light selection profiles
        // are verified separately against their source/beta implementations.
        let view = paint::View {
            usage_mask: u32::MAX,
            frustum: frustum.planes[..5].try_into().unwrap(),
            lights: &paint_lights,
            light_capacity: 16,
        };
        let frame = paint::build_static_frame(
            d,
            &pass.jobs,
            &self.assets.meshes,
            materials,
            &view,
            |_, _| 0,
        )
        .unwrap();
        assert!(
            frame
                .jobs
                .windows(2)
                .all(|pair| pair[0].sort_key <= pair[1].sort_key)
        );
        let rendering = std::rc::Rc::new(Rendering {
            camera: pass.camera.clone(),
            materials: materials.clone(),
            pages: self.pages.clone(),
            fps: 60.,
        });
        let mut packets = Vec::new();
        for job in frame.jobs {
            assert!(matches!(pass.jobs[job.scene_job], Job::Paint { .. }));
            match job.kind {
                DrawKind::Mesh(_) => stats.mesh_paints += 1,
                DrawKind::Effect => {
                    stats.effect_jobs += 1;
                    packets.extend(
                        e.draw_deferred_effect(
                            d,
                            &pass.jobs[job.scene_job],
                            std::rc::Rc::clone(&rendering),
                        )
                        .unwrap(),
                    );
                }
            }
        }
        for packet in packets {
            match packet.content {
                Content::Particles(quads) => {
                    stats.particle_packets += 1;
                    stats.particle_quads += quads.len();
                    for quad in quads {
                        assert!(
                            quad.iter()
                                .all(|v| v[..3].iter().chain(&v[6..]).all(|f| f.is_finite()))
                        );
                    }
                }
                Content::Print(draw) => {
                    stats.print_packets += 1;
                    stats.print_vertices += draw.geometry.vertices.len();
                    assert!(
                        draw.geometry
                            .vertices
                            .iter()
                            .all(|v| v[..3].iter().chain(&v[6..]).all(|f| f.is_finite()))
                    );
                    assert!(
                        draw.geometry
                            .indices
                            .iter()
                            .all(|&i| usize::from(i) < draw.geometry.vertices.len())
                    );
                }
            }
        }
        Ok(())
    }
}
#[test]
#[ignore = "generates the full authored collision world"]
fn authored_gameplay_root_executes_movement_and_firing_ticks() {
    let d = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    let root = d.roots[2].unwrap();
    let assets = paint::prepare_scene_assets(&d, root, true).unwrap();
    let mut needed = vec![false; d.operators.len()];
    let mut pending = vec![root];
    while let Some(i) = pending.pop() {
        if needed[i] {
            continue;
        }
        needed[i] = true;
        pending.extend(d.operators[i].inputs.iter().copied());
        pending.extend(d.operators[i].links.iter().flatten().copied());
    }
    let font_roots: Vec<_> = d
        .operators
        .iter()
        .enumerate()
        .filter(|(i, op)| {
            needed[*i] && d.classes[op.class].id == 0x31 && op.parameters[5] & 0x70 != 0
        })
        .map(|(i, _)| i)
        .collect();
    let fonts = texture::generate_many_with_fonts(&d, &font_roots, 0).unwrap();
    let stats = std::rc::Rc::new(std::cell::RefCell::new(RenderStats::default()));
    let mut e = Execution::new_player(&d);
    e.renderer = Some(Box::new(AuthoredRenderer {
        assets,
        pages: fonts.font_pages,
        stats: std::rc::Rc::clone(&stats),
    }));
    e.prepare_game_operators(&d, root).unwrap();
    let position = e.game.parameters.start_position;
    assert_eq!(position, [0., 1.75, -21., 1.]);
    let world = World::from_document(&d, 5171).unwrap();
    let mut sim = Simulation::new(world, &mut e, position, 0, [0, 0], Random::new(1)).unwrap();
    sim.restart(&mut e, [0, 0]).unwrap();
    e.overlay = Some(Pool::new(
        Viewport {
            screen: 0,
            render_target: -1,
            window: [0, 0, 640, 360],
        },
        std::array::from_fn(|i| 100 + i as i32),
    ));
    sim.prepare_scene(&mut e, 0, 0, 1., false).unwrap();
    sim.execute_scene(&mut e, &d, root).unwrap();
    assert_eq!(e.game.monsters.len(), 132);
    assert_eq!(e.render_passes.len(), 2);
    assert!(sim.player_cell.is_some());
    assert_eq!(sim.motion.collider.position, position);
    // Original refill/unlock inputs allow selecting an ammo-consuming weapon.
    sim.key(&mut e, 109, [0, 0]).unwrap();
    sim.key(&mut e, 110, [0, 0]).unwrap();
    sim.key(&mut e, 50, [0, 0]).unwrap();
    assert_eq!(e.game.weapon.next, 1);
    assert!(e.game.events.shot[1].is_some());
    let ammo = e.game.inventory.amounts;
    e.finish_events();
    sim.key(&mut e, 119, [0, 0]).unwrap();
    sim.key(&mut e, kkrieger_oxide::game::input::MOUSE_LEFT, [0, 0])
        .unwrap();
    for step in 1..=30 {
        let now = step * 10;
        assert!(sim.tick(&mut e, &d, 1, now, [0, 0]).unwrap().ran);
        sim.prepare_scene(&mut e, now * 64, now, 1., false).unwrap();
        sim.execute_scene(&mut e, &d, root).unwrap();
        assert_eq!(e.game.monsters.len(), 132);
        assert_eq!(e.render_passes.len(), 2);
        assert!(sim.player_cell.is_some());
        assert!(sim.motion.collider.position.into_iter().all(f32::is_finite));
        e.finish_events();
    }
    let stats = stats.borrow();
    eprintln!("authored synchronous render: {stats:?}");
    assert_eq!(stats.scene_passes, 62);
    assert_eq!(stats.overlay_commands, 0);
    assert!(stats.particle_packets > 0);
    assert!(stats.particle_quads > 0);
    assert!(stats.print_packets > 0);
    assert!(stats.print_vertices > 0);
    assert!(stats.prepared_lights > 0);
    assert!(stats.mesh_paints > 0);
    assert!(stats.effect_jobs > 0);
    assert_eq!(sim.tick_count, 30);
    assert!(sim.motion.collider.position[2] > position[2]);

    assert!(
        e.game.inventory.amounts[4..]
            .iter()
            .zip(&ammo[4..])
            .any(|(after, before)| after < before),
        "firing must consume ammunition: current={} next={} timer={} fire={} before={:?} after={:?}",
        e.game.weapon.current,
        e.game.weapon.next,
        e.game.weapon.timer,
        e.game.weapon.fire_key,
        ammo,
        e.game.inventory.amounts
    );
}
