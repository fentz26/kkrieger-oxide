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
