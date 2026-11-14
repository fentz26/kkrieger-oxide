mod hud;
mod math;
mod procedural;
mod render;
mod synth;

fn main() {
    if std::env::args().any(|a| a == "--bench") {
        bench();
        return;
    }
    miniquad::start(
        miniquad::conf::Conf {
            window_title: "kkrieger / oxide — procedural systems lab".into(),
            window_width: 1280,
            window_height: 800,
            sample_count: 4,
            high_dpi: true,
            ..Default::default()
        },
        || Box::new(render::App::new()),
    );
}

fn bench() {
    let start = std::time::Instant::now();
    let room = procedural::Room::generate();
    let mesh_time = start.elapsed();
    let start = std::time::Instant::now();
    let atlas = procedural::texture_atlas();
    let texture_time = start.elapsed();
    let start = std::time::Instant::now();
    let mut synth = synth::Synth::new();
    let mut samples = vec![0_f32; 44100 * 2];
    synth.render(&mut samples);
    let audio_time = start.elapsed();
    let peak = samples.iter().fold(0_f32, |a, &b| a.max(b.abs()));
    let executable = std::env::current_exe()
        .ok()
        .and_then(|p| std::fs::metadata(p).ok())
        .map(|m| m.len())
        .unwrap_or(0);
    println!("kkrieger-oxide / procedural baseline");
    println!(
        "executable: {executable} bytes ({:.1} KiB)",
        executable as f64 / 1024.
    );
    println!(
        "mesh: {} vertices / {} triangles / {} bytes / {mesh_time:?}",
        room.vertices.len(),
        room.indices.len() / 3,
        room.vertices.len() * std::mem::size_of::<procedural::Vertex>() + room.indices.len() * 2
    );
    println!("texture atlas: {} bytes / {texture_time:?}", atlas.len());
    println!("synthesis: 1 second stereo / {audio_time:?} / peak {peak:.3}");
    assert!(peak > 0. && peak <= 1. && samples.iter().all(|s| s.is_finite()));
    assert!(room.can_stand(0., 3.5));
    std::hint::black_box((room, atlas, samples));
}
