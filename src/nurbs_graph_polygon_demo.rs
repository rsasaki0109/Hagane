//! Checked convex source-UV graph solids shared by native and WASM demos.
use crate::*;
#[allow(clippy::too_many_arguments)]
pub fn nurbs_graph_polygon_demo_json(
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
    polygon: Vec<[f64; 2]>,
) -> Result<String> {
    let source = crate::nurbs_graph_classification_demo::query_source(
        [width, depth, height],
        bulge,
        error,
        angle,
        [tx, ty, tz],
        [[u0, u1], [v0, v1]],
    )?;
    let body = NurbsGraphPolygonSolid::new(&source, polygon, Tolerance::default())?;
    let text = serialize_polygon_graph(&body, width, depth, height, bulge, error)?;
    let mut data: serde_json::Value = serde_json::from_str(&text)
        .map_err(|_| Error::InvalidInput("polygon graph display serialization failed"))?;
    data["placement"] =
        serde_json::json!({"angle":angle,"translation":[tx,ty,tz],"axis":[0.,1.,0.]});
    Ok(data.to_string())
}
/// Safe variable-length numeric payload: 13 model values followed by 3..16 UV pairs.
pub fn nurbs_graph_polygon_numeric_demo_json(values: &[f64]) -> Result<String> {
    if !(19..=45).contains(&values.len())
        || values.len() % 2 != 1
        || values.iter().any(|x| !x.is_finite())
    {
        return Err(Error::InvalidInput(
            "polygon graph payload needs 13 finite model values and 3..16 UV pairs",
        ));
    }
    let polygon = values[13..].as_chunks::<2>().0.to_vec();
    nurbs_graph_polygon_demo_json(
        values[0], values[1], values[2], values[3], values[4], values[5], values[6], values[7],
        values[8], values[9], values[10], values[11], values[12], polygon,
    )
}
pub(crate) fn serialize_polygon_graph(
    graph: &NurbsGraphPolygonSolid,
    width: f64,
    depth: f64,
    height: f64,
    bulge: f64,
    error: f64,
) -> Result<String> {
    graph.validate(Tolerance::default())?;
    let display = graph.tessellate_bounded(error, 65536, Tolerance::default())?;
    let solid = graph.brep();
    let xyz = |p: Point3| [p.x, p.y, p.z];
    let positions = display
        .mesh
        .triangles
        .iter()
        .flat_map(|t| t.iter().flat_map(|id| xyz(display.mesh.positions[*id])))
        .collect::<Vec<_>>();
    let normals = display
        .mesh
        .triangles
        .iter()
        .flat_map(|t| {
            t.iter().flat_map(|id| {
                let n = display.mesh.normals[*id];
                [n.x, n.y, n.z]
            })
        })
        .collect::<Vec<_>>();
    let boundary_samples = solid
        .edges
        .iter()
        .map(|edge| match &edge.curve {
            Curve::Line { a, b } => Ok(vec![a.x, a.y, a.z, b.x, b.y, b.z]),
            Curve::Nurbs(curve) => Ok(curve
                .tessellate_bounded(error, 65536)?
                .points
                .iter()
                .flat_map(|p| xyz(*p))
                .collect::<Vec<_>>()),
            _ => Err(Error::Unsupported(
                "graph demo boundary curve is unsupported",
            )),
        })
        .collect::<Result<Vec<_>>>()?;
    let bounds = graph.bounds()?;
    let mass = graph.mass_properties(Tolerance::default())?;
    Ok(serde_json::json!({
        "polygon":graph.polygon(),"bounds_kind":"conservative control hull bounds","scope":"strictly convex CCW source-UV polygon graph solid; generic STEP, classification and Booleans remain unsupported","width":width,"depth":depth,"height":height,"bulge":bulge,"error":error,"source_domain":graph.source().source_domain(),"volume":mass.volume,"mass_properties":{"volume":mass.volume,"centroid":xyz(mass.centroid),"units":"mm","volume_units":"mm3","centroid_units":"mm","density":"uniform","method":"polynomial fan integration"},"bounds":{"min":xyz(bounds.min),"max":xyz(bounds.max)},
        "positions":positions,"normals":normals,"boundary_samples":boundary_samples,
        "mesh":{"positions":display.mesh.positions.iter().map(|p|xyz(*p)).collect::<Vec<_>>(),"normals":display.mesh.normals.iter().map(|n|[n.x,n.y,n.z]).collect::<Vec<_>>(),"triangles":display.mesh.triangles,"face_ids":display.mesh.face_ids},
        "vertex_nodes":display.vertex_nodes,"vertex_uv":display.vertex_uv,"vertex_faces":display.vertex_faces,"error_bounds":display.error_bounds,"subdivisions":display.subdivisions,
        "brep":serialize_polygon_brep(graph)?
    }).to_string())
}

pub(crate) fn serialize_polygon_brep(graph: &NurbsGraphPolygonSolid) -> Result<serde_json::Value> {
    graph.validate(Tolerance::default())?;
    serialize_graph_brep(graph.brep())
}

/// Geometry metadata only; callers validate their typed wrapper before serialization.
pub(crate) fn serialize_graph_brep(solid: &Solid) -> Result<serde_json::Value> {
    let xyz = |p: Point3| [p.x, p.y, p.z];
    let curves=solid.edges.iter().map(|edge|match &edge.curve{
        Curve::Nurbs(curve)=>Ok(serde_json::json!({"degree":curve.degree(),"knots":curve.knots(),"weights":curve.weights(),"control_points":curve.control_points().iter().map(|p|xyz(*p)).collect::<Vec<_>>()})),
        Curve::Line{a,b}=>Ok(serde_json::json!({"degree":1,"knots":[0.,0.,1.,1.],"weights":[1.,1.],"control_points":[xyz(*a),xyz(*b)]})),
        _=>Err(Error::Unsupported("graph demo boundary curve is unsupported")),
    }).collect::<Result<Vec<_>>>()?;
    let face_edges = solid
        .shell
        .faces
        .iter()
        .map(|face| {
            face.wires
                .iter()
                .flat_map(|wire| wire.coedges.iter().map(|c| c.edge))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    // Expose the oriented occurrence: face reversal reverses every coedge.
    let face_forwards = solid
        .shell
        .faces
        .iter()
        .map(|face| {
            face.wires
                .iter()
                .flat_map(|wire| {
                    wire.coedges.iter().map(|c| {
                        if face.orientation > 0 {
                            c.forward
                        } else {
                            !c.forward
                        }
                    })
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let surfaces=solid.shell.faces.iter().map(|face|match &face.surface {
        Surface::Nurbs(source)=>Ok(serde_json::json!({"kind":"nurbs","degrees":source.degrees(),"knots":[source.knots(0)?,source.knots(1)?],"counts":source.control_counts(),"domain":source.domain(),"weights":source.weights(),"control_points":source.control_points().iter().map(|p|xyz(*p)).collect::<Vec<_>>() })),
        Surface::Plane{origin,u,v}=>Ok(serde_json::json!({"kind":"plane","origin":xyz(*origin),"u":[u.x,u.y,u.z],"v":[v.x,v.y,v.z]})),
        _=>Err(Error::Unsupported("graph demo supporting surface is unsupported")),
    }).collect::<Result<Vec<_>>>()?;
    let pcurves = solid
        .shell
        .faces
        .iter()
        .map(|face| {
            face.wires
                .iter()
                .flat_map(|wire| {
                    wire.coedges.iter().map(|c| match c.pcurve {
                        PCurve::Affine { origin, direction } => {
                            Ok(serde_json::json!({"origin":origin,"direction":direction}))
                        }
                        _ => Err(Error::Unsupported("graph demo pcurve is unsupported")),
                    })
                })
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<Vec<_>>>()?;
    let wire_edges = solid
        .shell
        .faces
        .iter()
        .map(|f| {
            f.wires
                .iter()
                .map(|w| w.coedges.iter().map(|c| c.edge).collect::<Vec<_>>())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let wire_forwards = solid
        .shell
        .faces
        .iter()
        .map(|f| {
            f.wires
                .iter()
                .map(|w| {
                    w.coedges
                        .iter()
                        .map(|c| {
                            if f.orientation > 0 {
                                c.forward
                            } else {
                                !c.forward
                            }
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let wire_pcurves = solid
        .shell
        .faces
        .iter()
        .map(|f| {
            f.wires
                .iter()
                .map(|w| {
                    w.coedges
                        .iter()
                        .map(|c| match c.pcurve {
                            PCurve::Affine { origin, direction } => {
                                Ok(serde_json::json!({"origin":origin,"direction":direction}))
                            }
                            _ => Err(Error::Unsupported("graph demo pcurve is unsupported")),
                        })
                        .collect::<Result<Vec<_>>>()
                })
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(
        serde_json::json!({"wire_edges":wire_edges,"wire_forwards":wire_forwards,"wire_pcurves":wire_pcurves,"face_orientations":solid.shell.faces.iter().map(|f|f.orientation).collect::<Vec<_>>(),"vertices":solid.vertices.iter().map(|v|xyz(v.point)).collect::<Vec<_>>(),"edge_vertices":solid.edges.iter().map(|e|e.vertices).collect::<Vec<_>>(),"faces":solid.shell.faces.len(),"edges":solid.edges.len(),"closed":true,"face_edges":face_edges,"face_forwards":face_forwards,"curves":curves,"pcurves":pcurves,"surfaces":surfaces}),
    )
}
