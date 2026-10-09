//! Checked world-point queries on exact retained multi-opening graph solids.
use crate::*;

/// Model payload followed by world XYZ and a positive absolute linear tolerance.
/// The full finite transport has 26..151 values; no display mesh is requested.
pub fn nurbs_graph_polygon_multi_hole_point_demo_json(values: &[f64]) -> Result<String> {
    if !(26..=151).contains(&values.len()) || values.iter().any(|value| !value.is_finite()) {
        return Err(Error::InvalidInput(
            "multiple polygon opening query requires 26..151 finite model/XYZ/tolerance values",
        ));
    }
    let model_len = values.len() - 4;
    let body =
        crate::nurbs_graph_polygon_multi_hole_demo::polygon_multi_hole_model(&values[..model_len])?;
    let tail = &values[model_len..];
    let point = Point3::new(tail[0], tail[1], tail[2]);
    let tolerance = GeometryTolerance::new(tail[3], GeometryTolerance::default().angular(), 0.)?;
    let location = body.classify_point(point, tolerance)?;
    let mapped = body.source().placement().inverse()?.point(point);
    if !mapped.finite() {
        return Err(Error::InvalidInput(
            "query source point is not representable",
        ));
    }
    let(name,reason)=match location{
        PointLocation::Inside=>("Inside","Inside retained material; boundary distance exceeds the checked Euclidean tolerance."),
        PointLocation::Outside=>("Outside","Outside retained material, including any through opening; boundary distance exceeds the checked Euclidean tolerance."),
        PointLocation::Boundary=>("Boundary","A retained boundary point is within the checked Euclidean tolerance."),
    };
    Ok(serde_json::json!({"location":name,"reason":reason,"point":[point.x,point.y,point.z],"source_point":[mapped.x,mapped.y,mapped.z],"linear_tolerance":tail[3],"relative_tolerance":0.,"genus":body.genus(),"scope":"convex polynomial graph solid with 1..4 disjoint strictly convex polygon through openings; checked Euclidean boundary tolerance"}).to_string())
}
