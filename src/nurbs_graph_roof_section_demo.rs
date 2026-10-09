//! Exact scoped affine source-UV roof section queries shared by native and WASM.
use crate::*;
#[allow(clippy::too_many_arguments)]
pub fn nurbs_graph_roof_section_demo_json(
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
    start_u: f64,
    start_v: f64,
    end_u: f64,
    end_v: f64,
    linear: f64,
) -> Result<String> {
    let graph = crate::nurbs_graph_classification_demo::query_source(
        [width, depth, height],
        bulge,
        error,
        angle,
        [tx, ty, tz],
        [[u0, u1], [v0, v1]],
    )?;
    let section = graph.roof_section(
        [start_u, start_v],
        [end_u, end_v],
        GeometryTolerance::new(linear, GeometryTolerance::default().angular(), 0.)?,
    )?;
    section_json(section, linear, error)
}
#[allow(clippy::too_many_arguments)]
pub fn nurbs_graph_hole_roof_section_demo_json(
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
    start_u: f64,
    start_v: f64,
    end_u: f64,
    end_v: f64,
    linear: f64,
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
    let section = graph.roof_section(
        [start_u, start_v],
        [end_u, end_v],
        GeometryTolerance::new(linear, GeometryTolerance::default().angular(), 0.)?,
    )?;
    section_json(section, linear, error)
}

fn section_json(section: NurbsGraphRoofSection, linear: f64, error: f64) -> Result<String> {
    let xyz = |p: Point3| [p.x, p.y, p.z];
    let spans = section.spans.iter().map(|span| {
        let PCurve::Affine { origin, direction } = span.pcurve else {
            return Err(Error::InvalidTopology("graph roof section requires affine source pcurves"));
        };
        let display = span.curve.tessellate_bounded(error, 65536)?;
        Ok(serde_json::json!({"face":span.face,"parameter_range":span.parameter_range,
            "curve":{"degree":span.curve.degree(),"knots":span.curve.knots(),"weights":span.curve.weights(),"control_points":span.curve.control_points().iter().map(|p|xyz(*p)).collect::<Vec<_>>()},
            "pcurve":{"origin":origin,"direction":direction},
            "polyline":{"positions":display.points.iter().map(|p|xyz(*p)).collect::<Vec<_>>(),"parameters":display.parameters,"error_bounds":display.error_bounds}}))
    }).collect::<Result<Vec<_>>>()?;
    Ok(serde_json::json!({"start":section.start,"end":section.end,"spans":spans,"linear_tolerance":linear,"relative_tolerance":0.,"display_error":error,"scope":"finite affine source-UV roof intersection curves; walls, closed section profiles and general solid slicing remain unsupported"}).to_string())
}
