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
    let mass_properties = match body.inertia_properties(policy) {
        Ok(mass) => {
            serde_json::json!({"ok":true,"volume":mass.volume,"centroid":xyz(mass.centroid),"inertia":mass.inertia,"reference":"centroid","tensor_axes":"world","volume_units":"mm^3","centroid_units":"mm","inertia_units":"mm^5","density":1})
        }
        Err(Error::Unsupported(reason)) => serde_json::json!({"ok":false,"error":reason}),
        Err(error) => return Err(error),
    };
    Ok(serde_json::json!({
        "is_lower":body.is_lower(),"hole_radius":body.source().radius(),"volume":body.volume(policy)?,
        "bounds":{"min":xyz(bounds.min),"max":xyz(bounds.max)},"bounds_kind":"conservative outer rational control hull",
        "mesh":{"positions":mesh.positions.into_iter().map(xyz).collect::<Vec<_>>(),"normals":mesh.normals.into_iter().map(xyz).collect::<Vec<_>>(),"triangles":mesh.triangles,"face_ids":mesh.face_ids},
        "brep":crate::nurbs_graph_polygon_demo::serialize_graph_brep(body.solid())?,
        "step":body.export_step_mm(policy)?,"step_exact":true,"step_schema":"AUTOMOTIVE_DESIGN",
        "mass_properties":mass_properties,
        "scope":"actual closed genus-one bored oblique frustum; optional analytic mass properties; point/segment/line/ray queries, repeated cuts and generic import unsupported"
    }))
}
