//! Display and metrics of the actual checked, unplaced imported frustum B-rep.
use crate::*;

/// Import a scoped rational frustum STEP body and display its retained geometry.
pub fn nurbs_frustum_step_import_demo_json(text: &str, error: f64) -> Result<String> {
    let policy = GeometryTolerance::default();
    let body = import_step_nurbs_frustum_mm(text, policy)?;
    let mut data = crate::nurbs_frustum_demo::serialize_frustum(&body, policy, error, 0.)?;
    data["scope"] = serde_json::json!("checked unplaced positive-radius coaxial rational frustum STEP import; identity frame at the origin, mm units; posed frusta, apex cones and general STEP import remain unsupported");
    data["import"] = serde_json::json!({
        "kind":"nurbs_frustum", "units":"mm", "validated_geometry":true,
        "source_geometry_preserved":true, "pcurve_representation_verified":true,
        "scope":"unplaced identity-frame positive-radius circular frustum, eight vertices, twelve edges and six faces"
    });
    Ok(data.to_string())
}
