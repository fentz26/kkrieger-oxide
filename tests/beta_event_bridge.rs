use kkrieger_oxide::scene::event::{Event, Scheduler};
#[test]
fn native_actor_events_enter_scene_scheduler_with_preserved_fields() {
    let bytes = include_bytes!("fixtures/beta-ai-multi-tick.bin");
    for record in bytes[4..].as_chunks::<1008>().0 {
        let event_bytes = &record[816..996];
        let words: [u32; 45] = std::array::from_fn(|i| {
            u32::from_le_bytes(event_bytes[i * 4..i * 4 + 4].try_into().unwrap())
        });
        let event = Event::from_released_words(&words, Some(17), None, Some(3));
        assert_eq!(event.scale.map(f32::to_bits), words[13..17]);
        assert_eq!(event.rotate.map(f32::to_bits), words[17..21]);
        assert_eq!(event.translate.map(f32::to_bits), words[21..25]);
        assert_eq!(event.select as u32, words[12]);
        assert_eq!(event.cull_distance, 35);
        assert_eq!(
            event
                .matrix
                .into_iter()
                .flatten()
                .map(f32::to_bits)
                .collect::<Vec<_>>(),
            words[27..43]
        );
        let mut scheduler = Scheduler::default();
        let id = scheduler.insert(event, true);
        let stored = scheduler.events[&id].borrow();
        assert_eq!(stored.operator, Some(17));
        assert_eq!(stored.monster, Some(3));
        assert_eq!(stored.variables(0)[4].map(f32::to_bits), words[13..17]);
    }
}
