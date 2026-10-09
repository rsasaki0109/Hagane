//! Scoped graph B-rep STEP export, shared by native and WASM demos.
use crate::*;
#[allow(clippy::too_many_arguments)]
pub fn nurbs_graph_step_demo_json(
    width: f64,
    depth: f64,
    height: f64,
    bulge: f64,
    error: f64,
    angle: f64,
    tx: f64,
    ty: f64,
    tz: f64,
    u0: f64,
    u1: f64,
    v0: f64,
    v1: f64,
) -> Result<String> {
    let graph = crate::nurbs_graph_classification_demo::query_source(
        [width, depth, height],
        bulge,
        error,
        angle,
        [tx, ty, tz],
        [[u0, u1], [v0, v1]],
    )?;
    let step = graph.export_step_mm(Tolerance::default())?;
    Ok(step_json(step))
}
#[allow(clippy::too_many_arguments)]
pub fn nurbs_graph_hole_step_demo_json(
    width: f64,
    depth: f64,
    height: f64,
    bulge: f64,
    error: f64,
    angle: f64,
    tx: f64,
    ty: f64,
    tz: f64,
    u0: f64,
    u1: f64,
    v0: f64,
    v1: f64,
    hu0: f64,
    hu1: f64,
    hv0: f64,
    hv1: f64,
) -> Result<String> {
    let source = crate::nurbs_graph_classification_demo::query_source(
        [width, depth, height],
        bulge,
        error,
        angle,
        [tx, ty, tz],
        [[u0, u1], [v0, v1]],
    )?;
    let graph = NurbsGraphHoledSolid::new(&source, [[hu0, hu1], [hv0, hv1]], Tolerance::default())?;
    let step = graph.export_step_mm(Tolerance::default())?;
    Ok(step_json(step))
}
fn step_json(step: String) -> String {
    serde_json::json!({"step":step,"units":"mm","schema":"AUTOMOTIVE_DESIGN","exact":true,"scope":"canonical polynomial graph B-rep; general NURBS STEP import remains unsupported"}).to_string()
}
