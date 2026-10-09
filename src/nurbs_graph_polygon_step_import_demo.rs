//! Display the actual checked imported polygon B-rep, without rebuilding it.
use crate::*;

pub fn nurbs_graph_polygon_step_import_demo_json(text: &str, error: f64) -> Result<String> {
    let body = import_step_nurbs_graph_polygon_mm(text, Tolerance::default())?;
    let [width, depth, height] = body.source().dimensions();
    let text = crate::nurbs_graph_polygon_demo::serialize_polygon_graph(
        &body,
        width,
        depth,
        height,
        body.source().bulge(),
        error,
    )?;
    let mut data: serde_json::Value = serde_json::from_str(&text)
        .map_err(|_| Error::InvalidInput("polygon STEP import display serialization failed"))?;
    data["placement"] = serde_json::json!({"angle":0.,"translation":[0.,0.,0.],"axis":[0.,1.,0.]});
    data["scope"] = serde_json::json!("checked unplaced full-UV convex polygon STEP import; holes, placement, trimmed sources and general STEP import remain unsupported");
    data["import"] = serde_json::json!({
        "kind":"polygon","units":"mm","validated_geometry":true,
        "scope":"unplaced full-UV strictly convex 3..16-corner polygon graph",
        "source_geometry_preserved":true,"pcurve_representation_verified":true
    });
    Ok(data.to_string())
}
