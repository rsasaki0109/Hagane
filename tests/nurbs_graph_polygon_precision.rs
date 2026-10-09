use hagane::*;

fn source() -> NurbsGraphSolid {
    NurbsGraphSolid::new([3., 2., 4.], 0.5, Tolerance::default()).unwrap()
}

#[test]
fn sixteen_convex_corners_are_supported_and_seventeen_are_rejected() {
    let t = Tolerance::default();
    let source = source();
    let polygon = |n: usize| {
        (0..n)
            .map(|i| {
                let angle = std::f64::consts::TAU * i as f64 / n as f64;
                [0.5 + 0.4 * angle.cos(), 0.5 + 0.4 * angle.sin()]
            })
            .collect()
    };
    let body = NurbsGraphPolygonSolid::new(&source, polygon(16), t).unwrap();
    assert_eq!(body.brep().vertices.len(), 32);
    assert_eq!(body.brep().edges.len(), 48);
    assert_eq!(body.brep().shell.faces.len(), 18);
    body.validate(t).unwrap();
    assert!(body.volume().unwrap() > 0.);
    assert!(NurbsGraphPolygonSolid::new(&source, polygon(17), t).is_err());
}

#[test]
fn physical_edges_and_almost_collinear_convex_corners_are_resolved() {
    let t = Tolerance::default();
    let source = source();
    let thin = |width: f64| vec![[0.5, 0.5], [0.5 + width / 3., 0.5], [0.5, 0.8]];
    assert!(NurbsGraphPolygonSolid::new(&source, thin(3. * t.linear), t).is_err());
    let good = NurbsGraphPolygonSolid::new(&source, thin(6. * t.linear), t).unwrap();
    good.validate(t).unwrap();
    assert!(good.mass_properties(t).unwrap().volume > 0.);
    let shallow = |dip: f64| {
        vec![
            [0.2, 0.2],
            [0.5, 0.2 - dip],
            [0.8, 0.2],
            [0.8, 0.8],
            [0.2, 0.8],
        ]
    };
    assert!(NurbsGraphPolygonSolid::new(&source, shallow(t.linear / 2.), t).is_err());
    NurbsGraphPolygonSolid::new(&source, shallow(4. * t.linear), t)
        .unwrap()
        .validate(t)
        .unwrap();
}

#[test]
fn original_retained_uv_domain_and_world_precision_are_enforced() {
    let t = Tolerance::default();
    let source = source().trimmed_uv([[0.2, 0.8], [0.3, 0.9]], t).unwrap();
    let body =
        NurbsGraphPolygonSolid::new(&source, vec![[0.25, 0.35], [0.75, 0.35], [0.5, 0.85]], t)
            .unwrap();
    body.validate(t).unwrap();
    assert_eq!(body.source().source_domain(), [[0.2, 0.8], [0.3, 0.9]]);
    for p in [
        vec![[0.1, 0.3], [0.8, 0.3], [0.5, 0.9]],
        vec![[0.2, 0.2], [0.8, 0.3], [0.5, 0.9]],
        vec![[0.2, 0.3], [0.9, 0.3], [0.5, 0.9]],
    ] {
        assert!(NurbsGraphPolygonSolid::new(&source, p, t).is_err());
    }
    let remote = Transform::translation(Point3::new(1e12, -1e12, 1e12)).unwrap();
    assert!(source.transformed(remote, t).is_err());
    let placed = source
        .transformed(
            Transform::translation(Point3::new(20., 30., -10.)).unwrap(),
            t,
        )
        .unwrap();
    NurbsGraphPolygonSolid::new(&placed, vec![[0.25, 0.35], [0.75, 0.35], [0.5, 0.8]], t)
        .unwrap()
        .validate(t)
        .unwrap();
}

#[test]
fn rounded_affine_endpoint_outside_closed_trim_domain_is_conservatively_rejected() {
    let tolerance = Tolerance::default();
    let source = source()
        .trimmed_uv([[0.2, 0.8], [0.3, 0.9]], tolerance)
        .unwrap();
    let pcurve = PCurve::Affine {
        origin: [0.8, 0.3],
        direction: [0.5 - 0.8, 0.9 - 0.3],
    };
    assert!(pcurve.evaluate(1.)[1] > 0.9);
    // Geometrically on the boundary, but the checked affine evaluation is not
    // snapped back into the source's closed parameter domain.
    assert!(NurbsGraphPolygonSolid::new(
        &source,
        vec![[0.2, 0.3], [0.8, 0.3], [0.5, 0.9]],
        tolerance
    )
    .is_err());
}

#[test]
fn tiny_nonzero_uv_component_is_rejected_without_snapping_to_horizontal() {
    let tolerance = Tolerance::default();
    let source = source();
    let polygon = vec![
        [0.9, 0.5],
        [0.7, 0.85],
        [0.3, 0.85],
        [0.1, 0.5],
        [0.3, 0.15],
        [0.7, 0.15],
    ];
    let body = NurbsGraphPolygonSolid::new(&source, polygon.clone(), tolerance).unwrap();
    body.validate(tolerance).unwrap();
    let mut near_horizontal = polygon;
    near_horizontal[2][1] = f64::from_bits(near_horizontal[2][1].to_bits() + 1);
    assert_ne!(near_horizontal[1][1], near_horizontal[2][1]);
    assert!(matches!(
        NurbsGraphPolygonSolid::new(&source, near_horizontal, tolerance),
        Err(Error::Unsupported(
            "surface UV direction is too short for stable affine composition"
        ))
    ));
}

#[test]
fn subtol_canonical_curve_surface_weight_and_pcurve_mutations_are_rejected() {
    let t = Tolerance::default();
    let original = NurbsGraphPolygonSolid::new(
        &source(),
        vec![[0.2, 0.2], [0.8, 0.2], [0.7, 0.8], [0.3, 0.8]],
        t,
    )
    .unwrap();
    let mut modified = original.clone();
    let Curve::Nurbs(curve) = &modified.solid.edges[0].curve else {
        panic!()
    };
    let mut points = curve.control_points().to_vec();
    points[1].z += t.linear / 100.;
    modified.solid.edges[0].curve = Curve::Nurbs(Box::new(
        NurbsCurve::new(
            curve.degree(),
            curve.knots().to_vec(),
            points,
            curve.weights().to_vec(),
        )
        .unwrap(),
    ));
    assert!(modified.validate(t).is_err());
    assert!(modified.volume().is_err());
    let mut modified = original.clone();
    let Curve::Nurbs(curve) = &modified.solid.edges[0].curve else {
        panic!()
    };
    let mut weights = curve.weights().to_vec();
    weights[1] = f64::from_bits(weights[1].to_bits() + 1);
    modified.solid.edges[0].curve = Curve::Nurbs(Box::new(
        NurbsCurve::new(
            curve.degree(),
            curve.knots().to_vec(),
            curve.control_points().to_vec(),
            weights,
        )
        .unwrap(),
    ));
    assert!(modified.validate(t).is_err());
    let mut modified = original.clone();
    let Surface::Nurbs(surface) = &modified.solid.shell.faces[1].surface else {
        panic!()
    };
    let mut points = surface.control_points().to_vec();
    points[4].z += t.linear / 100.;
    modified.solid.shell.faces[1].surface = Surface::Nurbs(Box::new(
        NurbsSurface::new(
            surface.degrees(),
            [
                surface.knots(0).unwrap().to_vec(),
                surface.knots(1).unwrap().to_vec(),
            ],
            surface.control_counts(),
            points,
            surface.weights().to_vec(),
        )
        .unwrap(),
    ));
    assert!(modified.validate(t).is_err());
    assert!(modified.bounds().is_err());
    let mut modified = original.clone();
    let PCurve::Affine { origin, .. } =
        &mut modified.solid.shell.faces[0].wires[0].coedges[0].pcurve
    else {
        panic!()
    };
    origin[0] = f64::from_bits(origin[0].to_bits() + 1);
    assert!(modified.validate(t).is_err());
    assert!(modified.tessellate_bounded(0.01, 65536, t).is_err());
}
