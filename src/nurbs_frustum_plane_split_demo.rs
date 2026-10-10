//! Two actual closed rational children of one strict oblique frustum split.
use crate::*;

fn part_report(
    body: &NurbsObliqueFrustumSolid,
    policy: GeometryTolerance,
    chord: f64,
) -> Result<serde_json::Value> {
    body.validate(policy)?;
    let mesh = body.tessellate(chord, policy)?;
    if mesh.triangles.len() > 33280 {
        return Err(Error::Unsupported(
            "oblique frustum child display exceeds 33280 triangles",
        ));
    }
    let bounds = body.bounds(policy)?;
    let xyz = |v: Vec3| [v.x, v.y, v.z];
    Ok(
        serde_json::json!({"is_lower":body.is_lower(),"volume":body.volume(policy)?,"bounds":{"min":xyz(bounds.min),"max":xyz(bounds.max)},"bounds_kind":"conservative rational control hull","mesh":{"positions":mesh.positions.into_iter().map(xyz).collect::<Vec<_>>(),"normals":mesh.normals.into_iter().map(xyz).collect::<Vec<_>>(),"triangles":mesh.triangles,"face_ids":mesh.face_ids},"brep":crate::nurbs_graph_polygon_demo::serialize_graph_brep(body.solid())?,"step":body.export_step_mm(policy)?,"step_exact":true,"step_schema":"AUTOMOTIVE_DESIGN","scope":"actual closed oblique frustum child; centroid, inertia and generic import unsupported"}),
    )
}

/// Model/display values, world plane origin/normal XYZ, selected 0=lower/1=upper.
pub fn nurbs_frustum_plane_split_demo_json(x: &[f64]) -> Result<String> {
    if x.len() != 16 || x.iter().any(|v| !v.is_finite()) || (x[15] != 0. && x[15] != 1.) {
        return Err(Error::InvalidInput(
            "oblique frustum split requires sixteen finite values and selection zero or one",
        ));
    }
    let (source, policy) = crate::nurbs_frustum_demo::query_body(&x[..9])?;
    let origin = Point3::new(x[9], x[10], x[11]);
    let normal = Vec3::new(x[12], x[13], x[14]).normalized()?;
    let reference = if normal.x.abs() < 0.9 {
        Vec3::new(1., 0., 0.)
    } else {
        Vec3::new(0., 1., 0.)
    };
    let u = reference.cross(normal).normalized()?;
    let v = normal.cross(u).normalized()?;
    let split = source.split_by_plane(&Surface::Plane { origin, u, v }, policy)?;
    let parts = [
        part_report(&split.lower, policy, x[8])?,
        part_report(&split.upper, policy, x[8])?,
    ];
    let selected = x[15] as usize;
    let xyz = |v: Vec3| [v.x, v.y, v.z];
    let geometry = |c: &NurbsCurve| serde_json::json!({"kind":"nurbs","degree":c.degree(),"knots":c.knots(),"weights":c.weights(),"control_points":c.control_points().iter().copied().map(xyz).collect::<Vec<_>>(),"parameter_range":c.domain()});
    let mut curves = Vec::new();
    for (edge, usage) in split.section.edges().iter().zip(split.section.uses()) {
        let (Curve::Nurbs(curve), PCurve::Nurbs(pcurve)) = (&edge.curve, &usage.pcurve) else {
            return Err(Error::InvalidTopology(
                "oblique frustum section requires actual rational curves",
            ));
        };
        let line = curve.tessellate_bounded(x[8], 16384)?;
        curves.push(serde_json::json!({"face_id":usage.face_id,"vertices":edge.vertices,"curve":geometry(curve),"pcurve":geometry(pcurve),"points":line.points.into_iter().map(xyz).collect::<Vec<_>>(),"parameters":line.parameters,"error_bounds":line.error_bounds}));
    }
    let frame = source.frame();
    Ok(serde_json::json!({"units":"mm","placement":{"translation":xyz(frame.origin()),"axes":frame.axes().map(xyz)},"linear_tolerance":policy.linear(),"display_chord_tolerance":x[8],"source_volume":source.volume(policy)?,"part_volumes":[parts[0]["volume"],parts[1]["volume"]],"selected":selected,"step":parts[selected]["step"],"step_exact":true,"step_schema":"AUTOMOTIVE_DESIGN","parts":parts,"section":{"plane_origin":xyz(origin),"plane_normal":xyz(normal),"vertices":split.section.vertices().iter().map(|v|xyz(v.point)).collect::<Vec<_>>(),"curves":curves,"closed":true,"pcurve_scope":"source side face parameters"},"scope":"two actual closed rational oblique frustum children; selected STEP is the actual child, both child exports are available; conservative control-hull bounds; centroid, inertia, generic STEP import and general NURBS Booleans unsupported"}).to_string())
}
