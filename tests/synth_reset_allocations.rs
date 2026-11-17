use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};
struct Count;
thread_local! { static ACTIVE: Cell<bool> = const { Cell::new(false) }; static CALLS: Cell<usize> = const { Cell::new(0) }; }
fn record() {
    ACTIVE.with(|active| {
        if active.get() {
            CALLS.with(|calls| calls.set(calls.get() + 1));
        }
    });
}
unsafe impl GlobalAlloc for Count {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record();
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record();
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        record();
        unsafe { System.realloc(ptr, layout, size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        record();
        unsafe { System.dealloc(ptr, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: Count = Count;
#[test]
fn real_synth_reset_reuses_all_audio_storage() {
    use kkrieger_oxide::{
        kx::Document,
        synth::{engine::Engine, speech::Ronan},
        v2m::{Song, conversion::convert},
    };
    let doc = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    let bytes = convert(doc.song).unwrap();
    let song = Song::parse(&bytes).unwrap();
    let mut engine = Engine::new(song.patches, 44100, 123, Ronan::default()).unwrap();
    engine.set_globals(song.globals).unwrap();
    engine.midi(&[0x90, 60, 100, 0xfd]).unwrap();
    engine.render(&mut [[0.0; 2]; 512]).unwrap();
    for rate in [22050, 44100, 48000, 96000] {
        CALLS.with(|calls| calls.set(0));
        ACTIVE.with(|active| active.set(true));
        let result = engine.reset(rate);
        ACTIVE.with(|active| active.set(false));
        result.unwrap();
        assert_eq!(
            CALLS.with(Cell::get),
            0,
            "heap operation during reset at {rate} Hz"
        );
    }
}

#[test]
fn complete_music_restart_and_loop_render_do_not_touch_heap() {
    use kkrieger_oxide::{
        kx::Document,
        music::{LOOP_FRAMES, Music},
        v2m::{Song, conversion::convert, playback::Synth},
    };
    let doc = Document::parse(include_bytes!("../assets/kkrieger3383.kx")).unwrap();
    let bytes = convert(doc.song).unwrap();
    let song = Song::parse(&bytes).unwrap();
    let synth = Synth::new(&song, 44100, 456).unwrap();
    let mut music = Music::new(&song, synth).unwrap();
    music.render(&mut [[0.0; 2]; 512]).unwrap();
    for _ in 0..3 {
        CALLS.with(|calls| calls.set(0));
        ACTIVE.with(|active| active.set(true));
        let result = music.restart();
        ACTIVE.with(|active| active.set(false));
        result.unwrap();
        assert_eq!(
            CALLS.with(Cell::get),
            0,
            "heap operation during full music restart"
        );
        music.timer = LOOP_FRAMES - 1;
        CALLS.with(|calls| calls.set(0));
        ACTIVE.with(|active| active.set(true));
        let result = music.render(&mut [[0.0; 2]; 64]);
        ACTIVE.with(|active| active.set(false));
        result.unwrap();
        assert_eq!(
            CALLS.with(Cell::get),
            0,
            "heap operation during loop callback"
        );
        CALLS.with(|calls| calls.set(0));
        ACTIVE.with(|active| active.set(true));
        let result = music.player.play(22000);
        ACTIVE.with(|active| active.set(false));
        result.unwrap();
        assert_eq!(
            CALLS.with(Cell::get),
            0,
            "heap operation during authored MIDI seek"
        );
    }
}
