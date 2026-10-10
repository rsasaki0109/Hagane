//! Display and metrics of the actual checked translated identity-axis frustum.
use crate::*;

pub fn nurbs_frustum_translated_step_import_demo_json(text: &str, chord: f64) -> Result<String> {
    let policy = GeometryTolerance::default();
    let body = import_step_nurbs_frustum_translated_mm(text, policy)?;
    let mut data = crate::nurbs_frustum_demo::serialize_frustum(&body, policy, chord, 0.)?;
    data["scope"] = serde_json::json!("checked translated identity-axis positive-radius rational frustum STEP import in mm; rotated frusta, apex cones and general STEP import remain unsupported");
    data["import"] = serde_json::json!({
        "kind":"nurbs_frustum_translated", "units":"mm", "validated_geometry":true,
        "source_geometry_preserved":true, "pcurve_representation_verified":true,
        "scope":"identity-axis positive-radius circular frustum with checked representable translation, eight vertices, twelve edges and six faces"
    });
    Ok(data.to_string())
}
