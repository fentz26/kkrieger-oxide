//! Offline authored-scene GPU integration check; not the game executable.
//! Exercises all visible light candidates with capacity 16. Published-source
//! KKRIEGER replacement/count behavior and released-beta selection stay separate.
use kkrieger_oxide::{
    collision::world::World,
    game::simulation::Simulation,
    gpu_scene, gpu_texture,
    kx::Document,
    random::Random,
    scene::{
        Error,
        exec::{Execution, Job, RenderCommand, Renderer},
        light,
        overlay::{Pool, Viewport},
    },
};
use miniquad::*;
use std::{cell::RefCell, collections::BTreeMap, io::Write, rc::Rc};
struct State {
    ctx: Box<dyn RenderingBackend>,
    scene: Option<gpu_scene::Scene>,
    passes: usize,
    targets: BTreeMap<(i32, i32), (RenderPass, TextureId)>,
}
struct CheckRenderer(Rc<RefCell<State>>);
impl Renderer for CheckRenderer {
    fn render(
        &mut self,
        execution: &mut Execution,
        doc: &Document<'_>,
        command: RenderCommand,
    ) -> Result<(), Error> {
        let RenderCommand::Scene(index) = command else {
            return Ok(());
        };
        let pass = execution.render_passes[index].clone();
        let [x0, y0, x1, y1] = pass.viewport.window;
        let dimensions = [x1 - x0, y1 - y0];
        let lighting = light::Context::from_camera(&pass.camera, dimensions, true).unwrap();
        let lights: Vec<_> = pass
            .jobs
            .iter()
            .filter(|job| matches!(job, Job::Light { .. }))
            .filter_map(|job| lighting.prepare(job).unwrap())
            .collect();
        let mut state = self.0.borrow_mut();
        let State {
            ctx,
            scene,
            passes,
            targets,
        } = &mut *state;
        let ctx = &mut **ctx;
        let key = (pass.viewport.screen, pass.viewport.render_target);
        let fresh = !targets.contains_key(&key);
        let (target, color) = *targets.entry(key).or_insert_with(|| {
            gpu_texture::depth_stencil_pass(ctx, dimensions[0] as u32, dimensions[1] as u32)
                .unwrap()
        });
        let rgba = [
            (pass.clear_color >> 16) & 255,
            (pass.clear_color >> 8) & 255,
            pass.clear_color & 255,
            pass.clear_color >> 24,
        ]
        .map(|v| v as f32 / 255.);
        gpu_texture::begin_pass(
            ctx,
            Some(target),
            PassAction::Clear {
                color: if pass.clear_flags & 1 != 0 {
                    Some((rgba[0], rgba[1], rgba[2], rgba[3]))
                } else if fresh {
                    Some((0., 0., 0., 0.))
                } else {
                    None
                },
                depth: (fresh || pass.clear_flags & 2 != 0).then_some(1.),
                stencil: (fresh || pass.clear_flags & 2 != 0).then_some(0),
            },
        );
        let options = gpu_scene::Options {
            light_capacity: 16,
            fps: 60.,
            ..Default::default()
        };
        let drawn = scene
            .as_mut()
            .unwrap()
            .draw(ctx, execution, doc, &pass, &lights, &options)
            .unwrap();
        ctx.end_render_pass();
        let mut pixels = vec![0; dimensions[0] as usize * dimensions[1] as usize * 4];
        ctx.texture_read_pixels(color, &mut pixels);
        assert!(
            pixels
                .as_chunks::<4>()
                .0
                .iter()
                .any(|pixel| pixel[..3].iter().any(|&byte| byte != 0)),
            "authored viewport is empty"
        );
        let path = format!(".reference/authored-frame-{passes}.ppm");
        let mut file = std::fs::File::create(&path).unwrap();
        writeln!(file, "P6\n{} {}\n255", dimensions[0], dimensions[1]).unwrap();
        for row in pixels.chunks_exact(dimensions[0] as usize * 4).rev() {
            for pixel in row.as_chunks::<4>().0 {
                file.write_all(&pixel[..3]).unwrap();
            }
        }
        println!(
            "authored viewport {passes}: {drawn:?}, {} visible light candidates, target={key:?}, clear={}, {path}",
            lights.len(),
            pass.clear_flags
        );
        *passes += 1;
        Ok(())
    }
}
struct Check;
impl EventHandler for Check {
    fn update(&mut self) {}
    fn draw(&mut self) {}
}
fn main() {
    let doc = Document::parse(include_bytes!("../../assets/kkrieger3383.kx")).unwrap();
    let root = doc.roots[2].unwrap();
    let resources = gpu_scene::Resources::prepare(&doc, root, true, 0).unwrap();
    let mut execution = Execution::new_player(&doc);
    execution.prepare_game_operators(&doc, root).unwrap();
    let position = execution.game.parameters.start_position;
    let world = World::from_document(&doc, 5171).unwrap();
    let mut simulation =
        Simulation::new(world, &mut execution, position, 0, [0, 0], Random::new(1)).unwrap();
    simulation.restart(&mut execution, [0, 0]).unwrap();
    execution.overlay = Some(Pool::new(
        Viewport {
            screen: 0,
            render_target: -1,
            window: [0, 0, 640, 360],
        },
        std::array::from_fn(|i| 100 + i as i32),
    ));
    miniquad::start(
        conf::Conf {
            window_title: "kkrieger-oxide authored frame check".into(),
            window_width: 64,
            window_height: 64,
            ..Default::default()
        },
        move || {
            let checked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let mut ctx = window::new_rendering_backend();
                let scene = gpu_scene::Scene::new(&mut *ctx, resources, 24).unwrap();
                let state = Rc::new(RefCell::new(State {
                    ctx,
                    scene: Some(scene),
                    passes: 0,
                    targets: Default::default(),
                }));
                execution.renderer = Some(Box::new(CheckRenderer(Rc::clone(&state))));
                simulation
                    .prepare_scene(&mut execution, 0, 0, 1., false)
                    .unwrap();
                simulation
                    .execute_scene(&mut execution, &doc, root)
                    .unwrap();
                assert_eq!(state.borrow().passes, 2);
                drop(execution.renderer.take());
                let mut state = state.borrow_mut();
                let scene = state.scene.take().unwrap();
                scene.delete(&mut *state.ctx);
                for (_, (target, _)) in std::mem::take(&mut state.targets) {
                    state.ctx.delete_render_pass(target);
                }
                window::quit();
            }));
            if checked.is_err() {
                std::process::exit(1);
            }
            Box::new(Check)
        },
    );
}
