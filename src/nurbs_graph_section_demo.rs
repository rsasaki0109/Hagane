//! Exact scoped source-vertical section queries shared by native and WASM.
use crate::*;
#[allow(clippy::too_many_arguments)]
pub fn nurbs_graph_section_demo_json(
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
    u: f64,
    v: f64,
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
    let section = graph.vertical_section(
        [u, v],
        GeometryTolerance::new(linear, GeometryTolerance::default().angular(), 0.)?,
    )?;
    section_json(section, linear)
}
#[allow(clippy::too_many_arguments)]
pub fn nurbs_graph_hole_section_demo_json(
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
    u: f64,
    v: f64,
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
    let section = graph.vertical_section(
        [u, v],
        GeometryTolerance::new(linear, GeometryTolerance::default().angular(), 0.)?,
    )?;
    section_json(section, linear)
}
fn section_json(section: NurbsGraphVerticalSection, linear: f64) -> Result<String> {
    let xyz = |p: Point3| [p.x, p.y, p.z];
    let segments = section
        .segments
        .iter()
        .map(|curve| match curve {
            Curve::Line { a, b } => Ok(serde_json::json!({"a":xyz(*a),"b":xyz(*b)})),
            _ => Err(Error::InvalidTopology(
                "source-vertical section requires line segments",
            )),
        })
        .collect::<Result<Vec<_>>>()?;
    let events=section.events.iter().map(|e|serde_json::json!({"parameter":e.parameter,"point":xyz(e.point),"normal":[e.normal.x,e.normal.y,e.normal.z],"face":e.face,"entering":e.entering})).collect::<Vec<_>>();
    Ok(serde_json::json!({"source_uv":section.uv,"line":{"origin":xyz(section.origin),"direction":[section.direction.x,section.direction.y,section.direction.z]},"intervals":section.intervals,"length":section.intervals.iter().map(|r|r[1]-r[0]).sum::<f64>(),"events":events,"segments":segments,"linear_tolerance":linear,"relative_tolerance":0.,"scope":"source-vertical material section; general line intersections remain unsupported"}).to_string())
}
