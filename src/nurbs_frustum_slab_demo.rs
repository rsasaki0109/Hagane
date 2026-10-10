//! Actual retained slab reports with optional analytic metrics and explicit query limits.
use crate::*;
fn report(
    solid: &Solid,
    mesh: Mesh,
    volume: f64,
    bounds: Bounds,
    step: String,
    bored: bool,
    mass: Result<NurbsGraphInertiaProperties>,
) -> Result<serde_json::Value> {
    let xyz = |p: Vec3| [p.x, p.y, p.z];
    let mass_properties = match mass {
        Ok(mass) => {
            serde_json::json!({"ok":true,"volume":mass.volume,"centroid":xyz(mass.centroid),"inertia":mass.inertia,"reference":"centroid","tensor_axes":"world","volume_units":"mm^3","centroid_units":"mm","inertia_units":"mm^5","density":1})
        }
        Err(Error::Unsupported(reason)) => serde_json::json!({"ok":false,"error":reason}),
        Err(error) => return Err(error),
    };
    Ok(
        serde_json::json!({"volume":volume,"bounds":{"min":xyz(bounds.min),"max":xyz(bounds.max)},"bounds_kind":"conservative rational control hull","mesh":{"positions":mesh.positions.into_iter().map(xyz).collect::<Vec<_>>(),"normals":mesh.normals.into_iter().map(xyz).collect::<Vec<_>>(),"triangles":mesh.triangles,"face_ids":mesh.face_ids},"brep":crate::nurbs_graph_polygon_demo::serialize_graph_brep(solid)?,"step":step,"step_exact":true,"step_schema":"AUTOMOTIVE_DESIGN","closed":true,"genus":if bored{1}else{0},"mass_properties":mass_properties,"scope":"actual closed rational slab with parallel planar caps and optional analytic mass properties; actual point and finite segment queries; line and ray queries and generic STEP import unsupported"}),
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
        body.inertia_properties(policy),
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
        body.inertia_properties(policy),
    )?;
    data["hole_radius"] = serde_json::json!(body.source().radius());
    Ok(data)
}
