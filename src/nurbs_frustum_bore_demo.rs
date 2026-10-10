//! Actual closed rational frustum with one coaxial through opening.
use crate::*;

/// Existing nine frustum model/display values followed by the bore radius.
pub fn nurbs_frustum_bore_demo_json(x: &[f64]) -> Result<String> {
    if x.len() != 10 || x.iter().any(|v| !v.is_finite()) {
        return Err(Error::InvalidInput(
            "coaxial frustum bore requires exactly ten finite values",
        ));
    }
    let (source, policy) = crate::nurbs_frustum_demo::query_body(&x[..9])?;
    let body = source.bore_coaxial(x[9], policy)?;
    body.validate(policy)?;
    let mesh = body.tessellate(x[8], policy)?;
    let xyz = |v: Vec3| [v.x, v.y, v.z];
    let bounds = body.bounds(policy)?;
    let mass_properties = match body.inertia_properties(policy) {
        Ok(mass) => {
            serde_json::json!({"ok":true,"volume":mass.volume,"centroid":xyz(mass.centroid),"inertia":mass.inertia,"reference":"centroid","tensor_axes":"world","volume_units":"mm^3","centroid_units":"mm","inertia_units":"mm^5","density":1.})
        }
        Err(Error::Unsupported(reason)) => serde_json::json!({"ok":false,"error":reason}),
        Err(error) => return Err(error),
    };
    let frame = body.source().frame();
    Ok(serde_json::json!({"units":"mm","kind":"coaxial_bored_frustum","radii":body.source().radii(),"height":body.source().height(),"hole_radius":body.radius(),"placement":{"translation":xyz(frame.origin()),"axes":frame.axes().map(xyz)},"axis":xyz(frame.axes()[2]),"linear_tolerance":policy.linear(),"display_chord_tolerance":x[8],"source_volume":body.source().volume(policy)?,"removed_volume":body.tool().volume(policy)?,"volume":body.volume(policy)?,"mass_properties":mass_properties,"bounds":{"min":xyz(bounds.min),"max":xyz(bounds.max)},"bounds_kind":"outer endpoint-circle axis envelopes","mesh":{"positions":mesh.positions.into_iter().map(xyz).collect::<Vec<_>>(),"normals":mesh.normals.into_iter().map(xyz).collect::<Vec<_>>(),"triangles":mesh.triangles,"face_ids":mesh.face_ids},"brep":crate::nurbs_graph_polygon_demo::serialize_graph_brep(body.solid())?,"step":body.export_step_mm(policy)?,"step_exact":true,"step_schema":"AUTOMOTIVE_DESIGN","closed":true,"genus":1,"source_unchanged":true,"scope":"actual closed rational frustum with one strictly contained coaxial cylindrical through opening; annular caps and inward rational walls; optional checked mass properties; generic STEP import, off-axis bores, blind cavities and general Booleans unsupported"}).to_string())
}
