//! Checked bounded analytic STEP import, rendering the actual imported B-rep.
use crate::*;
/// Exact rounded stock with one rectangular and one four-arc circular opening.
/// This creates analytic B-rep geometry before exporting it; no mesh Boolean is used.
pub fn bounded_analytic_openings_sample_step() -> Result<String> {
    let tol = Tolerance::new(1e-6)?;
    let outer = rounded_rectangle_profile(Point3::new(0., 0., 0.), 80., 60., 8., tol)?;
    let rectangle = [[-24., -6.], [-16., -6.], [-16., 6.], [-24., 6.]];
    let rectangle = (0..4)
        .map(|i| PlanarSegment::Line {
            a: rectangle[i],
            b: rectangle[(i + 1) % 4],
        })
        .collect();
    let circle = (0..4)
        .map(|i| PlanarSegment::Arc {
            center: [20., 0.],
            radius: 6.,
            start_angle: i as f64 * std::f64::consts::FRAC_PI_2,
            sweep: std::f64::consts::FRAC_PI_2,
        })
        .collect();
    let region = ArcLineRegion {
        origin: outer.origin,
        outer: outer.segments,
        holes: vec![rectangle, circle],
    };
    let solid = extrude_arc_line_region(&region, 20., tol)?;
    export_step_bounded_analytic_mm(&solid, tol.linear)
}

pub fn bounded_analytic_openings_sample_json() -> Result<String> {
    let input_step = bounded_analytic_openings_sample_step()?;
    let report: serde_json::Value =
        serde_json::from_str(&import_step_bounded_analytic_json(&input_step)?).map_err(|_| {
            Error::InvalidInput("bounded openings sample report serialization failed")
        })?;
    Ok(serde_json::json!({"input_step":input_step,"report":report}).to_string())
}

pub fn import_step_bounded_analytic_json(input: &str) -> Result<String> {
    let tolerance = Tolerance::new(1e-6)?;
    let solid = import_step_bounded_analytic_mm(input, tolerance)?;
    solid.validate(tolerance)?;
    let display = crate::edge_fillet_demo::solid_json(&solid, 0.1, tolerance)?;
    let step = export_step_bounded_analytic_mm(&solid, tolerance.linear)?;
    Ok(serde_json::json!({"units":"mm","mode":"bounded_analytic","source_format":"AP214","solid":display,"display_chord_tolerance":0.1,"linear_tolerance":tolerance.linear,"step":step,"exact":true,"source_bytes_preserved":false,"scope":"checked bounded line/circular-arc planar/cylindrical STEP subset; actual geometry is retained, canonical re-export may reorder records; general STEP and rational-surface import remain unsupported"}).to_string())
}
