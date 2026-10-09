//! Checked world-point classification of an exact circular-bore solid, without meshing.
use crate::*;
pub fn nurbs_graph_circular_hole_point_demo_json(x: &[f64]) -> Result<String> {
    if x.len() != 20 || x.iter().any(|v| !v.is_finite()) {
        return Err(Error::InvalidInput(
            "circular bore query requires exactly 20 finite model/XYZ/tolerance values",
        ));
    }
    let source = crate::nurbs_graph_classification_demo::query_source(
        [x[0], x[1], x[2]],
        x[3],
        x[4],
        x[5],
        [x[6], x[7], x[8]],
        [[x[9], x[10]], [x[11], x[12]]],
    )?;
    let body = source.through_xy_circle([x[13], x[14]], x[15], Tolerance::default())?;
    let point = Point3::new(x[16], x[17], x[18]);
    let tolerance = GeometryTolerance::new(x[19], GeometryTolerance::default().angular(), 0.)?;
    let location = body.classify_point(point, tolerance)?;
    let mapped = source.placement().inverse()?.point(point);
    if !mapped.finite() {
        return Err(Error::InvalidInput(
            "circular bore query source point is not representable",
        ));
    }
    let (name,reason)=match location{
        PointLocation::Inside=>("Inside","Inside retained material; boundary distance exceeds the checked Euclidean tolerance."),
        PointLocation::Outside=>("Outside","Outside retained material, including the exact circular through bore; boundary distance exceeds the checked Euclidean tolerance."),
        PointLocation::Boundary=>("Boundary","A retained boundary point is within the checked Euclidean tolerance."),
    };
    Ok(serde_json::json!({"location":name,"reason":reason,"point":[point.x,point.y,point.z],"source_point":[mapped.x,mapped.y,mapped.z],"units":"mm","linear_tolerance":x[19],"relative_tolerance":0.,"genus":1,"center":body.center(),"radius":body.radius(),"source_domain":source.source_domain(),"placement":{"angle":x[5],"translation":[x[6],x[7],x[8]],"axis":[0.,1.,0.]},"scope":"one exact circular through bore in rectangular polynomial graph stock; checked Euclidean boundary tolerance"}).to_string())
}
