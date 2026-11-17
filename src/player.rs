//! Native window adapter around the authored player coordinator.
//! Authored simulation, rendering and native music output.
use kkrieger_oxide::{
    collision::world::World,
    game::{input, simulation::Simulation},
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
use std::{cell::RefCell, rc::Rc, time::Instant};
pub struct Prepared {
    doc: Document<'static>,
    root: usize,
    execution: Execution,
    simulation: Simulation,
    resources: gpu_scene::Resources,
}
impl Prepared {
    pub fn report(&self, elapsed: std::time::Duration) {
        let vertices: usize = self
            .resources
            .paint
            .meshes
            .values()
            .map(|asset| asset.mesh.vertices.len())
            .sum();
        let mut unique = std::collections::BTreeSet::new();
        let texture_bytes: usize = self
            .resources
            .bitmaps
            .values()
            .filter(|bitmap| unique.insert(Rc::as_ptr(bitmap) as usize))
            .map(|bitmap| {
                bitmap.pixels.len() * std::mem::size_of::<kkrieger_oxide::bitmap::Pixel>()
            })
            .sum();
        let executable = std::env::current_exe()
            .ok()
            .and_then(|path| std::fs::metadata(path).ok())
            .map_or(0, |metadata| metadata.len());
        println!("kkrieger-oxide authored player assets");
        println!("executable: {executable} bytes");
        println!("precalculation: {:.3} ms", elapsed.as_secs_f64() * 1000.);
        println!(
            "meshes: {} | vertices: {vertices} | materials: {}",
            self.resources.paint.meshes.len(),
            self.resources.paint.materials.len()
        );
        println!(
            "bitmaps: {} | host pixel bytes: {texture_bytes}",
            self.resources.bitmaps.len()
        );
    }
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let doc = Document::parse(include_bytes!("../assets/kkrieger3383.kx"))?;
        let root = doc.roots[0].ok_or("authored intro root missing")?;
        let resources = gpu_scene::Resources::prepare(&doc, root, true, 0)?;
        let mut execution = Execution::new_player(&doc);
        execution.prepare_game_operators(&doc, root)?;
        let position = execution.game.parameters.start_position;
        let scene = kkrieger_oxide::game::root::painter_scene(&doc, root)?
            .ok_or("intro root has no painter scene")?;
        let world = World::from_document(&doc, scene)?;
        let mut simulation =
            Simulation::new(world, &mut execution, position, 0, [0, 0], Random::new(1))?;
        simulation.restart(&mut execution, [0, 0])?;
        execution.switches[1] = 8;
        Ok(Self {
            doc,
            root,
            execution,
            simulation,
            resources,
        })
    }
}
struct Graphics {
    ctx: Box<dyn RenderingBackend>,
    scene: gpu_scene::Scene,
    targets: gpu_target::Targets,
    present: gpu_present::Present,
    copies: gpu_overlay::Overlays,
    fps: f32,
}
struct Bridge(Rc<RefCell<Graphics>>);
impl Renderer for Bridge {
    fn render(
        &mut self,
        e: &mut Execution,
        doc: &Document<'_>,
        command: RenderCommand,
    ) -> Result<(), Error> {
        let fail = |reason| Error {
            operator: 0,
            reason,
        };
        if let RenderCommand::Overlay(index) = command {
            let draw = e.overlay_draws[index].clone();
            let pool = e
                .overlay
                .as_ref()
                .ok_or_else(|| fail("overlay target pool missing"))?;
            let (width, height) = window::screen_size();
            let dimensions = gpu_target::attachment_dimensions(
                pool,
                draw.viewport,
                [width as u32, height as u32],
            )
            .map_err(fail)?;
            let mut graphics = self.0.borrow_mut();
            let Graphics {
                ctx,
                targets,
                copies,
                ..
            } = &mut *graphics;
            let source = targets
                .texture(-1, draw.texture)
                .ok_or_else(|| fail("overlay source target has not been rendered"))?;
            let mut sources = vec![source];
            if let Some(texture) = draw.texture1 {
                sources.push(
                    targets
                        .texture(-1, texture)
                        .ok_or_else(|| fail("second overlay target has not been rendered"))?,
                );
            }

            if let Some(texture) = draw.texture2 {
                sources.push(
                    targets
                        .texture(-1, texture)
                        .ok_or_else(|| fail("third overlay target has not been rendered"))?,
                );
            }

            targets
                .begin(&mut **ctx, draw.viewport, dimensions, 0, 0)
                .map_err(fail)?;
            let result = copies.draw_sources(&mut **ctx, &draw, &sources);
            ctx.end_render_pass();
            return result.map_err(|error| {
                eprintln!("{error}");
                fail("original GPU overlay failed")
            });
        }
        let RenderCommand::Scene(index) = command else {
            return Ok(());
        };
        let pass = e.render_passes[index].clone();
        let fail = |reason| Error {
            operator: 0,
            reason,
        };
        let [x0, y0, x1, y1] = pass.viewport.window;
        let context =
            light::Context::from_camera(&pass.camera, [x1 - x0, y1 - y0], true).map_err(fail)?;
        let candidates = pass
            .jobs
            .iter()
            .filter(|job| matches!(job, Job::Light { .. }))
            .map(|job| context.prepare(job))
            .collect::<Result<Vec<_>, _>>()
            .map_err(fail)?
            .into_iter()
            .flatten()
            .collect();
        // Released ordering/clamp; geometric preparation remains the explicitly
        // ported published-source profile until beta geometry ABI is complete.
        let lights = light::beta_select(candidates);
        let mut graphics = self.0.borrow_mut();
        let Graphics {
            ctx,
            scene,
            targets,
            fps,
            ..
        } = &mut *graphics;
        let ctx = &mut **ctx;
        let pool = e
            .overlay
            .as_ref()
            .ok_or_else(|| fail("screen target pool missing"))?;
        let (width, height) = window::screen_size();
        let dimensions =
            gpu_target::attachment_dimensions(pool, pass.viewport, [width as u32, height as u32])
                .map_err(fail)?;
        targets
            .begin(
                ctx,
                pass.viewport,
                dimensions,
                pass.clear_flags,
                pass.clear_color,
            )
            .map_err(fail)?;
        let result = scene.draw(
            ctx,
            e,
            doc,
            &pass,
            &lights,
            &gpu_scene::Options {
                fps: *fps,
                ..Default::default()
            },
        );
        ctx.end_render_pass();
        result.map_err(|error| {
            eprintln!("{error}");
            fail("authored GPU scene failed")
        })?;
        Ok(())
    }
}
pub struct App {
    #[cfg(target_os = "macos")]
    music: kkrieger_oxide::music::output::Output,
    doc: Document<'static>,
    root: usize,
    root_slot: usize,
    execution: Execution,
    simulation: Simulation,
    graphics: Rc<RefCell<Graphics>>,
    start: Instant,
    clock: kkrieger_oxide::game::frame::Clock,
    mouse: [f64; 2],
    grabbed: bool,
    frames: u64,
    limit: Option<u64>,
}
impl App {
    pub fn new(prepared: Prepared, limit: Option<u64>) -> Result<Self, Box<dyn std::error::Error>> {
        let Prepared {
            doc,
            root,
            mut execution,
            simulation,
            resources,
        } = prepared;
        let mut ctx = window::new_rendering_backend();
        let scene = gpu_scene::Scene::new(&mut *ctx, resources, 24)?;
        let present = gpu_present::Present::new(&mut *ctx)?;
        let copies = gpu_overlay::Overlays::new(&mut *ctx, 24)?;
        let graphics = Rc::new(RefCell::new(Graphics {
            ctx,
            scene,
            targets: Default::default(),
            present,
            copies,
            fps: 0.,
        }));
        execution.renderer = Some(Box::new(Bridge(Rc::clone(&graphics))));
        let (width, height) = window::screen_size();
        execution.overlay = Some(Pool::new(
            Viewport {
                screen: 0,
                render_target: -1,
                window: [0, 0, width as i32, height as i32],
            },
            std::array::from_fn(|i| 100 + i as i32),
        ));
        #[cfg(target_os = "macos")]
        let music =
            kkrieger_oxide::music::output::Output::open_with_effects(doc.song, doc.samples)?;
        Ok(Self {
            #[cfg(target_os = "macos")]
            music,
            doc,
            root,
            root_slot: 0,
            execution,
            simulation,
            graphics,
            start: Instant::now(),
            clock: kkrieger_oxide::game::frame::Clock::new(0),
            mouse: [0.; 2],
            grabbed: false,
            frames: 0,
            limit,
        })
    }
    fn frame(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        #[cfg(target_os = "macos")]
        self.music.check()?;
        let frame = self
            .clock
            .advance(self.start.elapsed().as_millis() as i32, self.doc.bpm);
        let mouse = self.mouse.map(|v| v as i32);
        let tick = self.simulation.tick(
            &mut self.execution,
            &self.doc,
            frame.slices,
            frame.time,
            mouse,
        )?;
        #[cfg(target_os = "macos")]
        self.music.tick(tick, frame.time)?;
        #[cfg(not(target_os = "macos"))]
        let _ = tick;
        let (width, height) = window::screen_size();
        if width <= 0. || height <= 0. {
            return Ok(());
        }
        let master = Viewport {
            screen: 0,
            render_target: -1,
            window: kkrieger_oxide::game::frame::master_window(width as i32, height as i32),
        };
        self.execution.overlay.as_mut().unwrap().set_master(master);
        let selection = self.simulation.select_root(&mut self.execution, mouse)?;
        if selection.quit {
            window::quit();
            return Ok(());
        }
        let root = self.doc.roots[selection.index].ok_or("selected player root missing")?;
        if selection.index != self.root_slot {
            #[cfg(target_os = "macos")]
            self.music.pause_music()?;
            self.execution.reset_view();
            self.execution.init_frame(0, 0, false);
            let resources = gpu_scene::Resources::prepare(&self.doc, root, true, 0)?;
            self.execution.prepare_game_operators(&self.doc, root)?;
            self.execution.finish_events();
            if let Some(scene) = kkrieger_oxide::game::root::painter_scene(&self.doc, root)? {
                let world = World::from_document(&self.doc, scene)?;
                self.simulation
                    .reset_root(world, &mut self.execution, mouse, false)?;
            } else {
                self.simulation.restart(&mut self.execution, mouse)?;
            }
            let mut graphics = self.graphics.borrow_mut();
            let Graphics { ctx, scene, .. } = &mut *graphics;
            let replacement = gpu_scene::Scene::new(&mut **ctx, resources, 24)?;
            ctx.commit_frame();
            std::mem::replace(scene, replacement).delete(&mut **ctx);
            self.root = root;
            self.root_slot = selection.index;
            #[cfg(target_os = "macos")]
            self.music.restart_music()?;
        }
        self.execution.aspect = 2.;
        self.graphics.borrow_mut().fps = frame.fps;
        self.simulation
            .prepare_scene(&mut self.execution, frame.beat, frame.time, 1., false)?;
        {
            let mut graphics = self.graphics.borrow_mut();
            let Graphics { ctx, targets, .. } = &mut *graphics;
            targets.begin(
                &mut **ctx,
                Viewport {
                    window: [0, 0, width as i32, height as i32],
                    ..master
                },
                [width as u32, height as u32],
                3,
                0,
            )?;
            ctx.end_render_pass();
        }
        self.simulation
            .execute_scene(&mut self.execution, &self.doc, self.root)?;
        self.execution.finish_events();
        let mut graphics = self.graphics.borrow_mut();
        let Graphics {
            ctx,
            targets,
            present,
            ..
        } = &mut *graphics;
        let source = targets
            .texture(0, -1)
            .ok_or("authored frame did not produce screen target")?;
        ctx.begin_default_pass(PassAction::Nothing);
        present.draw(&mut **ctx, source);
        ctx.end_render_pass();
        ctx.commit_frame();
        self.frames += 1;
        if self.limit.is_some_and(|limit| self.frames >= limit) {
            window::quit();
        }
        Ok(())
    }
    fn key(&mut self, key: u32) {
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            let result =
                self.simulation
                    .key(&mut self.execution, key, self.mouse.map(|v| v as i32))?;
            #[cfg(target_os = "macos")]
            for sound in result.sounds {
                self.music
                    .play(sound, self.simulation.motion.collider.position)?;
            }
            #[cfg(not(target_os = "macos"))]
            let _ = result;
            Ok(())
        })();
        if let Err(error) = result {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
    fn capture(&mut self, capture: bool) {
        self.grabbed = capture;
        window::set_cursor_grab(capture);
        window::show_mouse(!capture);
    }
}
fn key_code(key: KeyCode) -> Option<u32> {
    match key {
        key if (KeyCode::A as u32..=KeyCode::Z as u32).contains(&(key as u32)) => {
            Some(key as u32 + 32)
        }
        key if (KeyCode::Key0 as u32..=KeyCode::Key9 as u32).contains(&(key as u32)) => {
            Some(key as u32)
        }
        KeyCode::Space => Some(32),
        KeyCode::Enter => Some(input::ENTER),
        KeyCode::Backspace => Some(input::BACKSPACE),
        KeyCode::Escape => Some(input::ESCAPE),
        KeyCode::Up => Some(input::UP),
        KeyCode::Down => Some(input::DOWN),
        KeyCode::Left => Some(input::LEFT),
        KeyCode::Right => Some(input::RIGHT),
        KeyCode::LeftShift => Some(input::SHIFT_LEFT),
        KeyCode::RightControl => Some(input::CONTROL_RIGHT),
        _ => None,
    }
}
impl EventHandler for App {
    fn update(&mut self) {}
    fn draw(&mut self) {
        if let Err(error) = self.frame() {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
    fn key_down_event(&mut self, key: KeyCode, _: KeyMods, repeat: bool) {
        if key == KeyCode::Escape {
            self.capture(false);
        }
        if !repeat && let Some(key) = key_code(key) {
            self.key(key);
        }
    }
    fn key_up_event(&mut self, key: KeyCode, _: KeyMods) {
        if let Some(key) = key_code(key) {
            self.key(key | input::BREAK);
        }
    }
    fn raw_mouse_motion(&mut self, dx: f32, dy: f32) {
        if self.grabbed {
            self.mouse[0] += f64::from(dx);
            self.mouse[1] += f64::from(dy);
        }
    }
    fn mouse_button_down_event(&mut self, button: MouseButton, _: f32, _: f32) {
        if button == MouseButton::Left {
            self.capture(true);
            self.key(input::MOUSE_LEFT);
        }
    }
    fn mouse_button_up_event(&mut self, button: MouseButton, _: f32, _: f32) {
        if button == MouseButton::Left {
            self.key(input::MOUSE_LEFT | input::BREAK);
        }
    }
    fn window_minimized_event(&mut self) {
        self.capture(false);
        for key in [119, 97, 115, 100, input::MOUSE_LEFT] {
            self.key(key | input::BREAK);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_keys_preserve_original_character_and_special_key_codes() {
        assert_eq!(key_code(KeyCode::W), Some(119));
        assert_eq!(key_code(KeyCode::Key2), Some(50));
        assert_eq!(key_code(KeyCode::LeftShift), Some(input::SHIFT_LEFT));
        assert_eq!(key_code(KeyCode::RightControl), Some(input::CONTROL_RIGHT));
        assert_eq!(key_code(KeyCode::Up), Some(input::UP));
        assert_eq!(key_code(KeyCode::Escape), Some(input::ESCAPE));
        assert_eq!(
            key_code(KeyCode::W).unwrap() | input::BREAK,
            119 | input::BREAK
        );
        assert_eq!(key_code(KeyCode::Unknown), None);
    }
}
