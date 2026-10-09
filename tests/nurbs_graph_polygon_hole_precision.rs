use hagane::*;

fn square(a: f64, b: f64) -> Vec<[f64; 2]> {
    vec![[a, a], [b, a], [b, b], [a, b]]
}

#[test]
fn thin_material_mass_uses_positive_regions_without_stock_subtraction() {
    let t = Tolerance::new(1e-10).unwrap();
    let source = NurbsGraphSolid::new([1., 1., 1.], 0., t).unwrap();
    let d = 2f64.powi(-26);
    let body = source.through_uv_polygon(square(d, 1. - d), t).unwrap();
    let expected = 4. * d * (1. - d);
    let mass = body.mass_properties(t).unwrap();
    assert!((mass.volume - expected).abs() < expected * 2e-13);
    assert!((mass.centroid - Point3::new(0.5, 0.5, 0.5)).norm() < 2e-13);
    let diamond = |r: f64| {
        vec![
            [0.5, 0.5 - r],
            [0.5 + r, 0.5],
            [0.5, 0.5 + r],
            [0.5 - r, 0.5],
        ]
    };
    let outer = NurbsGraphPolygonSolid::new(&source, diamond(0.375), t).unwrap();
    let body = outer.through_uv_polygon(diamond(0.375 - d), t).unwrap();
    let expected = 4. * 0.375 * d - 2. * d * d;
    let mass = body.mass_properties(t).unwrap();
    assert!(
        (mass.volume - expected).abs() < expected * 2e-13,
        "{} vs {expected}",
        mass.volume
    );
}

#[test]
fn strict_containment_and_physical_clearance_reject_touching_and_near_walls() {
    let t = Tolerance::default();
    let source = NurbsGraphSolid::new([1., 1., 1.], 0., t).unwrap();
    for hole in [
        square(0., 0.8),
        square(-0.01, 0.8),
        square(3. * t.linear, 0.8),
    ] {
        assert!(source.through_uv_polygon(hole, t).is_err());
    }
    source
        .through_uv_polygon(square(8. * t.linear, 0.8), t)
        .unwrap();
    let mut clockwise = square(0.2, 0.8);
    clockwise.reverse();
    assert!(source.through_uv_polygon(clockwise, t).is_err());
    for hole in [
        vec![[0.2, 0.2], [0.8, 0.2]],
        vec![[0.2, 0.2], [0.8, 0.2], [0.4, 0.4], [0.8, 0.8], [0.2, 0.8]],
        vec![[f64::NAN, 0.2], [0.8, 0.2], [0.5, 0.8]],
    ] {
        assert!(source.through_uv_polygon(hole, t).is_err());
    }
}

#[test]
fn physical_scaling_and_public_canonical_topology_remain_strict() {
    for scale in [1e-6, 1., 100.] {
        let t = Tolerance::new(scale * 1e-8).unwrap();
        let source = NurbsGraphSolid::new([scale; 3], 0., t).unwrap();
        let body = source.through_uv_polygon(square(0.25, 0.75), t).unwrap();
        assert!((body.volume().unwrap() / scale.powi(3) - 0.75).abs() < 1e-13);
        let mut dirty = body.clone();
        dirty.solid.vertices[0].point.x += t.linear / 100.;
        assert!(dirty.validate(t).is_err());
        let mut dirty = body.clone();
        dirty.solid.shell.faces[0].wires[1].coedges[0].forward ^= true;
        assert!(dirty.validate(t).is_err());
        let mut dirty = body.clone();
        dirty.solid.shell.faces[1].wires[1].coedges[0].edge = 0;
        assert!(dirty.validate(t).is_err());
    }
}

// Test the entire segment against the open void, rather than testing only
// triangle centroids. A segment enters an axis-aligned open square precisely
// when its open clipping interval is nonempty.
fn enters_void(a: [f64; 2], b: [f64; 2]) -> bool {
    let mut lower: f64 = 0.;
    let mut upper: f64 = 1.;
    for axis in 0..2 {
        let delta = b[axis] - a[axis];
        if delta == 0. {
            if a[axis] <= 0.25 || a[axis] >= 0.75 {
                return false;
            }
        } else {
            let x = (0.25 - a[axis]) / delta;
            let y = (0.75 - a[axis]) / delta;
            lower = lower.max(x.min(y));
            upper = upper.min(x.max(y));
        }
    }
    lower < upper
}

#[test]
fn every_material_mesh_triangle_edge_avoids_the_open_void() {
    let t = Tolerance::default();
    let source = NurbsGraphSolid::new([1.; 3], 0., t).unwrap();
    let body = source.through_uv_polygon(square(0.25, 0.75), t).unwrap();
    let mesh = body.tessellate_bounded(0.01, 65536, t).unwrap();
    for triangle in &mesh.mesh.triangles {
        let ids = *triangle;
        if mesh.vertex_faces[ids[0]] > 1 {
            continue;
        }
        for i in 0..3 {
            assert!(!enters_void(
                mesh.vertex_uv[ids[i]],
                mesh.vertex_uv[ids[(i + 1) % 3]]
            ));
        }
    }
}

#[test]
fn sixteen_opening_corners_and_strict_curve_weight_identity() {
    let t = Tolerance::default();
    let source = NurbsGraphSolid::new([1.; 3], 0., t).unwrap();
    let ring = |count: usize| {
        (0..count)
            .map(|i| {
                let a = std::f64::consts::TAU * (i as f64 + 0.37) / count as f64;
                [0.5 + 0.2 * a.cos(), 0.5 + 0.2 * a.sin()]
            })
            .collect()
    };
    let body = source.through_uv_polygon(ring(16), t).unwrap();
    body.validate(t).unwrap();
    assert!(source.through_uv_polygon(ring(17), t).is_err());
    let mut dirty = body.clone();
    let Curve::Nurbs(curve) = &dirty.solid.edges[4].curve else {
        panic!()
    };
    let mut weights = curve.weights().to_vec();
    weights[1] = f64::from_bits(weights[1].to_bits() + 1);
    dirty.solid.edges[4].curve = Curve::Nurbs(Box::new(
        NurbsCurve::new(
            curve.degree(),
            curve.knots().to_vec(),
            curve.control_points().to_vec(),
            weights,
        )
        .unwrap(),
    ));
    assert!(dirty.validate(t).is_err());
    let mut dirty = body.clone();
    let Curve::Nurbs(curve) = &dirty.solid.edges[4].curve else {
        panic!()
    };
    let mut controls = curve.control_points().to_vec();
    controls[1].z += t.linear / 100.;
    dirty.solid.edges[4].curve = Curve::Nurbs(Box::new(
        NurbsCurve::new(
            curve.degree(),
            curve.knots().to_vec(),
            controls,
            curve.weights().to_vec(),
        )
        .unwrap(),
    ));
    assert!(dirty.validate(t).is_err());
}
