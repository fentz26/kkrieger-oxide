use kkrieger_oxide::{
    bitmap::Letter,
    game::Inventory,
    mesh::identity,
    scene::{
        camera::Environment,
        print::{Layout, Values},
    },
};
#[test]
fn formatted_print_geometry_preserves_center_and_each_newline_advance() {
    let inventory = Inventory {
        amounts: [7, 0, 0, 0, 0, 0, 0, 0],
        ..Inventory::default()
    };
    let switches = [0; 256];
    let values = Values {
        inventory: &inventory,
        current_weapon: 0,
        switches: &switches,
        fps: 60.,
    };
    let letters = [Letter {
        uv: [0., 0., 1., 1.],
        width: 1.,
        ..Letter::default()
    }; 256];
    let layout = Layout {
        flags: 0x102,
        size: [1., 2.],
        space: [0., 0.5],
        radius: 0.,
    };
    let geometry = layout.generate(b"%2l;%|A\n\nA", &values, &letters).unwrap();
    assert_eq!(geometry.vertices.len(), 16);
    assert_eq!(&geometry.vertices[0][..3], &[-2., 1., 0.]);
    assert_eq!(&geometry.vertices[8][..3], &[0., 1., 0.]);
    assert_eq!(&geometry.vertices[12][..3], &[-0.5, -4., 0.]);
}
#[test]
fn print_environment_uses_authored_model_or_original_screen_defaults() {
    let camera = Environment {
        zoom_x: 3.,
        fog_color: 0xff123456,
        ..Environment::default()
    };
    let mut model = identity();
    model[3] = [2., 3., 4., 1.];
    let mut layout = Layout::from_words(&[0, 1f32.to_bits(), 2f32.to_bits(), 0, 0, 0]).unwrap();
    let world = layout.environment(&camera, model);
    assert_eq!(world.model, model);
    assert_eq!(world.zoom_x, 3.);
    assert_eq!(world.fog_color, 0xff123456);
    layout.flags = 4;
    let screen = layout.environment(&camera, model);
    assert_eq!(screen.orthogonal, 1);
    assert_eq!(screen.model[3], [-1., 1., 0., 1.]);
    assert_eq!(screen.zoom_x, 1.);
    assert_eq!(screen.space, identity());
    assert!(Layout::from_words(&[0; 5]).is_err());
}

#[test]
fn released_print_ignores_later_bearings_and_advance_metrics() {
    let mut letters = [Letter::default(); 256];
    letters[65] = Letter {
        uv: [0.125, 0.25, 0.5, 0.75],
        pre_space: 10.,
        width: 20.,
        ..Letter::default()
    };
    let layout = Layout {
        flags: 0,
        size: [2., 3.],
        space: [0.5, 0.],
        radius: 0.,
    };
    let inventory = Inventory::default();
    let switches = [0; 256];
    let values = Values {
        inventory: &inventory,
        current_weapon: 0,
        switches: &switches,
        fps: 60.,
    };
    let geometry = layout
        .generate_for_layout(
            b"AA",
            &values,
            &letters,
            kkrieger_oxide::kx::Layout::Beta2004,
        )
        .unwrap();
    assert_eq!(geometry.vertices[0][0], 0.);
    assert_eq!(geometry.vertices[1][0], 1.5);
    assert_eq!(geometry.vertices[4][0], 2.);
    assert_eq!(geometry.vertices[5][0], 3.5);
    assert_eq!(
        layout.width_for_layout(b"AA", None, &letters, kkrieger_oxide::kx::Layout::Beta2004),
        3.5
    );
}

#[test]
fn released_print_widths_and_glyphs_match_native_uv_only_metrics() {
    let fixture = include_bytes!("fixtures/beta-print-metrics.bin");
    assert_eq!(u32::from_le_bytes(fixture[..4].try_into().unwrap()), 64);
    assert_eq!(fixture.len(), 4 + 64 * 200);
    let inventory = Inventory::default();
    let switches = [0; 256];
    let values = Values {
        inventory: &inventory,
        current_weapon: 0,
        switches: &switches,
        fps: 60.,
    };
    for row in fixture[4..].as_chunks::<200>().0 {
        let words: Vec<_> = row
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| u32::from_le_bytes(*b))
            .collect();
        let floats: Vec<_> = words.iter().copied().map(f32::from_bits).collect();
        let layout = Layout {
            flags: 0,
            size: [floats[0], floats[1]],
            space: [floats[2], floats[3]],
            radius: 0.,
        };
        let mut letters = [Letter::default(); 256];
        letters[65] = Letter {
            uv: floats[4..8].try_into().unwrap(),
            pre_space: 123.,
            width: 456.,
            ..Letter::default()
        };
        let width =
            layout.width_for_layout(b"A", None, &letters, kkrieger_oxide::kx::Layout::Beta2004);
        assert_eq!(width.to_bits(), words[8]);
        let geometry = layout
            .generate_for_layout(
                b"AA",
                &values,
                &letters,
                kkrieger_oxide::kx::Layout::Beta2004,
            )
            .unwrap();
        assert_eq!(geometry.vertices.len(), words[9] as usize);
        for (vertex, expected) in geometry.vertices.iter().zip(words[10..].as_chunks::<5>().0) {
            assert_eq!(
                [vertex[0], vertex[1], vertex[2], vertex[6], vertex[7]].map(f32::to_bits),
                *expected
            );
        }
    }
}
