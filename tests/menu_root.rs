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

#[test]
fn published_intro_events_render_and_select_main_menu() {
    use kkrieger_oxide::game::root;
    let d = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    let mut e = Execution::new_player(&d);
    e.switches[1] = 8;
    e.overlay = Some(Pool::new(
        Viewport {
            screen: 0,
            render_target: -1,
            window: [0, 120, 1280, 600],
        },
        std::array::from_fn(|i| 100 + i as i32),
    ));
    let intro = d.roots[0].unwrap();
    e.prepare_game_operators(&d, intro).unwrap();
    for time in [0, 1000, 5000, 10000, 20000] {
        e.init_frame(
            ((i64::from(time) * i64::from(d.bpm)) / 60000) as i32,
            time,
            false,
        );
        e.add_document_events();
        e.aspect = 2.;
        e.frame(&d, intro).unwrap();
        assert!(!e.render_passes.is_empty());
        if time < 20000 {
            assert_eq!(e.switches[1], 8);
            assert!(!e.overlay_draws.is_empty());
        }
        e.finish_events();
    }
    assert_eq!(e.switches[1], 6);
    assert_eq!(root::select(&mut e.switches).index, 1);
    e.reset_view();
    e.init_frame(0, 0, false);
    e.prepare_game_operators(&d, d.roots[1].unwrap()).unwrap();
    e.finish_events();
    e.init_frame(0, 20001, false);
    e.add_document_events();
    e.frame(&d, d.roots[1].unwrap()).unwrap();
    assert_eq!(e.render_passes.len(), 2);
    assert_eq!(e.overlay_draws.len(), 9);
}
