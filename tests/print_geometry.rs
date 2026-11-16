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
