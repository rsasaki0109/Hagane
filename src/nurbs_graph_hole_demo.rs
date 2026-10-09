//! Native and WASM use the same retained genus-one graph-solid construction.
use crate::*;
#[allow(clippy::too_many_arguments)]
pub fn nurbs_graph_hole_demo_json(
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
) -> Result<String> {
    let transform = Transform::translation(Vec3::new(tx, ty, tz))?
        .compose(Transform::rotation(Vec3::new(0., 1., 0.), angle)?)?;
    let source = NurbsGraphSolid::new([width, depth, height], bulge, Tolerance::default())?
        .trimmed_uv([[u0, u1], [v0, v1]], Tolerance::default())?
        .transformed(transform, Tolerance::default())?;
    let hole = [[hu0, hu1], [hv0, hv1]];
    let graph = NurbsGraphHoledSolid::new(&source, hole, Tolerance::default())?;
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
    let curves=solid.edges.iter().map(|edge|match &edge.curve{
        Curve::Nurbs(curve)=>Ok(serde_json::json!({"degree":curve.degree(),"knots":curve.knots(),"weights":curve.weights(),"control_points":curve.control_points().iter().map(|p|xyz(*p)).collect::<Vec<_>>()})),
        Curve::Line{a,b}=>Ok(serde_json::json!({"degree":1,"knots":[0.,0.,1.,1.],"weights":[1.,1.],"control_points":[xyz(*a),xyz(*b)]})),
        _=>Err(Error::Unsupported("graph demo boundary curve is unsupported")),
    }).collect::<Result<Vec<_>>>()?;
    let bounds = graph.bounds()?;
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
    let mass = graph.mass_properties(Tolerance::default())?;
    Ok(serde_json::json!({
        "width":width,"depth":depth,"height":height,"bulge":bulge,"error":error,"source_domain":source.source_domain(),"volume":graph.volume()?,"mass_properties":{"volume":mass.volume,"centroid":xyz(mass.centroid),"units":"mm","volume_units":"mm3","centroid_units":"mm","density":"uniform","method":"polynomial column integration"},"bounds":{"min":xyz(bounds.min),"max":xyz(bounds.max)},"hole":hole,"source_volume":source.volume()?,"removed_volume":source.volume()?-graph.volume()?,"placement":{"angle":angle,"translation":[tx,ty,tz],"axis":[0.,1.,0.]},"bounds_kind":"conservative control hull bounds",
        "positions":positions,"normals":normals,"boundary_samples":boundary_samples,
        "mesh":{"positions":display.mesh.positions.iter().map(|p|xyz(*p)).collect::<Vec<_>>(),"normals":display.mesh.normals.iter().map(|n|[n.x,n.y,n.z]).collect::<Vec<_>>(),"triangles":display.mesh.triangles,"face_ids":display.mesh.face_ids},
        "vertex_nodes":display.vertex_nodes,"vertex_uv":display.vertex_uv,"vertex_faces":display.vertex_faces,"error_bounds":display.error_bounds,"subdivisions":display.subdivisions,
        "brep":{"vertices":solid.vertices.iter().map(|v|xyz(v.point)).collect::<Vec<_>>(),"edge_vertices":solid.edges.iter().map(|e|e.vertices).collect::<Vec<_>>(),"faces":solid.shell.faces.len(),"edges":solid.edges.len(),"closed":true,"genus":1,"face_edges":face_edges,"face_wires":solid.shell.faces.iter().map(|f|f.wires.len()).collect::<Vec<_>>(),"wire_edges":solid.shell.faces.iter().map(|f|f.wires.iter().map(|w|w.coedges.iter().map(|c|c.edge).collect::<Vec<_>>()).collect::<Vec<_>>()).collect::<Vec<_>>(),"face_forwards":face_forwards,"curves":curves,"pcurves":pcurves,"surfaces":surfaces}
    }).to_string())
}
