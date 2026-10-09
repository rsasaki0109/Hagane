//! Exact retained convex graph B-rep STEP exports and checked numeric model builders.
use crate::*;
pub fn nurbs_graph_polygon_step_demo_json(values: &[f64]) -> Result<String> {
    let body = polygon_model(values)?;
    Ok(step_json(
        body.export_step_mm(Tolerance::default())?,
        "convex polygon graph B-rep export only; polygon STEP import remains unsupported",
    ))
}
pub fn nurbs_graph_polygon_hole_step_demo_json(values: &[f64]) -> Result<String> {
    let body = polygon_hole_model(values)?;
    Ok(step_json(body.export_step_mm(Tolerance::default())?,"convex polygon graph B-rep export only with one convex through opening; polygon STEP import remains unsupported"))
}
fn step_json(step: String, scope: &str) -> String {
    serde_json::json!({"step":step,"units":"mm","schema":"AUTOMOTIVE_DESIGN","exact":true,"scope":scope,"import_supported":false}).to_string()
}
pub(crate) fn polygon_model(values: &[f64]) -> Result<NurbsGraphPolygonSolid> {
    if !(19..=45).contains(&values.len())
        || values.len().is_multiple_of(2)
        || values.iter().any(|v| !v.is_finite())
    {
        return Err(Error::InvalidInput(
            "polygon model requires 13 finite model values and 3..16 UV pairs",
        ));
    }
    NurbsGraphPolygonSolid::new(
        &model_source(values)?,
        values[13..].as_chunks::<2>().0.to_vec(),
        Tolerance::default(),
    )
}
pub(crate) fn polygon_hole_model(values: &[f64]) -> Result<NurbsGraphPolygonHoledSolid> {
    if !(21..=79).contains(&values.len())
        || values.len().is_multiple_of(2)
        || values.iter().any(|v| !v.is_finite())
    {
        return Err(Error::InvalidInput(
            "polygon opening model requires 21..79 finite values",
        ));
    }
    let count = |v: f64, outer: bool| -> Result<usize> {
        if v.fract() != 0. || !((3. ..=16.).contains(&v) || (outer && v == 0.)) {
            return Err(Error::InvalidInput(
                "polygon counts must be integral 3..16, or outer zero",
            ));
        }
        Ok(v as usize)
    };
    let no = count(values[13], true)?;
    let ni = count(values[14], false)?;
    if values.len() != 15 + 2 * (no + ni) {
        return Err(Error::InvalidInput(
            "polygon opening count does not match UV pairs",
        ));
    }
    let source = model_source(values)?;
    let polygon = if no == 0 {
        vec![
            [values[9], values[11]],
            [values[10], values[11]],
            [values[10], values[12]],
            [values[9], values[12]],
        ]
    } else {
        values[15..15 + 2 * no].as_chunks::<2>().0.to_vec()
    };
    let outer = NurbsGraphPolygonSolid::new(&source, polygon, Tolerance::default())?;
    NurbsGraphPolygonHoledSolid::new(
        &outer,
        values[15 + 2 * no..].as_chunks::<2>().0.to_vec(),
        Tolerance::default(),
    )
}
fn model_source(values: &[f64]) -> Result<NurbsGraphSolid> {
    crate::nurbs_graph_classification_demo::query_source(
        [values[0], values[1], values[2]],
        values[3],
        values[4],
        values[5],
        [values[6], values[7], values[8]],
        [[values[9], values[10]], [values[11], values[12]]],
    )
}
