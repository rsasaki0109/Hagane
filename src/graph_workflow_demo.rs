//! Render accepted workflow geometry directly from the retained typed shape.
use crate::*;
use serde_json::Value;

/// Match the native and WASM output protocol, including diagnostic reports.
pub fn graph_workflow_output(result: Result<String>) -> (i32, String) {
    match result {
        Ok(text) => {
            let ok = serde_json::from_str::<Value>(&text)
                .ok()
                .and_then(|v| v["ok"].as_bool())
                .unwrap_or(false);
            (if ok { 0 } else { 1 }, text)
        }
        Err(error) => (
            1,
            serde_json::json!({"ok":false,"error":error.to_string()}).to_string(),
        ),
    }
}

pub(crate) fn serialize_graph_workflow_shape(
    shape: &GraphWorkflowShape,
    document: &GraphWorkflowDocument,
) -> Result<Value> {
    let tol = Tolerance::new(document.tolerance.linear)?;
    shape.validate(tol)?;
    let error = document.display.max_error;
    let display = shape.tessellate_bounded(error, document.display.max_triangles, tol)?;
    let mass = shape.mass_properties(tol)?;
    let (inertia_properties, inertia_error) =
        crate::nurbs_graph_polygon_demo::serialize_polygon_inertia(shape.inertia_properties(tol));
    let bounds = shape.bounds()?;
    let xyz = |p: Point3| [p.x, p.y, p.z];
    let positions: Vec<_> = display
        .mesh
        .triangles
        .iter()
        .flat_map(|t| t.iter().flat_map(|id| xyz(display.mesh.positions[*id])))
        .collect();
    let normals: Vec<_> = display
        .mesh
        .triangles
        .iter()
        .flat_map(|t| {
            t.iter().flat_map(|id| {
                let n = display.mesh.normals[*id];
                [n.x, n.y, n.z]
            })
        })
        .collect();
    let boundary_samples = shape
        .brep()
        .edges
        .iter()
        .map(|edge| match &edge.curve {
            Curve::Line { a, b } => Ok(vec![a.x, a.y, a.z, b.x, b.y, b.z]),
            Curve::Nurbs(c) => Ok(c
                .tessellate_bounded(error, 65536)?
                .points
                .iter()
                .flat_map(|p| xyz(*p))
                .collect()),
            _ => Err(Error::Unsupported(
                "graph workflow boundary curve unsupported",
            )),
        })
        .collect::<Result<Vec<_>>>()?;
    let source = shape.source();
    let [width, depth, height] = source.dimensions();
    let (kind, circle) = match shape {
        GraphWorkflowShape::Plain(_) => ("plain", Value::Null),
        GraphWorkflowShape::Circular(body) => (
            "circular",
            serde_json::json!({"center":body.center(),"radius":body.radius(),"removed_volume":body.removed_volume(tol)?}),
        ),
    };
    Ok(serde_json::json!({
        "kind":kind,"circle":circle,"width":width,"depth":depth,"height":height,"bulge":source.bulge(),"source_domain":source.source_domain(),"error":error,"units":"mm",
        "volume":mass.volume,"mass_properties":{"volume":mass.volume,"centroid":xyz(mass.centroid),"units":"mm","volume_units":"mm3","centroid_units":"mm","density":"uniform"},
        "inertia_properties":inertia_properties,"inertia_error":inertia_error,"bounds_kind":"conservative control hull bounds","bounds":{"min":xyz(bounds.min),"max":xyz(bounds.max)},
        "positions":positions,"normals":normals,"boundary_samples":boundary_samples,
        "mesh":{"positions":display.mesh.positions.iter().map(|p|xyz(*p)).collect::<Vec<_>>(),"normals":display.mesh.normals.iter().map(|n|[n.x,n.y,n.z]).collect::<Vec<_>>(),"triangles":display.mesh.triangles,"face_ids":display.mesh.face_ids},
        "vertex_nodes":display.vertex_nodes,"vertex_uv":display.vertex_uv,"vertex_faces":display.vertex_faces,"error_bounds":display.error_bounds,"subdivisions":display.subdivisions,
        "brep":crate::nurbs_graph_polygon_demo::serialize_graph_brep(shape.brep())?
    }))
}

pub fn graph_workflow_step_export_json(session: &GraphWorkflowSession) -> Result<String> {
    let shape = session
        .accepted_shape()
        .ok_or(Error::InvalidInput("graph workflow has no accepted shape"))?;
    let document = session.accepted_document().ok_or(Error::InvalidInput(
        "graph workflow has no accepted document",
    ))?;
    let step = shape.export_step_mm(Tolerance::new(document.tolerance.linear)?)?;
    Ok(serde_json::json!({"ok":true,"step":step,"units":"mm","schema":"AUTOMOTIVE_DESIGN","exact":true,"scope":"STEP export of actual accepted graph workflow geometry; workflow JSON stores modeling intent"}).to_string())
}

pub fn graph_workflow_point_query_json(
    session: &GraphWorkflowSession,
    point: Point3,
    linear: f64,
) -> Result<String> {
    let shape = session
        .accepted_shape()
        .ok_or(Error::InvalidInput("graph workflow has no accepted shape"))?;
    let tol = GeometryTolerance::new(linear, GeometryTolerance::default().angular(), 0.)?;
    let location = shape.classify_point(point, tol)?;
    let mapped = shape.source().placement().inverse()?.point(point);
    if !mapped.finite() {
        return Err(Error::InvalidInput(
            "graph workflow query source point is unresolved",
        ));
    }
    let (name, reason) = match location {
        PointLocation::Inside => (
            "Inside",
            "Inside retained material outside the checked Euclidean boundary band.",
        ),
        PointLocation::Outside => (
            "Outside",
            "Outside retained material outside the checked Euclidean boundary band.",
        ),
        PointLocation::Boundary => (
            "Boundary",
            "A retained boundary witness lies within the checked Euclidean tolerance.",
        ),
    };
    Ok(serde_json::json!({"ok":true,"location":name,"reason":reason,"point":[point.x,point.y,point.z],"source_point":[mapped.x,mapped.y,mapped.z],"units":"mm","linear_tolerance":linear,"relative_tolerance":0.,"scope":"point query of actual accepted graph workflow geometry"}).to_string())
}
