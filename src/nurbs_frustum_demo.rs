//! Native/WASM rendering and exact STEP for checked rational frustum B-reps.
use crate::*;

/// Lower/upper radius, height, Y pose angle, translation XYZ, linear/chord tolerances.
pub fn nurbs_frustum_demo_json(x: &[f64]) -> Result<String> {
    if x.len() != 9 || x.iter().any(|v| !v.is_finite()) {
        return Err(Error::InvalidInput(
            "NURBS frustum requires exactly nine finite values",
        ));
    }
    let (body, policy) = query_body(x)?;
    Ok(serialize_frustum(&body, policy, x[8], x[3])?.to_string())
}

pub(crate) fn serialize_frustum(
    body: &NurbsFrustumSolid,
    policy: GeometryTolerance,
    chord_error: f64,
    angle: f64,
) -> Result<serde_json::Value> {
    let frame = body.frame();
    let origin = frame.origin();
    let mass = body.mass_properties(policy)?;
    let inertia = body.inertia_properties(policy)?;
    let bounds = body.bounds(policy)?;
    let mesh = body.tessellate(chord_error, policy)?;
    let xyz = |p: Point3| [p.x, p.y, p.z];
    Ok(serde_json::json!({
        "units":"mm","radii":body.radii(),"height":body.height(),"volume":mass.volume,"centroid":xyz(mass.centroid),"inertia":inertia.inertia,
        "bounds":{"min":xyz(bounds.min),"max":xyz(bounds.max)},"bounds_kind":"exact endpoint-circle axis envelopes",
        "placement":{"angle":angle,"translation":[origin.x,origin.y,origin.z],"axis":[0.,1.,0.]},"axis":[frame.axes()[2].x,frame.axes()[2].y,frame.axes()[2].z],
        "linear_tolerance":policy.linear(),"display_chord_tolerance":chord_error,
        "mesh":{"positions":mesh.positions.iter().map(|p|xyz(*p)).collect::<Vec<_>>(),"normals":mesh.normals.iter().map(|n|[n.x,n.y,n.z]).collect::<Vec<_>>(),"triangles":mesh.triangles,"face_ids":mesh.face_ids},
        "brep":crate::nurbs_graph_polygon_demo::serialize_graph_brep(body.solid())?,
        "step":body.export_step_mm(policy)?,"step_exact":true,"step_schema":"AUTOMOTIVE_DESIGN",
        "scope":"typed positive-radius coaxial circular frustum; four exact rational NURBS ruled sides, rational rims, plane caps and line generators; apex, arbitrary lofts, general Booleans and frustum STEP import unsupported"
    }))
}

pub(crate) fn query_body(x: &[f64]) -> Result<(NurbsFrustumSolid, GeometryTolerance)> {
    let policy = GeometryTolerance::new(x[7], GeometryTolerance::default().angular(), 0.)?;
    let pose = Transform::translation(Vec3::new(x[4], x[5], x[6]))?
        .compose(Transform::rotation(Vec3::new(0., 1., 0.), x[3])?)?;
    let frame = Frame3::new_with_tolerance(
        Point3::new(x[4], x[5], x[6]),
        [
            pose.vector(Vec3::new(1., 0., 0.)),
            pose.vector(Vec3::new(0., 1., 0.)),
            pose.vector(Vec3::new(0., 0., 1.)),
        ],
        policy,
    )?;
    let body = NurbsFrustumSolid::new(frame, [x[0], x[1]], x[2], policy)?;
    Ok((body, policy))
}

/// Mesh-free world-point query: nine model/display values, then world XYZ.
pub fn nurbs_frustum_classification_demo_json(x: &[f64]) -> Result<String> {
    if x.len() != 12 || x.iter().any(|v| !v.is_finite()) {
        return Err(Error::InvalidInput(
            "frustum point query requires exactly twelve finite values",
        ));
    }
    let (body, policy) = query_body(x)?;
    let point = Point3::new(x[9], x[10], x[11]);
    let location = match body.classify_point(point, policy)? {
        PointLocation::Inside => "inside",
        PointLocation::Outside => "outside",
        PointLocation::Boundary => "boundary",
    };
    Ok(serde_json::json!({"location":location,"point":[x[9],x[10],x[11]],"radii":body.radii(),"height":body.height(),"units":"mm","linear_tolerance":x[7],"placement":{"angle":x[3],"translation":[x[4],x[5],x[6]],"axis":[0.,1.,0.]},"scope":"validated finite rational frustum; Euclidean boundary band, mesh-free query"}).to_string())
}
