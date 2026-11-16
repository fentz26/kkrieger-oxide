use kkrieger_oxide::{
    kx::Document,
    scene::{
        exec::Execution,
        overlay::{Pool, Viewport},
    },
};
#[test]
fn published_menu_root_executes_across_menu_states() {
    let d = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    let root = d.roots[1].unwrap();
    for state in [0, 1, 6, 7, 9] {
        let mut e = Execution::new_player(&d);
        e.switches[1] = state;
        e.overlay = Some(Pool::new(
            Viewport {
                screen: 0,
                render_target: -1,
                window: [0, 0, 640, 360],
            },
            std::array::from_fn(|i| 100 + i as i32),
        ));
        e.frame(&d, root)
            .unwrap_or_else(|error| panic!("menu state {state}: {error}"));
        assert!(!e.render_commands.is_empty());
    }
}
