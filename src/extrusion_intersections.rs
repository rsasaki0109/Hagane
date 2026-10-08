//! Checked line intersections with exact circular translation surfaces.
use crate::*;
/// Intersect an infinite line with the full angular lateral surface of a
/// circular extrusion, bounded only by its normal span. Reuses the typed
/// point/contact/generator results of `intersect_line_cylinder`; face angular
/// trims and end caps are not intersected. Returned parameters use the caller's
/// original direction, not a transformed or normalized direction.
/// Conservative shear-scaled guards reject unresolved contacts and conditioning.
pub fn intersect_line_extruded_circle(
    anchor: Point3,
    direction: Vec3,
    surface: &Surface,
    tol: GeometryTolerance,
) -> Result<LineCylinderIntersection> {
    let Surface::ExtrudedCircle {
        frame,
        radius,
        height,
        drift,
    } = *surface
    else {
        return Err(Error::Unsupported(
            "line circular-extrusion intersection requires an ExtrudedCircle surface",
        ));
    };
    let skew = drift[0].hypot(drift[1]);
    let generator = Vec3::new(drift[0], drift[1], 1.);
    let extent = radius.max(height * generator.norm());
    let stretch = 1. + skew; // ||unshear|| <= ||I|| + ||rank-one shear||.
    if !anchor.finite()
        || !radius.is_finite()
        || radius <= 10. * tol.linear()
        || !height.is_finite()
        || height <= 10. * tol.linear()
        || !skew.is_finite()
        || !extent.is_finite()
        || !stretch.is_finite()
    {
        return Err(Error::InvalidInput(
            "invalid circular extrusion dimensions or finite range",
        ));
    }
    let budget = tol.length_at_scale(extent)?;
    let proxy_budget = budget * stretch;
    if !proxy_budget.is_finite() || proxy_budget <= 0. {
        return Err(Error::InvalidInput(
            "circular extrusion conditioning exceeds finite tolerance range",
        ));
    }
    let unshear = |p: Vec3| Vec3::new(p.x - drift[0] * p.z, p.y - drift[1] * p.z, p.z);
    let unit = direction.normalized()?;
    let local_unit = frame.local_vector(unit);
    let a = unshear(frame.local_point(anchor));
    let d = unshear(local_unit);
    if !a.finite() || !d.finite() {
        return Err(Error::InvalidInput(
            "unsheared line exceeds finite coordinate range",
        ));
    }
    if d.x.hypot(d.y) > 0.
        && local_unit.cross(generator.normalized()?).norm() <= tol.angular().sin()
    {
        return Err(Error::Unsupported(
            "near-generator circular extrusion intersection is unresolved under angular policy",
        ));
    }
    let proxy = Surface::Cylinder {
        center: Point3::new(0., 0., 0.),
        radius,
        height,
    };
    let proxy_tol = GeometryTolerance::new(proxy_budget, tol.angular(), 0.)?;
    let result = intersect_line_cylinder(a, d, &proxy, proxy_tol)?;
    // The proxy line parameter multiplies the unsheared WORLD unit direction,
    // and is therefore travel along the original world-space unit direction.
    let recover = |travel: f64, uv: [f64; 2]| -> Result<(Point3, f64)> {
        let parameter = crate::intersections::line_parameter(travel, direction)?;
        let point = anchor + unit * travel;
        let evaluated = surface.evaluate(uv[0], uv[1]);
        let reconstructed = anchor + direction * parameter;
        let local = frame.local_point(point);
        let proxy_local = unshear(local);
        let expected = Vec3::new(
            radius * uv[0].cos() + drift[0] * uv[1],
            radius * uv[0].sin() + drift[1] * uv[1],
            uv[1],
        );
        if !local.finite()
            || !proxy_local.finite()
            || !expected.finite()
            || (local - expected).norm() > budget
            || (proxy_local.x.hypot(proxy_local.y) - radius).abs() > proxy_budget
        {
            return Err(Error::InvalidInput(
                "circular extrusion hit loses local dimensions at world coordinate magnitude",
            ));
        }
        if !point.finite()
            || !evaluated.finite()
            || !reconstructed.finite()
            || !tol.coincident(point, evaluated, extent)?
            || !tol.coincident(point, reconstructed, extent.max(travel.abs()))?
        {
            return Err(Error::InvalidInput(
                "circular extrusion hit loses world line/surface agreement",
            ));
        }
        Ok((point, parameter))
    };
    Ok(match result {
        LineCylinderIntersection::Empty => LineCylinderIntersection::Empty,
        LineCylinderIntersection::Points(points) => {
            let mut hits = Vec::new();
            for hit in points {
                let (point, parameter) = recover(hit.parameter, hit.uv)?;
                hits.push(CylinderIntersectionPoint {
                    point,
                    parameter,
                    uv: hit.uv,
                    contact: hit.contact,
                });
            }
            if hits.len() == 2
                && (hits[1].parameter <= hits[0].parameter
                    || (hits[1].point - hits[0].point).norm() <= budget)
            {
                return Err(Error::InvalidInput(
                    "circular extrusion roots are unresolved in world coordinates",
                ));
            }
            LineCylinderIntersection::Points(hits)
        }
        LineCylinderIntersection::Coincident {
            parameter_range,
            angle,
        } => {
            let levels = if d.z > 0. { [0., height] } else { [height, 0.] };
            let (_, first) = recover(parameter_range[0], [angle, levels[0]])?;
            let (_, last) = recover(parameter_range[1], [angle, levels[1]])?;
            if last <= first {
                return Err(Error::InvalidInput(
                    "circular extrusion generator interval is unrepresentable",
                ));
            }
            LineCylinderIntersection::Coincident {
                parameter_range: [first, last],
                angle,
            }
        }
    })
}
/// Actual native/WASM intersection fixture. Mode 0 is transverse; modes 1/2
/// follow a forward/reverse generator. Placement rotates both the line and solid.
pub fn extrusion_intersections_demo_json(mode: u32, offset: f64, placement: f64) -> Result<String> {
    if mode > 2 || !offset.is_finite() || !placement.is_finite() {
        return Err(Error::InvalidInput(
            "extrusion intersection demo requires mode 0..2 and finite values",
        ));
    }
    let tol = GeometryTolerance::default();
    let frame = Transform::rotation(Vec3::new(1., 2., 3.), placement)?
        .compose(Frame3::translation(Point3::new(0., 0., -12.))?)?;
    let surface = Surface::ExtrudedCircle {
        frame,
        radius: 24.,
        height: 24.,
        drift: [0.5, -0.25],
    };
    let p = ArcLineRegion {
        origin: Point3::new(0., 0., 0.),
        outer: vec![
            PlanarSegment::Arc {
                center: [0., 0.],
                radius: 24.,
                start_angle: 0.,
                sweep: std::f64::consts::PI,
            },
            PlanarSegment::Arc {
                center: [0., 0.],
                radius: 24.,
                start_angle: std::f64::consts::PI,
                sweep: std::f64::consts::PI,
            },
        ],
        holes: Vec::new(),
    };
    let solid = extrude_arc_line_region_in_frame(
        &p,
        frame.vector(Vec3::new(12., -6., 24.)),
        frame,
        tol.absolute(),
    )?;
    let (a, d) = match mode {
        0 => (Point3::new(-34., offset - 3., 12.), Vec3::new(2., 0., 0.)),
        1 => (Point3::new(24. + offset, 0., 0.), Vec3::new(12., -6., 24.)),
        _ => (
            Point3::new(36. + offset, -6., 24.),
            Vec3::new(-12., 6., -24.),
        ),
    };
    let (a, d) = (frame.point(a), frame.vector(d));
    let xyz = |p: Vec3| format!("[{},{},{}]", p.x, p.y, p.z);
    let result = match intersect_line_extruded_circle(a, d, &surface, tol)? {
        LineCylinderIntersection::Empty => "{\"kind\":\"empty\",\"hits\":[]}".into(),
        LineCylinderIntersection::Coincident {
            parameter_range,
            angle,
        } => format!(
            "{{\"kind\":\"coincident\",\"hits\":[],\"range\":[{},{}],\"angle\":{}}}",
            parameter_range[0], parameter_range[1], angle
        ),
        LineCylinderIntersection::Points(points) => {
            let hits:Vec<_>=points.iter().map(|p|format!("{{\"point\":{},\"normal\":{},\"parameter\":{},\"uv\":[{},{}],\"contact\":\"{}\"}}",xyz(p.point),xyz(surface.normal(p.uv[0])),p.parameter,p.uv[0],p.uv[1],if p.contact==IntersectionContact::Tangent {"tangent"} else {"crossing"})).collect();
            format!("{{\"kind\":\"points\",\"hits\":[{}]}}", hits.join(","))
        }
    };
    Ok(format!(
        "{{\"intersection\":{result},\"line\":{{\"anchor\":{},\"direction\":{}}},\"mesh\":{}}}",
        xyz(a),
        xyz(d),
        solid.mesh_json(0.05, tol.absolute())?
    ))
}
