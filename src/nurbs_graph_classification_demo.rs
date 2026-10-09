//! Scoped graph-solid queries shared by native and WASM demonstrations.
use crate::*;
#[allow(clippy::too_many_arguments)]
pub fn nurbs_graph_point_demo_json(
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
    x: f64,
    y: f64,
    z: f64,
    linear: f64,
) -> Result<String> {
    let source = query_source(
        [width, depth, height],
        bulge,
        error,
        angle,
        [tx, ty, tz],
        [[u0, u1], [v0, v1]],
    )?;
    let point = Point3::new(x, y, z);
    let tol = GeometryTolerance::new(linear, GeometryTolerance::default().angular(), 0.)?;
    let location = source.classify_point(point, tol)?;
    Ok(query_json(
        location,
        point,
        linear,
        "polynomial graph solid",
    ))
}
#[allow(clippy::too_many_arguments)]
pub fn nurbs_graph_hole_point_demo_json(
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
    x: f64,
    y: f64,
    z: f64,
    linear: f64,
) -> Result<String> {
    let source = query_source(
        [width, depth, height],
        bulge,
        error,
        angle,
        [tx, ty, tz],
        [[u0, u1], [v0, v1]],
    )?;
    let graph = NurbsGraphHoledSolid::new(&source, [[hu0, hu1], [hv0, hv1]], Tolerance::default())?;
    let point = Point3::new(x, y, z);
    let tol = GeometryTolerance::new(linear, GeometryTolerance::default().angular(), 0.)?;
    let location = graph.classify_point(point, tol)?;
    Ok(query_json(
        location,
        point,
        linear,
        "polynomial graph solid with rectangular through hole",
    ))
}
fn query_source(
    dimensions: [f64; 3],
    bulge: f64,
    error: f64,
    angle: f64,
    translation: [f64; 3],
    domain: [[f64; 2]; 2],
) -> Result<NurbsGraphSolid> {
    if !error.is_finite() || error <= 0. {
        return Err(Error::InvalidInput(
            "graph query display error must be positive",
        ));
    }
    let transform =
        Transform::translation(Vec3::new(translation[0], translation[1], translation[2]))?
            .compose(Transform::rotation(Vec3::new(0., 1., 0.), angle)?)?;
    NurbsGraphSolid::new(dimensions, bulge, Tolerance::default())?
        .trimmed_uv(domain, Tolerance::default())?
        .transformed(transform, Tolerance::default())
}
fn query_json(location: PointLocation, point: Point3, linear: f64, scope: &str) -> String {
    let (name,reason)=match location {
        PointLocation::Inside=>("Inside","Inside the retained material; boundary distance exceeds the Euclidean tolerance."),
        PointLocation::Outside=>("Outside","Outside the retained material, including any through opening; boundary distance exceeds the Euclidean tolerance."),
        PointLocation::Boundary=>("Boundary","A retained boundary point is within the checked Euclidean tolerance."),
    };
    serde_json::json!({"location":name,"reason":reason,"point":[point.x,point.y,point.z],"linear_tolerance":linear,"relative_tolerance":0.,"scope":scope}).to_string()
}
