mod player;
fn main() {
    let limit = std::env::args()
        .find_map(|arg| arg.strip_prefix("--frames=").map(str::parse::<u64>))
        .transpose()
        .unwrap();
    let start = std::time::Instant::now();
    let prepared = player::Prepared::load().unwrap_or_else(|error| {
        eprintln!("{error}");
        std::process::exit(1)
    });
    if std::env::args().any(|arg| arg == "--bench") {
        prepared.report(start.elapsed());
        return;
    }
    miniquad::start(
        miniquad::conf::Conf {
            window_title: "kkrieger-oxide".into(),
            window_width: 1280,
            window_height: 720,
            ..Default::default()
        },
        move || {
            Box::new(player::App::new(prepared, limit).unwrap_or_else(|error| {
                eprintln!("{error}");
                std::process::exit(1)
            }))
        },
    );
}
