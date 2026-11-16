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
