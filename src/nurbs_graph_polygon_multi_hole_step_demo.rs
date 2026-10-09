//! Exact retained multi-opening graph B-rep export; no display mesh is generated.
use crate::*;

/// The checked model transport is shared with display, but display tolerance
/// only needs to be finite and positive; it does not control exact STEP export.
pub fn nurbs_graph_polygon_multi_hole_step_demo_json(values: &[f64]) -> Result<String> {
    let body = crate::nurbs_graph_polygon_multi_hole_demo::polygon_multi_hole_model(values)?;
    let step = body.export_step_mm(Tolerance::default())?;
    Ok(serde_json::json!({
        "step":step,"units":"mm","schema":"AUTOMOTIVE_DESIGN","exact":true,
        "scope":"1..4 disjoint strictly convex polygon through openings in convex graph B-rep; exact export only, polygon STEP import and generic Booleans remain unsupported",
        "import_supported":false,"genus":body.genus(),"outer_polygon":body.outer_polygon(),"openings":body.openings()
    }).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn corrupted_retained_model_never_exports_cached_geometry() {
        let values = vec![
            80., 60., 20., 30., 0.5, 0., 0., 0., 0., 0., 1., 0., 1., 0., 2., 4., 0.2, 0.5, 0.3,
            0.4, 0.4, 0.5, 0.3, 0.6, 4., 0.6, 0.5, 0.7, 0.4, 0.8, 0.5, 0.7, 0.6,
        ];
        let mut body =
            crate::nurbs_graph_polygon_multi_hole_demo::polygon_multi_hole_model(&values).unwrap();
        body.solid.vertices[0].point.x += 0.001;
        assert!(body.export_step_mm(Tolerance::default()).is_err());
    }
}
