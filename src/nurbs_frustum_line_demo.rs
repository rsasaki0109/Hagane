//! Infinite-line and half-ray queries on the actual typed rational frustum.
use crate::*;

/// Model/display values, world origin XYZ, direction XYZ, then mode 0=line/1=ray.
pub fn nurbs_frustum_line_demo_json(x: &[f64]) -> Result<String> {
    if x.len() != 16 || x.iter().any(|v| !v.is_finite()) || (x[15] != 0. && x[15] != 1.) {
        return Err(Error::InvalidInput(
            "frustum line demo requires sixteen finite values and mode zero or one",
        ));
    }
    let (body, policy) = crate::nurbs_frustum_demo::query_body(&x[..9])?;
    let origin = Point3::new(x[9], x[10], x[11]);
    let direction = Vec3::new(x[12], x[13], x[14]);
    let result = if x[15] == 0. {
        body.intersect_line(origin, direction, policy)?
    } else {
        body.intersect_ray(origin, direction, policy)?
    };
    let mut data = crate::nurbs_frustum_demo::serialize_frustum(&body, policy, x[8], x[3])?;
    let xyz = |v: Vec3| [v.x, v.y, v.z];
    let frame = body.frame();
    data["placement"] =
        serde_json::json!({"translation":xyz(frame.origin()),"axes":frame.axes().map(xyz)});
    let hits:Vec<_> = result.hits.iter().map(|hit|serde_json::json!({
        "parameter":hit.parameter,"point":xyz(hit.point),
        "faces":hit.faces.iter().map(|face|serde_json::json!({"face_id":face.face_id,"uv":face.uv})).collect::<Vec<_>>()
    })).collect();
    data["line"] = serde_json::json!({"origin":xyz(origin),"direction":xyz(direction),"kind":if x[15]==0. {"line"}else{"ray"},"hits":hits,"material_interval":result.material_interval});
    data["scope"] = serde_json::json!("typed rational frustum infinite line or half-ray query with actual surface witnesses; display is independent of query geometry; tangent and unresolved contacts unsupported; query does not modify the B-rep");
    Ok(data.to_string())
}
