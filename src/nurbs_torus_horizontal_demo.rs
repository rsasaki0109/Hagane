//! Actual horizontal torus child reports; no accepted-source replacement.
use crate::*;
pub(crate) fn serialize_horizontal_split(
    source: &NurbsTorusSolid,
    split: &NurbsTorusHorizontalSplit,
    chord: f64,
    policy: GeometryTolerance,
) -> Result<serde_json::Value> {
    let height = split.lower.cut_height();
    // Serialize both actual children completely before reporting success.
    let lower = serialize_child(&split.lower, policy, chord)?;
    let upper = serialize_child(&split.upper, policy, chord)?;
    let xyz = |p: Point3| [p.x, p.y, p.z];
    let frame = source.frame();
    let axes = frame.axes().map(|v| [v.x, v.y, v.z]);
    Ok(
        serde_json::json!({"kind":"nurbs_torus_horizontal_split","units":"mm","cut_height":height,"source_volume":source.volume(policy)?,"source_radii":source.radii(),"source_placement":{"translation":xyz(frame.origin()),"axes":axes},"cut_plane":{"point":xyz(frame.point(Point3::new(0.,0.,height))),"normal":axes[2]},"lower":lower,"upper":upper,"scope":"two actual closed horizontally bounded rational torus children; accepted full torus unchanged; actual cached child point queries; optional analytic child mass and world centroidal inertia; child segment, line and ray operations unsupported"}),
    )
}
fn serialize_child(
    body: &NurbsTorusHorizontalSolid,
    policy: GeometryTolerance,
    chord: f64,
) -> Result<serde_json::Value> {
    body.validate(policy)?;
    let xyz = |p: Point3| [p.x, p.y, p.z];
    let bounds = body.bounds(policy)?;
    let mesh = body.tessellate(chord, policy)?;
    let mass = match body.inertia_properties(policy) {
        Ok(m) => {
            serde_json::json!({"ok":true,"volume":m.volume,"centroid":xyz(m.centroid),"inertia":m.inertia,"reference":"centroid","tensor_axes":"world","volume_units":"mm^3","centroid_units":"mm","inertia_units":"mm^5","density":1})
        }
        Err(Error::Unsupported(reason)) => serde_json::json!({"ok":false,"error":reason}),
        Err(error) => return Err(error),
    };
    Ok(
        serde_json::json!({"volume":body.volume(policy)?,"mass_properties":mass,"is_lower":body.is_lower(),"bounds":{"min":xyz(bounds.min),"max":xyz(bounds.max)},"bounds_kind":"conservative positive-weight rational control hull","mesh":{"positions":mesh.positions.iter().map(|p|xyz(*p)).collect::<Vec<_>>(),"normals":mesh.normals.iter().map(|v|[v.x,v.y,v.z]).collect::<Vec<_>>(),"triangles":mesh.triangles,"face_ids":mesh.face_ids},"brep":crate::nurbs_graph_polygon_demo::serialize_graph_brep(body.solid())?,"step":body.export_step_mm(policy)?,"step_exact":true,"step_schema":"AUTOMOTIVE_DESIGN"}),
    )
}
