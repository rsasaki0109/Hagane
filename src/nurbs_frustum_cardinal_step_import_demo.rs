//! Display and metrics of the actual checked signed-cardinal-frame frustum.
use crate::*;

pub fn nurbs_frustum_cardinal_step_import_demo_json(text: &str, chord: f64) -> Result<String> {
    let policy = GeometryTolerance::default();
    let body = import_step_nurbs_frustum_cardinal_mm(text, policy)?;
    let mut data = crate::nurbs_frustum_demo::serialize_frustum(&body, policy, chord, 0.)?;
    let frame = body.frame();
    let xyz = |v: Vec3| [v.x, v.y, v.z];
    data["placement"] =
        serde_json::json!({"translation":xyz(frame.origin()), "axes":frame.axes().map(xyz)});
    data["scope"] = serde_json::json!("checked signed-cardinal-frame positive-radius rational frustum STEP import in mm; arbitrary rotations, apex cones and general STEP import remain unsupported");
    data["import"] = serde_json::json!({
        "kind":"nurbs_frustum_cardinal", "units":"mm", "validated_geometry":true,
        "source_geometry_preserved":true, "pcurve_representation_verified":true,
        "scope":"positive-radius circular frustum with one of 24 exact right-handed signed cardinal bases and checked representable translation, eight vertices, twelve edges and six faces"
    });
    Ok(data.to_string())
}
