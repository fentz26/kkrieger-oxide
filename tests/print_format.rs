use kkrieger_oxide::{
    game::Inventory,
    scene::print::{Values, format_lines},
};
#[test]
fn original_print_formats_inventory_choices_center_and_newline_counts() {
    let inventory = Inventory {
        amounts: [87, 12, 0, 0, 17, 28, 39, 40],
        ..Inventory::default()
    };
    let mut switches = [0; 256];
    switches[3] = 2;
    let values = Values {
        inventory: &inventory,
        current_weapon: 0,
        switches: &switches,
        fps: 60.25,
    };
    let lines = format_lines(
        b"%3l; %2a; %2w5; %3W; %5f;%|X\n\n%s259zero|one|two\\|bar; %i3=2yes|no; %i3!=2yes|no;",
        &values,
    )
    .unwrap();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].text, b"087 12 28 inf 06025X");
    assert_eq!(lines[0].center, Some(19));
    assert_eq!(lines[0].newlines, 2);
    assert_eq!(lines[1].text, b"two|bar yes no");
    assert_eq!(lines[1].newlines, 0);
}
#[test]
fn original_print_preserves_truncated_tokens_and_negative_decimal_remainders() {
    let inventory = Inventory {
        amounts: [-12, 0, 0, 0, 0, 28, 0, 0],
        ..Inventory::default()
    };
    let switches = [0; 256];
    let values = Values {
        inventory: &inventory,
        current_weapon: 3,
        switches: &switches,
        fps: 0.,
    };
    let lines = format_lines(b"%3l;/%2W;/%3l", &values).unwrap();
    assert_eq!(lines[0].text, b"0/./28/%3l");
    assert!(format_lines(&vec![b'A'; 512], &values).is_err());
    assert_eq!(
        format_lines(&vec![b'A'; 511], &values).unwrap()[0]
            .text
            .len(),
        511
    );
}

#[test]
fn native_original_print_expansion_matches_bytes_centers_and_newlines() {
    let mut bytes: &[u8] = include_bytes!("fixtures/print-format.bin");
    fn word(bytes: &mut &[u8]) -> u32 {
        let n = u32::from_le_bytes(bytes[..4].try_into().unwrap());
        *bytes = &bytes[4..];
        n
    }
    assert_eq!(word(&mut bytes), 512);
    for case in 0..512 {
        let inventory = Inventory {
            amounts: std::array::from_fn(|_| word(&mut bytes) as i32),
            ..Inventory::default()
        };
        let current_weapon = word(&mut bytes) as usize;
        let fps = f32::from_bits(word(&mut bytes));
        let switches = std::array::from_fn(|_| word(&mut bytes) as u8);
        let count = word(&mut bytes) as usize;
        let text: Vec<_> = (0..count).map(|_| word(&mut bytes) as u8).collect();
        let values = Values {
            inventory: &inventory,
            current_weapon,
            switches: &switches,
            fps,
        };
        let lines = format_lines(&text, &values).unwrap();
        assert_eq!(
            lines.len(),
            word(&mut bytes) as usize,
            "case {case} line count"
        );
        for line in lines {
            let len = word(&mut bytes) as usize;
            let expected: Vec<_> = (0..len).map(|_| word(&mut bytes) as u8).collect();
            assert_eq!(line.text, expected, "case {case} text");
            let center = word(&mut bytes) as i32;
            assert_eq!(
                line.center,
                (center >= 0).then_some(center as usize),
                "case {case} center"
            );
            assert_eq!(
                line.newlines,
                word(&mut bytes) as usize,
                "case {case} newlines"
            );
        }
    }
    assert!(bytes.is_empty());
}
