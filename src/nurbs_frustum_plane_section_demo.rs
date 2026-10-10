//! Actual rational oblique plane section of a typed frustum; no modeling cut.
use crate::*;

/// Model/display values, then world plane origin XYZ and plane normal XYZ.
pub fn nurbs_frustum_plane_section_demo_json(x: &[f64]) -> Result<String> {
    if x.len() != 15 || x.iter().any(|v| !v.is_finite()) {
        return Err(Error::InvalidInput(
            "frustum plane section requires fifteen finite values",
        ));
    }
    let (body, policy) = crate::nurbs_frustum_demo::query_body(&x[..9])?;
    let origin = Point3::new(x[9], x[10], x[11]);
    let normal = Vec3::new(x[12], x[13], x[14]).normalized()?;
    let reference = if normal.x.abs() < 0.9 {
        Vec3::new(1., 0., 0.)
    } else {
        Vec3::new(0., 1., 0.)
    };
    let u = reference.cross(normal).normalized()?;
    let v = normal.cross(u).normalized()?;
    let plane = Surface::Plane { origin, u, v };
    let section = body.section_by_plane(&plane, policy)?;
    section.validate(policy)?;
    let xyz = |p: Vec3| [p.x, p.y, p.z];
    let geometry = |c: &NurbsCurve| serde_json::json!({"kind":"nurbs","degree":c.degree(),"knots":c.knots(),"weights":c.weights(),"control_points":c.control_points().iter().copied().map(xyz).collect::<Vec<_>>(),"parameter_range":c.domain()});
    let mut curves = Vec::with_capacity(4);
    for (edge, usage) in section.edges().iter().zip(section.uses()) {
        let (Curve::Nurbs(curve), PCurve::Nurbs(pcurve)) = (&edge.curve, &usage.pcurve) else {
            return Err(Error::InvalidTopology(
                "frustum section requires retained rational edge and UV curves",
            ));
        };
        let display = curve.tessellate_bounded(x[8], 16384)?;
        curves.push(serde_json::json!({"face_id":usage.face_id,"vertices":edge.vertices,"curve":geometry(curve),"pcurve":geometry(pcurve),"points":display.points.into_iter().map(xyz).collect::<Vec<_>>(),"parameters":display.parameters,"error_bounds":display.error_bounds}));
    }
    let mut data = crate::nurbs_frustum_demo::serialize_frustum(&body, policy, x[8], x[3])?;
    let frame = body.frame();
    data["placement"] =
        serde_json::json!({"translation":xyz(frame.origin()),"axes":frame.axes().map(xyz)});
    data["section"] = serde_json::json!({"plane_origin":xyz(origin),"plane_normal":xyz(normal),"vertices":section.vertices().iter().map(|vertex|xyz(vertex.point)).collect::<Vec<_>>(),"curves":curves,"closed":true,"mesh_free":true,"display_kind":"actual rational curve bounded polylines","display_chord_tolerance":x[8],"stock_unchanged":true,"cut_created":false});
    data["scope"]=serde_json::json!("closed four-edge rational section strictly between the frustum caps; actual side same-parameter UV curves, bounded display polylines; section only, no solid split, cap face or section STEP export; STEP remains the original solid");
    Ok(data.to_string())
}
