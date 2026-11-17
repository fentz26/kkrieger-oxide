use kkrieger_oxide::game::{options::Settings, root};
#[test]
fn original_resolution_choices_and_shadow_usage_mask() {
    let mut switches = root::switches();
    assert_eq!(Settings::read(&switches).unwrap().resolution, [800, 600]);
    for (index, expected) in [[640, 480], [800, 600], [1024, 768], [1280, 1024]]
        .into_iter()
        .enumerate()
    {
        switches[8] = index as u8;
        for shadows in [0, 1, 255] {
            switches[10] = shadows;
            let settings = Settings::read(&switches).unwrap();
            assert_eq!(settings.resolution, expected);
            assert_eq!(settings.usage_mask & (1 << 3) != 0, shadows != 0);
            assert_eq!(settings.usage_mask | (1 << 3), u32::MAX);
        }
    }
    switches[8] = 4;
    assert!(Settings::read(&switches).is_err());
}
