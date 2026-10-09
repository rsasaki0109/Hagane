//! Display only after the strict canonical graph STEP importer succeeds.
use crate::*;
pub fn nurbs_graph_step_import_demo_json(text: &str, error: f64) -> Result<String> {
    let graph = import_step_nurbs_graph_mm(text, Tolerance::default())?;
    let [width, depth, height] = graph.dimensions();
    let data = nurbs_graph_trimmed_demo_json(
        width,
        depth,
        height,
        graph.bulge(),
        error,
        0.,
        0.,
        0.,
        0.,
        0.,
        1.,
        0.,
        1.,
    )?;
    let mut data: serde_json::Value = serde_json::from_str(&data)
        .map_err(|_| Error::InvalidInput("graph import display serialization failed"))?;
    data["import"] = serde_json::json!({"units":"mm","scope":"unplaced full-domain six-face canonical polynomial graph","validated_geometry":true});
    Ok(data.to_string())
}
