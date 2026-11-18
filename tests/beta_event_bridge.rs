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

#[test]
fn pending_event_publication_preserves_batch_on_resolution_failure() {
    use kkrieger_oxide::scene::event::ReleasedReference;
    let mut first = [0; 45];
    first[0] = 10;
    first[43] = 30;
    first[21] = 1f32.to_bits();
    let mut second = first;
    second[0] = 20;
    second[21] = 2f32.to_bits();
    let original = vec![first, second];
    let mut pending = original.clone();
    let mut scheduler = Scheduler::default();
    let existing = scheduler.insert(Event::default(), false);
    let result = scheduler.publish_released(&mut pending, |_, token| {
        if token == 20 {
            Err("unresolved operator")
        } else {
            Ok(token as usize)
        }
    });
    assert_eq!(result, Err("unresolved operator"));
    assert_eq!(pending, original);
    assert_eq!(scheduler.events.len(), 1);
    assert!(scheduler.events.contains_key(&existing));
    let ids = scheduler
        .publish_released(&mut pending, |kind, token| {
            Ok(match kind {
                ReleasedReference::Operator => token as usize + 100,
                ReleasedReference::Monster => 3,
                ReleasedReference::Spline => unreachable!(),
            })
        })
        .unwrap();
    assert!(pending.is_empty());
    assert_eq!(ids.len(), 2);
    for (index, id) in ids.iter().enumerate() {
        let event = scheduler.events[id].borrow();
        assert_eq!(event.operator, Some(110 + index * 10));
        assert_eq!(event.monster, Some(3));
        assert_eq!(event.translate[0], (index + 1) as f32);
    }
    assert!(
        scheduler
            .publish_released(&mut pending, |_, _| unreachable!())
            .unwrap()
            .is_empty()
    );
    assert_eq!(scheduler.events.len(), 3);
}
