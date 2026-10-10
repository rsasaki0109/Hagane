//! Actual retained cardinal coaxially bored STEP body reports.
use crate::*;
pub fn nurbs_frustum_bored_step_import_demo_json(text: &str, chord: f64) -> Result<String> {
    let policy = GeometryTolerance::default();
    let body = import_step_nurbs_bored_frustum_cardinal_mm(text, policy)?;
    let mut data = crate::nurbs_frustum_bore_demo::serialize_bored_frustum(&body, policy, chord)?;
    data["scope"]=serde_json::json!("checked signed-cardinal-frame coaxially bored positive-radius rational frustum STEP import in mm; arbitrary rotations, off-axis or blind openings and general STEP import unsupported");
    data["import"] = serde_json::json!({"kind":"nurbs_bored_frustum_cardinal","units":"mm","validated_geometry":true,"source_geometry_preserved":true,"pcurve_representation_verified":true,"scope":"one canonical coaxial through opening in a positive-radius frustum with one of 24 exact right-handed signed cardinal bases and checked representable translation; sixteen vertices, twenty-four edges and ten faces"});
    Ok(data.to_string())
}
