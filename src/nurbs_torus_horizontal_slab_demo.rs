//! Actual closed torus slab reports, separate from accepted full-source geometry.
use crate::*;
pub(crate) fn serialize_slab(
    body: &NurbsTorusHorizontalSlab,
    chord: f64,
    policy: GeometryTolerance,
) -> Result<serde_json::Value> {
    if !chord.is_finite() || chord <= 0. {
        return Err(Error::InvalidInput(
            "torus slab display chord must be finite and positive",
        ));
    }
    body.validate(policy)?;
    let xyz = |p: Point3| [p.x, p.y, p.z];
    let source = body.source();
    let frame = source.frame();
    let bounds = body.bounds(policy)?;
    let mesh = body.tessellate(chord, policy)?;
    Ok(
        serde_json::json!({"kind":"nurbs_torus_horizontal_slab","units":"mm","heights":body.heights(),"source_radii":source.radii(),"source_volume":source.volume(policy)?,"linear_tolerance":policy.linear(),"display_chord_tolerance":chord,"source_placement":{"translation":xyz(frame.origin()),"axes":frame.axes().map(|v|[v.x,v.y,v.z])},"volume":body.volume(policy)?,"bounds":{"min":xyz(bounds.min),"max":xyz(bounds.max)},"bounds_kind":"conservative positive-weight rational control hull","mesh":{"positions":mesh.positions.iter().map(|p|xyz(*p)).collect::<Vec<_>>(),"normals":mesh.normals.iter().map(|v|[v.x,v.y,v.z]).collect::<Vec<_>>(),"triangles":mesh.triangles,"face_ids":mesh.face_ids},"brep":crate::nurbs_graph_polygon_demo::serialize_graph_brep(body.solid())?,"step":body.export_step_mm(policy)?,"step_exact":true,"step_schema":"AUTOMOTIVE_DESIGN","scope":"actual closed rational torus slab between two strictly interior local horizontal planes; accepted source and separate horizontal pair unchanged; actual cached slab point, finite segment, line and ray queries; slab mass operations unsupported"}),
    )
}
