use super::*;

fn view() -> View {
    View::new([LightParam::default(); 256])
}

#[test]
fn clipping_matches_reference_boundaries_and_rejects_external_segments() {
    let mut v = view();
    v.set_viewport(0, 0, 10, 30, 20, 40);
    for (line, expected) in [
        ([-100, 25, 100, 25], Some([10, 25, 30, 25])),
        ([100, 25, -100, 25], Some([30, 25, 10, 25])),
        ([15, -100, 15, 100], Some([15, 20, 15, 40])),
        ([0, 10, 50, 60], Some([10, 20, 30, 40])),
        ([30, 40, 30, 40], Some([30, 40, 30, 40])),
        ([0, 19, 50, 19], None),
        ([31, 0, 31, 100], None),
        ([-50, -50, -20, -20], None),
    ] {
        assert_eq!(clip_wireframe(&v, line), expected, "{line:?}");
    }
}

#[test]
fn in_view_endpoints_and_existing_bresenham_pixels_are_unchanged() {
    let v = view();
    let points = [(0, 0), (495, 0), (0, 383), (495, 383), (170, 83)];
    for (x, y) in points {
        for (xx, yy) in points {
            let line = [x, y, xx, yy];
            assert_eq!(clip_wireframe(&v, line), Some(line));
        }
    }
    let mut pixels = vec![0; SCREEN_W * SCREEN_H];
    let mut stats = Model1RenderStats::default();
    let quad = Quad { points: [Point::default(); 4], z: 0.0,
        color: 0xfe12_3456, moire: false };
    draw_line(&mut pixels, &v, 1, 1, 5, 3, quad, &mut stats);
    let drawn: Vec<_> = pixels.iter().enumerate().filter_map(|(i, &p)|
        (p != 0).then_some((i % SCREEN_W, i / SCREEN_W, p))).collect();
    assert_eq!(drawn, vec![(1, 1, quad.color), (2, 2, quad.color),
        (3, 2, quad.color), (4, 3, quad.color), (5, 3, quad.color)]);
    assert_eq!(stats.pixels, 5);
}

#[test]
fn extreme_wireframes_complete_with_bounded_pixels_in_both_directions() {
    let v = view();
    let quad = Quad { points: [Point::default(); 4], z: 0.0,
        color: 0xfe12_3456, moire: false };
    for (a, b) in [(i32::MIN, i32::MAX), (i32::MAX, i32::MIN)] {
        let mut pixels = vec![0; SCREEN_W * SCREEN_H];
        let mut stats = Model1RenderStats::default();
        draw_line(&mut pixels, &v, a, 100, b, 100, quad, &mut stats);
        assert_eq!(stats.pixels, SCREEN_W);
        assert!(pixels[100 * SCREEN_W..101 * SCREEN_W].iter().all(|&p| p == quad.color));
        assert_eq!(pixels.iter().filter(|&&p| p != 0).count(), SCREEN_W);
    }
    for line in [[i32::MIN, i32::MIN, i32::MAX, i32::MAX],
                 [i32::MAX, i32::MIN, i32::MIN, i32::MAX],
                 [i32::MIN, 0, i32::MIN, i32::MAX]] {
        let clipped = clip_wireframe(&v, line);
        if let Some([x0, y0, x1, y1]) = clipped {
            assert!((x1 - x0).abs() < SCREEN_W as i32);
            assert!((y1 - y0).abs() < SCREEN_H as i32);
        }
        let mut pixels = vec![0; SCREEN_W * SCREEN_H];
        let mut stats = Model1RenderStats::default();
        draw_line(&mut pixels, &v, line[0], line[1], line[2], line[3], quad, &mut stats);
        assert!(stats.pixels <= SCREEN_W.max(SCREEN_H));
    }
}

#[test]
fn clipping_intersects_the_actual_framebuffer_and_rejects_empty_viewports() {
    let mut v = view();
    // Exercise the rasterizer boundary directly, independently of projection.
    v.px1 = i32::MIN;
    v.px2 = i32::MAX;
    v.y1 = i32::MIN;
    v.y2 = i32::MAX;
    assert_eq!(clip_wireframe(&v, [i32::MIN, 0, i32::MAX, 0]), Some([0, 0, 495, 0]));
    v.set_viewport(0, 0, 30, 10, 20, 40);
    assert_eq!(clip_wireframe(&v, [0, 0, 100, 100]), None);
    v.set_viewport(0, 0, 600, 700, 0, 100);
    assert_eq!(clip_wireframe(&v, [0, 0, 1000, 100]), None);
}
