use kkrieger_oxide::{
    bitmap::Letter,
    scene::print::{Geometry, Layout},
};
#[test]
fn original_print_uses_metric_advance_prespace_and_triangle_order() {
    let mut letters = [Letter::default(); 256];
    letters[b'A' as usize] = Letter {
        uv: [0.25, 0.5, 0.5, 1.],
        pre_space: 0.125,
        width: 0.375,
        ..Letter::default()
    };
    let layout = Layout {
        flags: 0,
        size: [2., 3.],
        space: [0.5, 1.],
        radius: 0.,
    };
    let mut geometry = Geometry::default();
    layout.append_line(b"AA", None, 4., &letters, &mut geometry);
    assert_eq!(geometry.vertices.len(), 8);
    assert_eq!(geometry.indices, [0, 1, 3, 0, 3, 2, 4, 5, 7, 4, 7, 6]);
    assert_eq!(&geometry.vertices[0][..3], &[0.5, 4., 0.]);
    assert_eq!(&geometry.vertices[1][..3], &[1.5, 4., 0.]);
    assert_eq!(&geometry.vertices[2][..3], &[0.5, 1., 0.]);
    assert_eq!(&geometry.vertices[4][..3], &[2.5, 4., 0.]);
    assert_eq!(geometry.vertices[0][5].to_bits(), 0xffffffff);
    assert_eq!(&geometry.vertices[3][6..], &[0.5, 1.]);
    assert_eq!(layout.width(b"AA", None, &letters), 3.5);
    assert_eq!(layout.width(b"", None, &letters), -0.5);
}
#[test]
fn original_print_skips_empty_glyphs_and_honors_quad_capacity() {
    let mut letters = [Letter::default(); 256];
    letters[65] = Letter {
        uv: [0., 0., 1., 1.],
        width: 1.,
        ..Letter::default()
    };
    let layout = Layout {
        flags: 0x100,
        size: [1.; 2],
        space: [0.; 2],
        radius: 0.,
    };
    let mut geometry = Geometry::default();
    layout.append_line(b"?AA", Some(1), 0., &letters, &mut geometry);
    assert_eq!(geometry.vertices.len(), 8);
    assert_eq!(geometry.vertices[0][0], 0.);
    layout.append_line(&vec![65; 5000], None, 0., &letters, &mut geometry);
    assert_eq!(geometry.vertices.len(), 4096 * 4);
    assert_eq!(geometry.indices.len(), 4096 * 6);
}

#[test]
fn native_original_print_layout_matches_vertex_and_index_words() {
    let mut bytes: &[u8] = include_bytes!("fixtures/print-geometry.bin");
    fn word(bytes: &mut &[u8]) -> u32 {
        let n = u32::from_le_bytes(bytes[..4].try_into().unwrap());
        *bytes = &bytes[4..];
        n
    }
    fn real(bytes: &mut &[u8]) -> f32 {
        f32::from_bits(word(bytes))
    }
    assert_eq!(word(&mut bytes), 256);
    for case in 0..256 {
        let flags = word(&mut bytes);
        let center = word(&mut bytes) as i32;
        let layout = Layout {
            flags,
            size: std::array::from_fn(|_| real(&mut bytes)),
            space: std::array::from_fn(|_| real(&mut bytes)),
            radius: real(&mut bytes),
        };
        let y = real(&mut bytes);
        let mut letters = [Letter::default(); 256];
        for code in [65, 66, 128, 32] {
            letters[code] = Letter {
                uv: std::array::from_fn(|_| real(&mut bytes)),
                pre_space: real(&mut bytes),
                width: real(&mut bytes),
                ..Letter::default()
            };
        }
        let len = word(&mut bytes) as usize;
        let text: Vec<_> = (0..len).map(|_| word(&mut bytes) as u8).collect();
        let mut geometry = Geometry::default();
        layout.append_line(
            &text,
            (center >= 0).then_some(center as usize),
            y,
            &letters,
            &mut geometry,
        );
        assert_eq!(
            layout.width(&text, None, &letters).to_bits(),
            word(&mut bytes),
            "case {case} width"
        );
        assert_eq!(geometry.vertices.len(), word(&mut bytes) as usize);
        assert_eq!(geometry.indices.len(), word(&mut bytes) as usize);
        for (i, v) in geometry.vertices.iter().flatten().enumerate() {
            assert_eq!(v.to_bits(), word(&mut bytes), "case {case} vertex word {i}");
        }
        for i in geometry.indices {
            assert_eq!(u32::from(i), word(&mut bytes), "case {case} index");
        }
    }
    assert!(bytes.is_empty());
}

#[test]
fn released_print_has_five_words_and_planar_geometry() {
    let words = [
        4,
        2f32.to_bits(),
        3f32.to_bits(),
        0.5f32.to_bits(),
        1f32.to_bits(),
    ];
    let layout =
        Layout::from_words_for_layout(&words, kkrieger_oxide::kx::Layout::Beta2004).unwrap();
    assert_eq!(layout.flags, 4);
    assert_eq!(layout.size, [2., 3.]);
    assert_eq!(layout.space, [0.5, 1.]);
    assert_eq!(layout.radius.to_bits(), 0);
    assert!(Layout::from_words(&words).is_err());
    assert!(
        Layout::from_words_for_layout(&words[..4], kkrieger_oxide::kx::Layout::Beta2004).is_err()
    );
}

#[test]
fn released_print_parameter_layout_matches_native_captures() {
    let fixture = include_bytes!("fixtures/beta-print-layout.bin");
    assert_eq!(u32::from_le_bytes(fixture[..4].try_into().unwrap()), 64);
    assert_eq!(fixture.len(), 4 + 64 * 20);
    for row in fixture[4..].as_chunks::<20>().0 {
        let words: Vec<_> = row
            .as_chunks::<4>()
            .0
            .iter()
            .map(|word| u32::from_le_bytes(*word))
            .collect();
        let layout =
            Layout::from_words_for_layout(&words, kkrieger_oxide::kx::Layout::Beta2004).unwrap();
        assert_eq!(layout.flags, words[0]);
        assert_eq!(layout.size.map(f32::to_bits), [words[1], words[2]]);
        assert_eq!(layout.space.map(f32::to_bits), [words[3], words[4]]);
        assert_eq!(layout.radius.to_bits(), 0);
    }
}
