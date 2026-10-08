use hagane::*;
fn check(s: &Solid) {
    s.validate(Tolerance::default()).unwrap();
    let m = s.tessellate(0.01, Tolerance::default()).unwrap();
    let key = |p: Point3| {
        [
            (p.x * 1e8).round() as i64,
            (p.y * 1e8).round() as i64,
            (p.z * 1e8).round() as i64,
        ]
    };
    let mut uses = std::collections::BTreeMap::new();
    for tri in &m.triangles {
        let p = tri.map(|i| m.positions[i]);
        assert!((p[1] - p[0]).cross(p[2] - p[0]).dot(m.normals[tri[0]]) > 0.);
        for i in 0..3 {
            let a = key(p[i]);
            let b = key(p[(i + 1) % 3]);
            let (k, sign) = if a < b { ((a, b), 1) } else { ((b, a), -1) };
            let u = uses.entry(k).or_insert((0, 0));
            u.0 += 1;
            u.1 += sign;
        }
    }
    assert!(uses.values().all(|&(n, s)| n == 2 && s == 0));
}
fn spec(min: [f64; 3], size: [f64; 3]) -> BoxSpec {
    BoxSpec {
        min: Point3::new(min[0], min[1], min[2]),
        size: Vec3::new(size[0], size[1], size[2]),
    }
}
fn apply(a: BoxSpec, b: BoxSpec, op: BoxBooleanOperation) -> Solid {
    let BoxBooleanResult::Solid(s) = boolean_boxes(a, b, op, GeometryTolerance::default()).unwrap()
    else {
        panic!("expected material")
    };
    check(&s);
    s
}
#[test]
fn identical_coplanar_and_contained_boxes() {
    let a = spec([0.; 3], [4.; 3]);
    for op in [
        BoxBooleanOperation::Union,
        BoxBooleanOperation::Intersection,
    ] {
        assert_eq!(apply(a, a, op).volume().unwrap(), 64.);
    }
    assert!(matches!(
        boolean_boxes(
            a,
            a,
            BoxBooleanOperation::Difference,
            GeometryTolerance::default()
        )
        .unwrap(),
        BoxBooleanResult::Empty
    ));
    let b = spec([2., 0., 0.], [4.; 3]);
    assert_eq!(
        apply(a, b, BoxBooleanOperation::Union).volume().unwrap(),
        96.
    );
    assert_eq!(
        apply(a, b, BoxBooleanOperation::Intersection)
            .volume()
            .unwrap(),
        32.
    );
    assert_eq!(
        apply(a, b, BoxBooleanOperation::Difference)
            .volume()
            .unwrap(),
        32.
    );
    let inner = spec([0.; 3], [2.; 3]);
    assert_eq!(
        apply(a, inner, BoxBooleanOperation::Union)
            .volume()
            .unwrap(),
        64.
    );
    assert_eq!(
        apply(a, inner, BoxBooleanOperation::Intersection)
            .volume()
            .unwrap(),
        8.
    );
    assert_eq!(
        apply(a, inner, BoxBooleanOperation::Difference)
            .volume()
            .unwrap(),
        56.
    );
}
#[test]
fn full_and_partial_face_contacts_in_every_axis() {
    let a = spec([0.; 3], [4.; 3]);
    for axis in 0..3 {
        for sign in [-1., 1.] {
            for partial in [false, true] {
                let size = if partial { [2.; 3] } else { [4.; 3] };
                let mut min = if partial { [1.; 3] } else { [0.; 3] };
                min[axis] = if sign > 0. { 4. } else { -size[axis] };
                let b = spec(min, size);
                for (first, second) in [(a, b), (b, a)] {
                    let s = apply(first, second, BoxBooleanOperation::Union);
                    assert!((s.volume().unwrap() - if partial { 72. } else { 128. }).abs() < 1e-10);
                    let mut point = [2.; 3];
                    point[axis] = if sign > 0. { 4. } else { 0. };
                    assert_eq!(
                        classify_point_in_solid(
                            &s,
                            Point3::new(point[0], point[1], point[2]),
                            GeometryTolerance::default()
                        )
                        .unwrap(),
                        PointLocation::Inside
                    );
                    assert!(matches!(
                        boolean_boxes(
                            first,
                            second,
                            BoxBooleanOperation::Intersection,
                            GeometryTolerance::default()
                        )
                        .unwrap(),
                        BoxBooleanResult::Empty
                    ));
                }
                assert_eq!(
                    apply(a, b, BoxBooleanOperation::Difference)
                        .volume()
                        .unwrap(),
                    64.
                );
            }
        }
    }
}
#[test]
fn rectangular_through_hole_and_partial_overlap() {
    let a = spec([-4., -3., -2.], [8., 6., 4.]);
    let b = spec([-1., -1., -3.], [2., 2., 6.]);
    let s = apply(a, b, BoxBooleanOperation::Difference);
    assert_eq!(s.volume().unwrap(), 176.);
    assert_eq!(
        classify_point_in_solid(&s, Point3::new(0., 0., 0.), GeometryTolerance::default()).unwrap(),
        PointLocation::Outside
    );
    let b = spec([1., -1., -1.], [6.; 3]);
    assert_eq!(
        apply(a, b, BoxBooleanOperation::Union).volume().unwrap(),
        372.
    );
    assert_eq!(
        apply(a, b, BoxBooleanOperation::Intersection)
            .volume()
            .unwrap(),
        36.
    );
    assert_eq!(
        apply(a, b, BoxBooleanOperation::Difference)
            .volume()
            .unwrap(),
        156.
    );
}
#[test]
fn disconnected_nonmanifold_cavity_and_near_contacts_fail() {
    let a = spec([0.; 3], [4.; 3]);
    let t = GeometryTolerance::default();
    for min in [
        [5., 0., 0.],
        [4., 4., 0.],
        [4., 4., 4.],
        [4. + 1e-9, 0., 0.],
        [4. - 1e-9, 0., 0.],
    ] {
        assert!(boolean_boxes(a, spec(min, [4.; 3]), BoxBooleanOperation::Union, t).is_err());
    }
    for min in [[4., 4., 0.], [4., 4., 4.]] {
        assert!(matches!(
            boolean_boxes(a, spec(min, [4.; 3]), BoxBooleanOperation::Intersection, t).unwrap(),
            BoxBooleanResult::Empty
        ));
    }
    let cavity = spec([1.; 3], [2.; 3]);
    assert!(boolean_boxes(a, cavity, BoxBooleanOperation::Difference, t).is_err());
    let slab = spec([1., -1., -1.], [2., 6., 6.]);
    assert!(boolean_boxes(a, slab, BoxBooleanOperation::Difference, t).is_err());
    for bad in [
        spec([f64::NAN, 0., 0.], [4.; 3]),
        spec([0.; 3], [0., 4., 4.]),
        spec([0.; 3], [-1., 4., 4.]),
    ] {
        assert!(boolean_boxes(a, bad, BoxBooleanOperation::Union, t).is_err());
    }
    let far = spec([8.; 3], [4.; 3]);
    assert_eq!(
        apply(a, far, BoxBooleanOperation::Difference)
            .volume()
            .unwrap(),
        64.
    );
    assert!(matches!(
        boolean_boxes(a, far, BoxBooleanOperation::Intersection, t).unwrap(),
        BoxBooleanResult::Empty
    ));
}
#[test]
fn tiny_dimensions_and_translated_contacts() {
    let t = GeometryTolerance::new(1e-14, 1e-10, 0.).unwrap();
    let a = spec([0.; 3], [4e-6; 3]);
    let b = spec([4e-6, 0., 0.], [4e-6; 3]);
    let BoxBooleanResult::Solid(s) = boolean_boxes(a, b, BoxBooleanOperation::Union, t).unwrap()
    else {
        panic!("expected material")
    };
    s.validate(t.absolute()).unwrap();
    assert!((s.volume().unwrap() - 128e-18).abs() < 1e-28);
    let a = spec([100., -200., 300.], [4.; 3]);
    let b = spec([104., -199., 301.], [2.; 3]);
    assert_eq!(
        apply(a, b, BoxBooleanOperation::Union).volume().unwrap(),
        72.
    );
}
