//! Actual retained slab reports; unsupported metrics and queries remain explicit.
use crate::*;
fn report(
    solid: &Solid,
    mesh: Mesh,
    volume: f64,
    bounds: Bounds,
    step: String,
    bored: bool,
) -> Result<serde_json::Value> {
    let xyz = |p: Vec3| [p.x, p.y, p.z];
    Ok(
        serde_json::json!({"volume":volume,"bounds":{"min":xyz(bounds.min),"max":xyz(bounds.max)},"bounds_kind":"conservative rational control hull","mesh":{"positions":mesh.positions.into_iter().map(xyz).collect::<Vec<_>>(),"normals":mesh.normals.into_iter().map(xyz).collect::<Vec<_>>(),"triangles":mesh.triangles,"face_ids":mesh.face_ids},"brep":crate::nurbs_graph_polygon_demo::serialize_graph_brep(solid)?,"step":step,"step_exact":true,"step_schema":"AUTOMOTIVE_DESIGN","closed":true,"genus":if bored{1}else{0},"mass_properties":{"ok":false,"error":"slab centroid and inertia are unsupported"},"scope":"actual closed rational slab with parallel planar caps; centroid, inertia, point, segment, line and ray queries and generic STEP import unsupported"}),
    )
}
pub(crate) fn serialize_frustum_slab(
    body: &NurbsFrustumSlabSolid,
    policy: GeometryTolerance,
    chord: f64,
) -> Result<serde_json::Value> {
    body.validate(policy)?;
    report(
        body.solid(),
        body.tessellate(chord, policy)?,
        body.volume(policy)?,
        body.bounds(policy)?,
        body.export_step_mm(policy)?,
        false,
    )
}
pub(crate) fn serialize_bored_frustum_slab(
    body: &NurbsBoredFrustumSlabSolid,
    policy: GeometryTolerance,
    chord: f64,
) -> Result<serde_json::Value> {
    body.validate(policy)?;
    let mut data = report(
        body.solid(),
        body.tessellate(chord, policy)?,
        body.volume(policy)?,
        body.bounds(policy)?,
        body.export_step_mm(policy)?,
        true,
    )?;
    data["hole_radius"] = serde_json::json!(body.source().radius());
    Ok(data)
}
