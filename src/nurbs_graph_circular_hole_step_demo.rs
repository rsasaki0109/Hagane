//! Exact circular-bore STEP export without display tessellation.
use crate::*;

pub fn nurbs_graph_circular_hole_step_demo_json(x: &[f64]) -> Result<String> {
    if x.len() != 16 || x.iter().any(|v| !v.is_finite()) {
        return Err(Error::InvalidInput(
            "circular hole STEP needs exactly 16 finite values",
        ));
    }
    let tol = Tolerance::default();
    let source = crate::nurbs_graph_classification_demo::query_source(
        [x[0], x[1], x[2]],
        x[3],
        x[4],
        x[5],
        [x[6], x[7], x[8]],
        [[x[9], x[10]], [x[11], x[12]]],
    )?;
    let body = source.through_xy_circle([x[13], x[14]], x[15], tol)?;
    let step = body.export_step_mm(tol)?;
    let bounds = body.bounds()?;
    let xyz = |p: Point3| [p.x, p.y, p.z];
    Ok(serde_json::json!({
        "step":step,"units":"mm","schema":"AUTOMOTIVE_DESIGN","application_protocol":"AP214","exact":true,
        "import_supported":false,"genus":1,"center":body.center(),"radius":body.radius(),
        "volume":body.volume()?,"removed_volume":body.removed_volume(tol)?,
        "bounds":{"min":xyz(bounds.min),"max":xyz(bounds.max)},"bounds_kind":"conservative control hull bounds",
        "scope":"one exact circular through bore in checked rectangular graph stock; STEP export only, circular-bore STEP import and generic Booleans remain unsupported"
    }).to_string())
}
