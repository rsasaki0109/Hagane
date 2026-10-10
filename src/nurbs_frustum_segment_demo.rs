//! Segment query and display of one actual typed rational frustum.
use crate::*;

/// Nine frustum model/display values followed by world start XYZ and end XYZ.
pub fn nurbs_frustum_segment_demo_json(x: &[f64]) -> Result<String> {
    if x.len() != 15 || x.iter().any(|v| !v.is_finite()) {
        return Err(Error::InvalidInput(
            "frustum segment demo requires exactly fifteen finite values",
        ));
    }
    let (body, policy) = crate::nurbs_frustum_demo::query_body(&x[..9])?;
    let start = Point3::new(x[9], x[10], x[11]);
    let end = Point3::new(x[12], x[13], x[14]);
    let result = body.intersect_segment(start, end, policy)?;
    let mut data = crate::nurbs_frustum_demo::serialize_frustum(&body, policy, x[8], x[3])?;
    let xyz = |v: Vec3| [v.x, v.y, v.z];
    let frame = body.frame();
    data["placement"] =
        serde_json::json!({"translation":xyz(frame.origin()),"axes":frame.axes().map(xyz)});
    let hits:Vec<_> = result.hits.iter().map(|hit|serde_json::json!({
        "parameter":hit.parameter,"point":xyz(hit.point),
        "faces":hit.faces.iter().map(|face|serde_json::json!({"face_id":face.face_id,"uv":face.uv})).collect::<Vec<_>>()
    })).collect();
    data["segment"] = serde_json::json!({"start":xyz(start),"end":xyz(end),"hits":hits,"material_interval":result.material_interval});
    data["scope"] = serde_json::json!("typed rational frustum finite segment query with actual surface witnesses; display is independent of intersection geometry; tangent, boundary-endpoint and unresolved contacts unsupported; query does not modify the B-rep");
    Ok(data.to_string())
}
