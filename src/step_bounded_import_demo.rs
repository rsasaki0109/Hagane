//! Checked bounded analytic STEP import, rendering the actual imported B-rep.
use crate::*;
pub fn import_step_bounded_analytic_json(input: &str) -> Result<String> {
    let tolerance = Tolerance::new(1e-6)?;
    let solid = import_step_bounded_analytic_mm(input, tolerance)?;
    solid.validate(tolerance)?;
    let display = crate::edge_fillet_demo::solid_json(&solid, 0.1, tolerance)?;
    let step = export_step_bounded_analytic_mm(&solid, tolerance.linear)?;
    Ok(serde_json::json!({"units":"mm","mode":"bounded_analytic","source_format":"AP214","solid":display,"display_chord_tolerance":0.1,"linear_tolerance":tolerance.linear,"step":step,"exact":true,"source_bytes_preserved":false,"scope":"checked bounded line/circular-arc planar/cylindrical STEP subset; actual geometry is retained, canonical re-export may reorder records; general STEP and rational-surface import remain unsupported"}).to_string())
}
