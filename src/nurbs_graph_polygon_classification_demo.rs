//! Typed convex polygon graph point queries shared by native and WASM demos.
use crate::*;
pub fn nurbs_graph_polygon_point_demo_json(values: &[f64]) -> Result<String> {
    if !(23..=49).contains(&values.len())
        || values.len().is_multiple_of(2)
        || values.iter().any(|v| !v.is_finite())
    {
        return Err(Error::InvalidInput(
            "polygon point query requires a valid model payload and world XYZ/linear tolerance",
        ));
    }
    let model = &values[..values.len() - 4];
    let source = query_source(model)?;
    let graph = NurbsGraphPolygonSolid::new(
        &source,
        model[13..].as_chunks::<2>().0.to_vec(),
        Tolerance::default(),
    )?;
    query(
        &source,
        values,
        |point, tol| graph.classify_point(point, tol),
        "convex polygon polynomial graph solid",
    )
}
pub fn nurbs_graph_polygon_hole_point_demo_json(values: &[f64]) -> Result<String> {
    if !(25..=83).contains(&values.len())
        || values.len().is_multiple_of(2)
        || values.iter().any(|v| !v.is_finite())
    {
        return Err(Error::InvalidInput(
            "polygon opening query requires a valid model payload and world XYZ/linear tolerance",
        ));
    }
    let model = &values[..values.len() - 4];
    let count = |v: f64, outer: bool| -> Result<usize> {
        if v.fract() != 0. || !((3. ..=16.).contains(&v) || (outer && v == 0.)) {
            return Err(Error::InvalidInput(
                "polygon query counts must be 3..16, or outer zero",
            ));
        }
        Ok(v as usize)
    };
    let no = count(model[13], true)?;
    let ni = count(model[14], false)?;
    if model.len() != 15 + 2 * (no + ni) {
        return Err(Error::InvalidInput(
            "polygon query count does not match UV pairs",
        ));
    }
    let source = query_source(model)?;
    let outer = if no == 0 {
        vec![
            [model[9], model[11]],
            [model[10], model[11]],
            [model[10], model[12]],
            [model[9], model[12]],
        ]
    } else {
        model[15..15 + 2 * no].as_chunks::<2>().0.to_vec()
    };
    let outer = NurbsGraphPolygonSolid::new(&source, outer, Tolerance::default())?;
    let graph = NurbsGraphPolygonHoledSolid::new(
        &outer,
        model[15 + 2 * no..].as_chunks::<2>().0.to_vec(),
        Tolerance::default(),
    )?;
    query(
        &source,
        values,
        |point, tol| graph.classify_point(point, tol),
        "convex polygon graph solid with one convex through opening",
    )
}
fn query_source(x: &[f64]) -> Result<NurbsGraphSolid> {
    crate::nurbs_graph_classification_demo::query_source(
        [x[0], x[1], x[2]],
        x[3],
        x[4],
        x[5],
        [x[6], x[7], x[8]],
        [[x[9], x[10]], [x[11], x[12]]],
    )
}
fn query(
    source: &NurbsGraphSolid,
    values: &[f64],
    classify: impl FnOnce(Point3, GeometryTolerance) -> Result<PointLocation>,
    scope: &str,
) -> Result<String> {
    let tail = &values[values.len() - 4..];
    let point = Point3::new(tail[0], tail[1], tail[2]);
    let tol = GeometryTolerance::new(tail[3], GeometryTolerance::default().angular(), 0.)?;
    let location = classify(point, tol)?;
    let mapped = source.placement().inverse()?.point(point);
    if !mapped.finite() {
        return Err(Error::InvalidInput(
            "query source point is not representable",
        ));
    }
    let(name,reason)=match location {
        PointLocation::Inside=>("Inside","Inside retained material; boundary distance exceeds the checked Euclidean tolerance."),
        PointLocation::Outside=>("Outside","Outside retained material, including the opening; boundary distance exceeds the checked Euclidean tolerance."),
        PointLocation::Boundary=>("Boundary","A retained boundary point is within the checked Euclidean tolerance."),
    };
    Ok(serde_json::json!({"location":name,"reason":reason,"point":[point.x,point.y,point.z],"source_point":[mapped.x,mapped.y,mapped.z],"linear_tolerance":tail[3],"relative_tolerance":0.,"scope":scope}).to_string())
}
