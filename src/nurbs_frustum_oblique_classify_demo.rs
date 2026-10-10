//! Point classification of both actual closed oblique frustum children.
use crate::*;

/// Sixteen split model/display values followed by the world query point XYZ.
pub fn nurbs_frustum_oblique_classify_demo_json(x: &[f64]) -> Result<String> {
    if x.len() != 19 || x.iter().any(|v| !v.is_finite()) || (x[15] != 0. && x[15] != 1.) {
        return Err(Error::InvalidInput(
            "oblique frustum point query requires nineteen finite values and selection zero or one",
        ));
    }
    let (source, policy) = crate::nurbs_frustum_demo::query_body(&x[..9])?;
    let origin = Point3::new(x[9], x[10], x[11]);
    let normal = Vec3::new(x[12], x[13], x[14]).normalized()?;
    let reference = if normal.x.abs() < 0.9 {
        Vec3::new(1., 0., 0.)
    } else {
        Vec3::new(0., 1., 0.)
    };
    let u = reference.cross(normal).normalized()?;
    let v = normal.cross(u).normalized()?;
    let split = source.split_by_plane(&Surface::Plane { origin, u, v }, policy)?;
    let point = Point3::new(x[16], x[17], x[18]);
    let label = |location: PointLocation| match location {
        PointLocation::Inside => "inside",
        PointLocation::Outside => "outside",
        PointLocation::Boundary => "boundary",
    };
    let classes = [
        label(split.lower.classify_point(point, policy)?),
        label(split.upper.classify_point(point, policy)?),
    ];
    let mut data: serde_json::Value =
        serde_json::from_str(&nurbs_frustum_plane_split_demo_json(&x[..16])?).map_err(|_| {
            Error::InvalidInput("oblique frustum point report serialization failed")
        })?;
    data["point_query"] = serde_json::json!({"point":[point.x,point.y,point.z],"classes":classes,"mesh_free":true,"scope":"Euclidean boundary-band classification of both actual closed children; unresolved distances return an error"});
    Ok(data.to_string())
}
