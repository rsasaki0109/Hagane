//! Reports the actual retained bored-oblique solid; unsupported queries stay explicit.
use crate::*;

pub(crate) fn serialize_bored_oblique_frustum(
    body: &NurbsBoredObliqueFrustumSolid,
    policy: GeometryTolerance,
    chord: f64,
) -> Result<serde_json::Value> {
    body.validate(policy)?;
    let mesh = body.tessellate(chord, policy)?;
    let bounds = body.bounds(policy)?;
    let xyz = |p: Vec3| [p.x, p.y, p.z];
    Ok(serde_json::json!({
        "is_lower":body.is_lower(),"hole_radius":body.source().radius(),"volume":body.volume(policy)?,
        "bounds":{"min":xyz(bounds.min),"max":xyz(bounds.max)},"bounds_kind":"conservative outer rational control hull",
        "mesh":{"positions":mesh.positions.into_iter().map(xyz).collect::<Vec<_>>(),"normals":mesh.normals.into_iter().map(xyz).collect::<Vec<_>>(),"triangles":mesh.triangles,"face_ids":mesh.face_ids},
        "brep":crate::nurbs_graph_polygon_demo::serialize_graph_brep(body.solid())?,
        "step":body.export_step_mm(policy)?,"step_exact":true,"step_schema":"AUTOMOTIVE_DESIGN",
        "mass_properties":{"ok":false,"error":"bored oblique frustum centroid and inertia are not implemented"},
        "scope":"actual closed genus-one bored oblique frustum; centroid, inertia, point/segment/line/ray queries, repeated cuts and generic import unsupported"
    }))
}
