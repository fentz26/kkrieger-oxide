//! Offline authored-scene GPU integration check; not the game executable.
//! Default exercises every visible candidate with capacity 16. Optional
//! --beta-light-order uses the verified released ordering/four-light clamp;
//! geometric/visibility preparation still follows the published source.
use kkrieger_oxide::{
    collision::world::World,
    game::simulation::Simulation,
    gpu_overlay, gpu_present, gpu_scene, gpu_target,
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
use std::{cell::RefCell, io::Write, rc::Rc};
struct State {
    ctx: Box<dyn RenderingBackend>,
    scene: Option<gpu_scene::Scene>,
    passes: usize,
    capture: [usize; 4],
    beta_light_order: bool,
    menu: bool,
    targets: gpu_target::Targets,
    copies: Option<gpu_overlay::Overlays>,
}
struct CheckRenderer(Rc<RefCell<State>>);
impl Renderer for CheckRenderer {
    fn render(
        &mut self,
        execution: &mut Execution,
        doc: &Document<'_>,
        command: RenderCommand,
    ) -> Result<(), Error> {
        if let RenderCommand::Overlay(index) = command {
            let draw = execution.overlay_draws[index].clone();
            let dimensions = gpu_target::attachment_dimensions(
                execution.overlay.as_ref().unwrap(),
                draw.viewport,
                [640, 360],
            )
            .unwrap();
            let mut state = self.0.borrow_mut();
            let State {
                ctx,
                targets,
                copies,
                ..
            } = &mut *state;
            let sources: Vec<_> = [Some(draw.texture), draw.texture1, draw.texture2]
                .into_iter()
                .flatten()
                .map(|texture| targets.texture(-1, texture).unwrap())
                .collect();
            targets
                .begin(&mut **ctx, draw.viewport, dimensions, 0, 0)
                .unwrap();
            copies
                .as_mut()
                .unwrap()
                .draw_sources(&mut **ctx, &draw, &sources)
                .unwrap();
            ctx.end_render_pass();
            return Ok(());
        }
        let RenderCommand::Scene(index) = command else {
            return Ok(());
        };
        let pass = execution.render_passes[index].clone();
        let [x0, y0, x1, y1] = pass.viewport.window;
        let dimensions = [x1 - x0, y1 - y0];
        let lighting = light::Context::from_camera(&pass.camera, dimensions, true).unwrap();
        let mut lights: Vec<_> = pass
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
            capture,
            beta_light_order,
            menu,
            targets,
            ..
        } = &mut *state;
        if *beta_light_order {
            lights = light::beta_select(lights);
        }
        let ctx = &mut **ctx;
        let key = (pass.viewport.screen, pass.viewport.render_target);
        let color = targets
            .begin(
                ctx,
                pass.viewport,
                [dimensions[0] as u32, dimensions[1] as u32],
                pass.clear_flags,
                pass.clear_color,
            )
            .unwrap();
        let options = gpu_scene::Options {
            light_capacity: if *beta_light_order { 4 } else { 16 },
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
            *menu
                || pixels
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .any(|pixel| pixel[..3].iter().any(|&byte| byte != 0)),
            "authored viewport is empty"
        );
        if capture.contains(passes) {
            let kind = if *menu { "menu" } else { "frame" };
            let path = format!(".reference/authored-{kind}-{passes}.ppm");
            let mut file = std::fs::File::create(&path).unwrap();
            writeln!(file, "P6\n{} {}\n255", dimensions[0], dimensions[1]).unwrap();
            for row in pixels.chunks_exact(dimensions[0] as usize * 4).rev() {
                for pixel in row.as_chunks::<4>().0 {
                    file.write_all(&pixel[..3]).unwrap();
                }
            }
        }
        println!(
            "authored viewport {passes}: {drawn:?}, {} visible light candidates, target={key:?}, clear={}",
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
    let ticks = std::env::args()
        .find_map(|arg| arg.strip_prefix("--ticks=").map(str::parse::<i32>))
        .transpose()
        .unwrap()
        .unwrap_or(0);
    assert!(ticks >= 0);
    let beta_light_order = std::env::args().any(|arg| arg == "--beta-light-order");
    let menu = std::env::args().any(|arg| arg == "--menu");
    assert!(
        !menu || ticks == 0,
        "menu check does not run gameplay ticks"
    );
    let doc = Document::parse(include_bytes!("../../assets/kkrieger3383.kx")).unwrap();
    let root = doc.roots[if menu { 1 } else { 2 }].unwrap();
    let resources = gpu_scene::Resources::prepare(&doc, root, true, 0).unwrap();
    let mut execution = Execution::new_player(&doc);
    execution.prepare_game_operators(&doc, root).unwrap();
    let position = execution.game.parameters.start_position;
    let world = World::from_document(&doc, 5171).unwrap();
    let mut simulation =
        Simulation::new(world, &mut execution, position, 0, [0, 0], Random::new(1)).unwrap();
    simulation.restart(&mut execution, [0, 0]).unwrap();
    if menu {
        execution.switches[1] = 6;
    }
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
                let copies = Some(gpu_overlay::Overlays::new(&mut *ctx, 24).unwrap());
                let state = Rc::new(RefCell::new(State {
                    ctx,
                    scene: Some(scene),
                    passes: 0,
                    capture: [0, 1, ticks as usize * 2, ticks as usize * 2 + 1],
                    beta_light_order,
                    menu,
                    targets: Default::default(),
                    copies,
                }));
                execution.renderer = Some(Box::new(CheckRenderer(Rc::clone(&state))));
                simulation
                    .prepare_scene(&mut execution, 0, 0, 1., false)
                    .unwrap();
                simulation
                    .execute_scene(&mut execution, &doc, root)
                    .unwrap();
                assert_eq!(state.borrow().passes, 2);
                if ticks > 0 {
                    // Same original refill, unlock, weapon-select and input
                    // sequence used by the authored CPU gameplay regression.
                    for key in [109, 110, 50, 119, kkrieger_oxide::game::input::MOUSE_LEFT] {
                        simulation.key(&mut execution, key, [0, 0]).unwrap();
                    }
                }
                let ammo = execution.game.inventory.amounts;
                execution.finish_events();
                for step in 1..=ticks {
                    let now = step * 10;
                    assert!(
                        simulation
                            .tick(&mut execution, &doc, 1, now, [0, 0])
                            .unwrap()
                            .ran
                    );
                    simulation
                        .prepare_scene(&mut execution, now * 64, now, 1., false)
                        .unwrap();
                    simulation
                        .execute_scene(&mut execution, &doc, root)
                        .unwrap();
                    execution.finish_events();
                }
                assert_eq!(state.borrow().passes, 2 * (ticks as usize + 1));
                if ticks > 0 {
                    assert!(simulation.motion.collider.position[2] > position[2]);
                    assert!(
                        execution.game.inventory.amounts[4..]
                            .iter()
                            .zip(&ammo[4..])
                            .any(|(after, before)| after < before)
                    );
                }
                drop(execution.renderer.take());
                let mut state = state.borrow_mut();
                let source = state.targets.texture(0, -1).unwrap();
                if menu {
                    let mut pixels = vec![0; 640 * 360 * 4];
                    state.ctx.texture_read_pixels(source, &mut pixels);
                    let mut file =
                        std::fs::File::create(".reference/authored-menu-final.ppm").unwrap();
                    writeln!(file, "P6\n640 360\n255").unwrap();
                    for row in pixels.as_chunks::<{ 640 * 4 }>().0.iter().rev() {
                        for pixel in row.as_chunks::<4>().0 {
                            file.write_all(&pixel[..3]).unwrap();
                        }
                    }
                }
                let present = gpu_present::Present::new(&mut *state.ctx).unwrap();
                state.ctx.begin_default_pass(PassAction::Nothing);
                present.draw(&mut *state.ctx, source);
                state.ctx.end_render_pass();
                state.ctx.commit_frame();
                present.delete(&mut *state.ctx);
                let copies = state.copies.take().unwrap();
                copies.delete(&mut *state.ctx);
                let scene = state.scene.take().unwrap();
                scene.delete(&mut *state.ctx);
                let targets = std::mem::take(&mut state.targets);
                targets.delete(&mut *state.ctx);
                window::quit();
            }));
            if checked.is_err() {
                std::process::exit(1);
            }
            Box::new(Check)
        },
    );
}
