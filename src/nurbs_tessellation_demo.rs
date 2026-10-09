//! Interactive bounded NURBS polyline fixture; standalone geometry, not a B-rep.
use crate::*;

/// Same exact rational quadratic as `nurbs_demo_json`, with bounded adaptive display.
pub fn nurbs_tessellation_demo_json(
    weight: f64,
    parameter: f64,
    chord_error: f64,
) -> Result<String> {
    let curve = NurbsCurve::new(
        2,
        vec![0., 0., 0., 1., 1., 1.],
        vec![
            Point3::new(1., 0., 0.),
            Point3::new(1., 1., 0.),
            Point3::new(0., 1., 0.),
        ],
        vec![1., weight, 1.],
    )?;
    let (point, tangent) = curve.evaluate_with_derivative(parameter, KnotSide::Right)?;
    let refined = curve.insert_knot(0.35, 1)?.insert_knot(0.7, 2)?;
    let span_ranges: Vec<_> = refined
        .bezier_spans()?
        .iter()
        .map(|span| span.parameter_range)
        .collect();
    let refined_controls: Vec<_> = refined
        .control_points()
        .iter()
        .flat_map(|p| [p.x, p.y, p.z])
        .collect();
    let polyline = refined.tessellate_bounded(chord_error, 16384)?;
    let samples: Vec<_> = polyline
        .points
        .iter()
        .flat_map(|p| [p.x, p.y, p.z])
        .collect();
    Ok(serde_json::json!({
        "degree": 2, "weight": weight, "parameter": parameter,
        "point": [point.x,point.y,point.z], "tangent": [tangent.x,tangent.y,tangent.z],
        "samples": samples, "parameters": polyline.parameters,
        "error_bounds": polyline.error_bounds, "requested_error": chord_error,
        "segments": polyline.points.len()-1,
        "inserted_knots": [0.35,0.7,0.7], "control_count": refined.control_points().len(),
        "refined_controls": refined_controls, "refined_weights": refined.weights(),
        "span_ranges": span_ranges,
    })
    .to_string())
}
