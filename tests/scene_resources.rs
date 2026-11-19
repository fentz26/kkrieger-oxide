use kkrieger_oxide::{gpu_scene::Resources, kx::Document};
#[test]
#[ignore = "precalculates authored gameplay paint and texture dependencies"]
fn authored_gameplay_resources_include_live_texture_slots_and_font_side_effects() {
    let doc = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    let resources = Resources::prepare(&doc, doc.roots[2].unwrap(), true, 0).unwrap();
    assert_eq!(resources.paint.meshes.len(), 169);
    assert_eq!(resources.paint.materials.len(), 67);
    for material in resources.paint.materials.values() {
        for pass in &material.passes {
            for texture in pass.shader.state.borrow().textures.into_iter().flatten() {
                assert!(
                    resources.bitmaps.contains_key(&texture),
                    "missing bitmap {texture}"
                );
            }
        }
    }
    assert!(resources.pages.contains_key(&1));
    eprintln!(
        "authored resources: {} meshes, {} materials, {} bitmaps, {} font pages",
        resources.paint.meshes.len(),
        resources.paint.materials.len(),
        resources.bitmaps.len(),
        resources.pages.len()
    );
}

#[test]
fn empty_selected_scene_has_no_unrelated_assets_and_invalid_roots_fail() {
    let mut doc = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    doc.operators.truncate(1);
    let scene = doc
        .classes
        .iter()
        .position(|class| class.id == 0xc1)
        .unwrap();
    doc.operators[0].class = scene;
    doc.operators[0].inputs.clear();
    doc.operators[0].links.clear();
    let resources = Resources::prepare(&doc, 0, true, 0).unwrap();
    assert!(resources.paint.meshes.is_empty());
    assert!(resources.paint.materials.is_empty());
    assert!(resources.bitmaps.is_empty());
    assert!(resources.pages.is_empty());
    assert!(Resources::prepare(&doc, 1, true, 0).is_err());
}

#[test]
fn viewport_simple_mode_replaces_mesh_usage_mask_without_filtering_effects() {
    use kkrieger_oxide::gpu_scene::usage_mask;
    assert_eq!(usage_mask(0, 0x1234), Some(0x1234));
    assert_eq!(usage_mask(1, 0), Some(1 << 7));
    assert_eq!(usage_mask(2, 0x1234), Some(0x1234));
    assert_eq!(usage_mask(3, u32::MAX), None);
}

#[test]
#[ignore = "requires independently extracted beta; prepares its authored player roots"]
fn released_beta_player_roots_prepare_render_resources() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let bytes = std::fs::read(".reference/verified-beta.kx").unwrap();
            let doc = Document::parse_layout(&bytes, kkrieger_oxide::kx::Layout::Beta2004).unwrap();
            let roots: std::collections::BTreeSet<_> =
                doc.roots.iter().take(3).flatten().copied().collect();
            assert!(!roots.is_empty());
            let mut meshes = 0;
            for root in roots {
                let resources = Resources::prepare(&doc, root, true, 0)
                    .unwrap_or_else(|error| panic!("released root {root}: {error}"));
                for asset in resources.paint.meshes.values() {
                    assert!(
                        asset
                            .mesh
                            .vertices
                            .iter()
                            .all(|v| v.position.iter().all(|value| value.is_finite())),
                        "non-finite vertex in root {root}"
                    );
                    for job in &asset.mesh.jobs {
                        assert!(
                            job.vertices
                                .iter()
                                .all(|&index| index < asset.mesh.vertices.len())
                        );
                        let scale = if job.program == kkrieger_oxide::material::Program::Shadow {
                            2
                        } else {
                            1
                        };
                        assert!(
                            job.indices
                                .iter()
                                .all(|&index| index < job.vertices.len() * scale)
                        );
                        assert_eq!(job.indices.len() % 3, 0);
                    }
                }
                for material in resources.paint.materials.values() {
                    for pass in &material.passes {
                        for texture in pass
                            .shader
                            .textures
                            .into_iter()
                            .chain(pass.shader.state.borrow().textures)
                            .flatten()
                        {
                            assert!(
                                resources.bitmaps.contains_key(&texture),
                                "root {root}: missing bitmap {texture}"
                            );
                        }
                    }
                }
                meshes += resources.paint.meshes.len();
                eprintln!(
                    "released root {root}: {} meshes, {} materials, {} bitmaps, {} font pages",
                    resources.paint.meshes.len(),
                    resources.paint.materials.len(),
                    resources.bitmaps.len(),
                    resources.pages.len()
                );
            }
            assert!(meshes > 0);
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
#[ignore = "requires extracted beta; native startup parameter ordering remains pending"]
fn released_beta_player_startup_initializes_native_simulation() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            use kkrieger_oxide::{
                collision::world::World,
                game::{root, simulation::Simulation},
                random::Random,
                scene::exec::Execution,
            };
            let bytes = std::fs::read(".reference/verified-beta.kx").unwrap();
            let doc = Document::parse_layout(&bytes, kkrieger_oxide::kx::Layout::Beta2004).unwrap();
            let root = doc.roots[0].unwrap();
            let mut execution = Execution::new_player(&doc);
            execution.prepare_game_operators(&doc, root).unwrap();
            let position = execution.game.parameters.start_position;
            let scene = kkrieger_oxide::game::root::painter_scene(&doc, root)
                .unwrap()
                .unwrap();
            assert_eq!((root, scene), (4815, 4357));
            let parameter = &doc.operators[doc.operators[scene].inputs[0]];
            assert_eq!(
                position,
                [
                    parameter.float(29).unwrap(),
                    parameter.float(30).unwrap(),
                    parameter.float(31).unwrap(),
                    0.
                ]
            );
            let world = World::from_document(&doc, scene).unwrap();
            let mut simulation =
                Simulation::new(world, &mut execution, position, 0, [0, 0], Random::new(1))
                    .unwrap();
            simulation.restart(&mut execution, [0, 0]).unwrap();
            execution.switches[1] = 8;
            assert!(simulation.released_actors.is_some());
            assert!(simulation.motion.released_player.is_some());
            assert!(simulation.player_cell.is_some());
            assert!(position.iter().all(|value| value.is_finite()));
            assert_eq!(root::painter_scene(&doc, root).unwrap(), Some(scene));
            eprintln!("released startup: root {root}, scene {scene}, position {position:?}");
        })
        .unwrap()
        .join()
        .unwrap();
}
