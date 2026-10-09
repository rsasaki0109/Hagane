//! Display only after the strict canonical graph STEP importer succeeds.
use crate::*;
pub fn nurbs_graph_step_import_demo_json(text: &str, error: f64) -> Result<String> {
    let graph = import_step_nurbs_graph_mm(text, Tolerance::default())?;
    render_plain_graph_import(&graph, error)
}

fn render_plain_graph_import(graph: &NurbsGraphSolid, error: f64) -> Result<String> {
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

/// Display only after all canonical through-hole geometry has been checked.
pub fn nurbs_graph_holed_step_import_demo_json(text: &str, error: f64) -> Result<String> {
    let graph = import_step_nurbs_graph_holed_mm(text, Tolerance::default())?;
    render_holed_graph_import(&graph, error)
}

fn render_holed_graph_import(graph: &NurbsGraphHoledSolid, error: f64) -> Result<String> {
    let [width, depth, height] = graph.source().dimensions();
    let [[hu0, hu1], [hv0, hv1]] = graph.hole();
    let data = nurbs_graph_hole_demo_json(
        width,
        depth,
        height,
        graph.source().bulge(),
        error,
        0.,
        0.,
        0.,
        0.,
        0.,
        1.,
        0.,
        1.,
        hu0,
        hu1,
        hv0,
        hv1,
    )?;
    let mut data: serde_json::Value = serde_json::from_str(&data)
        .map_err(|_| Error::InvalidInput("holed graph import display serialization failed"))?;
    data["import"] = serde_json::json!({"units":"mm", "scope":"unplaced full-domain polynomial graph with one exact rectangular through hole", "validated_geometry":true});
    Ok(data.to_string())
}

/// Parse one supported STEP body once, then display its checked graph-family geometry.
pub fn nurbs_graph_step_auto_import_demo_json(text: &str, error: f64) -> Result<String> {
    let body = import_step_nurbs_graph_auto_mm(text, Tolerance::default())?;
    let (kind, data) = match body {
        ImportedNurbsGraph::Plain(graph) => ("plain", render_plain_graph_import(&graph, error)?),
        ImportedNurbsGraph::Holed(graph) => ("holed", render_holed_graph_import(&graph, error)?),
    };
    let mut data: serde_json::Value = serde_json::from_str(&data)
        .map_err(|_| Error::InvalidInput("graph auto import display serialization failed"))?;
    data["import"]["kind"] = serde_json::json!(kind);
    Ok(data.to_string())
}
