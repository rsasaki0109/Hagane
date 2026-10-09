use hagane::{Curve, NurbsHoledFace, NurbsSurface, Point3, Surface, Tolerance};
fn source() -> NurbsSurface {
    let k = vec![0., 0., 0., 1., 1., 1.];
    let mut points = Vec::new();
    let mut weights = Vec::new();
    for i in 0..3 {
        for j in 0..3 {
            points.push(Point3::new(
                i as f64,
                j as f64,
                if i == 1 && j == 1 { 0.7 } else { 0. },
            ));
            weights.push(if i == 1 && j == 1 { 1.3 } else { 1. });
        }
    }
    NurbsSurface::new([2, 2], [k.clone(), k], [3, 3], points, weights).unwrap()
}
#[test]
fn exact_inner_wires_close_clockwise_and_surface_identity_is_retained() {
    let original = source();
    let tol = Tolerance::default();
    let outer = [[0.1, 0.9], [0.1, 0.9]];
    let holes = vec![[[0.2, 0.35], [0.3, 0.7]], [[0.55, 0.8], [0.4, 0.6]]];
    let face = NurbsHoledFace::new(original.clone(), outer, holes.clone(), 1, tol).unwrap();
    face.validate_boundary(tol).unwrap();
    assert_eq!(face.outer(), outer);
    assert_eq!(face.holes(), holes);
    assert_eq!(face.vertices.len(), 12);
    assert_eq!(face.edges.len(), 12);
    assert_eq!(face.face.wires.len(), 3);
    let Surface::Nurbs(surface) = &face.face.surface else {
        panic!()
    };
    for (wire_index, wire) in face.face.wires.iter().enumerate() {
        let mut previous = None;
        let mut first = None;
        let mut uv_corners = Vec::new();
        for coedge in &wire.coedges {
            let edge = &face.edges[coedge.edge];
            let [a, b] = edge.curve.range();
            let start = if coedge.forward {
                edge.vertices[0]
            } else {
                edge.vertices[1]
            };
            let end = if coedge.forward {
                edge.vertices[1]
            } else {
                edge.vertices[0]
            };
            if let Some(previous) = previous {
                assert_eq!(previous, start);
            } else {
                first = Some(start);
            }
            previous = Some(end);
            uv_corners.push(coedge.pcurve.evaluate(if coedge.forward { a } else { b }));
            for i in 0..=12 {
                let parameter = if i == 12 {
                    b
                } else {
                    a + (b - a) * i as f64 / 12.
                };
                let uv = coedge.pcurve.evaluate(parameter);
                let actual = edge.curve.try_evaluate(parameter).unwrap();
                assert!((actual - surface.evaluate(uv[0], uv[1]).unwrap()).norm() < 1e-12);
                assert!((actual - original.evaluate(uv[0], uv[1]).unwrap()).norm() < 1e-12);
            }
        }
        assert_eq!(previous, first);
        let signed_area = (0..4)
            .map(|i| {
                let a = uv_corners[i];
                let b = uv_corners[(i + 1) % 4];
                a[0] * b[1] - a[1] * b[0]
            })
            .sum::<f64>();
        assert!(if wire_index == 0 {
            signed_area > 0.
        } else {
            signed_area < 0.
        });
    }
}
#[test]
fn hole_aligned_bounded_mesh_has_no_interior_cells_and_compact_metadata() {
    let holes = vec![[[0.35, 0.65], [0.3, 0.7]]];
    let face = NurbsHoledFace::new(
        source(),
        [[0.1, 0.9], [0.1, 0.9]],
        holes.clone(),
        1,
        Tolerance::default(),
    )
    .unwrap();
    let bounded = face
        .tessellate_bounded(0.02, 4096, Tolerance::default())
        .unwrap();
    assert_eq!(bounded.mesh.triangles.len(), 2 * bounded.uv_ranges.len());
    assert_eq!(bounded.vertex_uv.len(), bounded.mesh.positions.len());
    assert_eq!(bounded.vertex_nodes.len(), bounded.mesh.positions.len());
    assert_eq!(bounded.normal_sides.len(), bounded.mesh.positions.len());
    assert!(bounded.error_bounds.iter().all(|b| *b <= 0.02));
    let mut used = std::collections::BTreeSet::new();
    let mut edges = std::collections::BTreeMap::new();
    for triangle in &bounded.mesh.triangles {
        used.extend(triangle.iter().copied());
        for (a, b) in [
            (triangle[0], triangle[1]),
            (triangle[1], triangle[2]),
            (triangle[2], triangle[0]),
        ] {
            let a = bounded.vertex_nodes[a];
            let b = bounded.vertex_nodes[b];
            let key = if a < b { (a, b) } else { (b, a) };
            *edges.entry(key).or_insert(0) += 1;
        }
    }
    assert_eq!(used.len(), bounded.mesh.positions.len());
    assert!(edges.values().all(|n| *n == 1 || *n == 2));
    let nodes = bounded
        .vertex_nodes
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(nodes.len(), *nodes.last().unwrap() + 1);
    let hole = holes[0];
    let area = bounded
        .uv_ranges
        .iter()
        .map(|r| {
            assert!(
                !(r[0][0].max(hole[0][0]) < r[0][1].min(hole[0][1])
                    && r[1][0].max(hole[1][0]) < r[1][1].min(hole[1][1]))
            );
            (r[0][1] - r[0][0]) * (r[1][1] - r[1][0])
        })
        .sum::<f64>();
    assert!((area - (0.8 * 0.8 - 0.3 * 0.4)).abs() < 1e-12);
    for ((a, b), count) in edges {
        if count == 1 {
            let ia = bounded
                .vertex_nodes
                .iter()
                .position(|node| *node == a)
                .unwrap();
            let ib = bounded
                .vertex_nodes
                .iter()
                .position(|node| *node == b)
                .unwrap();
            let midpoint = [
                (bounded.vertex_uv[ia][0] + bounded.vertex_uv[ib][0]) * 0.5,
                (bounded.vertex_uv[ia][1] + bounded.vertex_uv[ib][1]) * 0.5,
            ];
            assert!(
                [0.1, 0.9].contains(&midpoint[0])
                    || [0.1, 0.9].contains(&midpoint[1])
                    || ((hole[0].contains(&midpoint[0])
                        && midpoint[1] >= hole[1][0]
                        && midpoint[1] <= hole[1][1])
                        || (hole[1].contains(&midpoint[1])
                            && midpoint[0] >= hole[0][0]
                            && midpoint[0] <= hole[0][1]))
            );
        }
    }
}
#[test]
fn negative_orientation_and_dirty_topology_are_checked() {
    let tol = Tolerance::default();
    let holes = vec![[[0.35, 0.65], [0.3, 0.7]]];
    let positive =
        NurbsHoledFace::new(source(), [[0., 1.], [0., 1.]], holes.clone(), 1, tol).unwrap();
    let negative = NurbsHoledFace::new(source(), [[0., 1.], [0., 1.]], holes, -1, tol).unwrap();
    let a = positive.tessellate_bounded(0.05, 4096, tol).unwrap();
    let b = negative.tessellate_bounded(0.05, 4096, tol).unwrap();
    assert_eq!(a.mesh.positions, b.mesh.positions);
    assert_eq!(a.vertex_nodes, b.vertex_nodes);
    for (a, b) in a.mesh.normals.iter().zip(&b.mesh.normals) {
        assert_eq!(*a, *b * -1.);
    }
    for (a, b) in a.mesh.triangles.iter().zip(&b.mesh.triangles) {
        assert_eq!(*b, [a[0], a[2], a[1]]);
    }
    let mut dirty = positive.clone();
    dirty.face.wires[1].coedges[0].forward = !dirty.face.wires[1].coedges[0].forward;
    assert!(dirty.validate_boundary(tol).is_err());
    assert!(dirty.tessellate_bounded(0.1, 4096, tol).is_err());
    let mut dirty = positive;
    dirty.edges[4].curve = Curve::Line {
        a: Point3::new(0., 0., 0.),
        b: Point3::new(1., 0., 0.),
    };
    assert!(dirty.validate_boundary(tol).is_err());
}
#[test]
fn rejects_uv_contact_overlap_unresolved_gaps_and_limits() {
    let tol = Tolerance::default();
    for holes in [
        vec![[[0., 0.3], [0.2, 0.4]]],
        vec![[[0.2, 0.4], [0.2, 0.4]], [[0.4, 0.6], [0.3, 0.5]]],
        vec![[[0.2, 0.5], [0.2, 0.5]], [[0.3, 0.6], [0.3, 0.6]]],
        vec![[[1e-15, 0.3], [0.2, 0.4]]],
        vec![[[0.2, 0.4], [0.2, 0.4]], [[0.4 + 1e-15, 0.6], [0.3, 0.5]]],
        vec![[[0.2, f64::NAN], [0.3, 0.6]]],
        vec![[[0.5, 0.5], [0.3, 0.6]]],
    ] {
        assert!(NurbsHoledFace::new(source(), [[0., 1.], [0., 1.]], holes, 1, tol).is_err());
    }
    assert!(NurbsHoledFace::new(
        source(),
        [[0., 1.], [0., 1.]],
        vec![[[0.2, 0.3], [0.2, 0.3]]; 17],
        1,
        tol
    )
    .is_err());
    let valid = NurbsHoledFace::new(
        source(),
        [[0., 1.], [0., 1.]],
        vec![[[0.2, 0.4], [0.2, 0.4]], [[0.4 + 1e-10, 0.6], [0.3, 0.5]]],
        1,
        tol,
    )
    .unwrap();
    assert!(valid.tessellate_bounded(0.01, 1, tol).is_err());
}

#[test]
fn individually_tolerant_vertices_and_curves_must_still_agree() {
    let tol = Tolerance::default();
    let mut face = NurbsHoledFace::new(
        source(),
        [[0., 1.], [0., 1.]],
        vec![[[0.3, 0.7], [0.3, 0.7]]],
        1,
        tol,
    )
    .unwrap();
    let Curve::Nurbs(curve) = &face.edges[4].curve else {
        panic!()
    };
    let shifted = hagane::NurbsCurve::new(
        curve.degree(),
        curve.knots().to_vec(),
        curve
            .control_points()
            .iter()
            .map(|p| *p - Point3::new(0.75 * tol.linear, 0., 0.))
            .collect(),
        curve.weights().to_vec(),
    )
    .unwrap();
    face.edges[4].curve = Curve::Nurbs(Box::new(shifted));
    for vertex in &mut face.vertices {
        vertex.point = vertex.point + Point3::new(0.75 * tol.linear, 0., 0.);
    }
    assert!(face.validate_boundary(tol).is_err());
}

#[test]
fn refinement_resource_and_conservative_full_source_singular_errors() {
    let count = 256;
    let mut knots = vec![0., 0.];
    knots.extend((1..count - 1).map(|i| i as f64));
    knots.extend([count as f64, count as f64]);
    let huge = NurbsSurface::new(
        [1, 1],
        [knots.clone(), knots],
        [count, count],
        vec![Point3::new(0., 0., 0.); count * count],
        vec![1.; count * count],
    )
    .unwrap();
    assert!(matches!(
        NurbsHoledFace::new(
            huge,
            [[0., 256.], [0., 256.]],
            vec![[[0.25, 0.75], [0.25, 0.75]]],
            1,
            Tolerance::default()
        ),
        Err(hagane::Error::Unsupported(
            "NURBS hole refinement exceeds control limit"
        ))
    ));
    // Structurally valid singular data inside the removed UV region still fails
    // the conservative whole-source normal check, rather than partial success.
    let knots = vec![0., 0., 0., 1., 1., 1.];
    let squared = [0.25, -0.25, 0.25];
    let mut points = Vec::new();
    for i in 0..3 {
        for j in 0..3 {
            points.push(Point3::new(
                squared[i] - squared[j],
                2. * (i as f64 / 2. - 0.5) * (j as f64 / 2. - 0.5),
                0.,
            ));
        }
    }
    let singular =
        NurbsSurface::new([2, 2], [knots.clone(), knots], [3, 3], points, vec![1.; 9]).unwrap();
    assert!(singular.normal(0.5, 0.5).is_err());
    for i in 0..=8 {
        for j in 0..=8 {
            let u = i as f64 / 8.;
            let v = j as f64 / 8.;
            if !(u > 0.25 && u < 0.75 && v > 0.25 && v < 0.75) {
                assert_eq!(singular.normal(u, v).unwrap(), Point3::new(0., 0., 1.));
            }
        }
    }
    let face = NurbsHoledFace::new(
        singular,
        [[0., 1.], [0., 1.]],
        vec![[[0.25, 0.75], [0.25, 0.75]]],
        1,
        Tolerance::default(),
    )
    .unwrap();
    assert!(face
        .tessellate_bounded(0.01, 4096, Tolerance::default())
        .is_err());
}
#[test]
fn zero_hole_face_remains_an_exact_open_rectangular_patch() {
    let tol = Tolerance::default();
    let face = NurbsHoledFace::new(source(), [[0.1, 0.9], [0.2, 0.8]], vec![], 1, tol).unwrap();
    assert!(face.holes().is_empty());
    assert_eq!(face.face.wires.len(), 1);
    assert_eq!(face.vertices.len(), 4);
    face.validate_boundary(tol).unwrap();
    let mesh = face.tessellate_bounded(0.02, 4096, tol).unwrap();
    let area = mesh
        .uv_ranges
        .iter()
        .map(|r| (r[0][1] - r[0][0]) * (r[1][1] - r[1][0]))
        .sum::<f64>();
    assert!((area - 0.8 * 0.6).abs() < 1e-12);
}
