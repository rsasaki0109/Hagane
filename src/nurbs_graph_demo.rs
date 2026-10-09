//! The native and browser demos use the same checked graph-solid kernel.
use crate::*;
pub fn nurbs_graph_solid_demo_json(
    width: f64,
    depth: f64,
    height: f64,
    bulge: f64,
    error: f64,
) -> Result<String> {
    let graph = NurbsGraphSolid::new([width, depth, height], bulge, Tolerance::default())?;
    serialize_graph(&graph, width, depth, height, bulge, error)
}
/// Rotate around the world Y axis, then translate the retained graph B-rep.
#[allow(clippy::too_many_arguments)]
pub fn nurbs_graph_placed_demo_json(
    width: f64,
    depth: f64,
    height: f64,
    bulge: f64,
    error: f64,
    angle: f64,
    tx: f64,
    ty: f64,
    tz: f64,
) -> Result<String> {
    let transform = Transform::translation(Vec3::new(tx, ty, tz))?
        .compose(Transform::rotation(Vec3::new(0., 1., 0.), angle)?)?;
    let graph = NurbsGraphSolid::new([width, depth, height], bulge, Tolerance::default())?
        .transformed(transform, Tolerance::default())?;
    let text = serialize_graph(&graph, width, depth, height, bulge, error)?;
    let mut data: serde_json::Value = serde_json::from_str(&text)
        .map_err(|_| Error::InvalidInput("graph demo JSON serialization failed"))?;
    data["placement"] =
        serde_json::json!({"angle":angle,"translation":[tx,ty,tz],"axis":[0.,1.,0.]});
    data["bounds_kind"] = serde_json::json!(if graph.placement() == Transform::IDENTITY
        && graph.source_domain() == [[0., 1.], [0., 1.]]
    {
        "exact local graph bounds"
    } else {
        "conservative control hull bounds"
    });
    Ok(data.to_string())
}
/// Retain a source-UV subrectangle, including its exact curved roof edges.
#[allow(clippy::too_many_arguments)]
pub fn nurbs_graph_trimmed_demo_json(
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
) -> Result<String> {
    let transform = Transform::translation(Vec3::new(tx, ty, tz))?
        .compose(Transform::rotation(Vec3::new(0., 1., 0.), angle)?)?;
    let graph = NurbsGraphSolid::new([width, depth, height], bulge, Tolerance::default())?
        .trimmed_uv([[u0, u1], [v0, v1]], Tolerance::default())?
        .transformed(transform, Tolerance::default())?;
    let text = serialize_graph(&graph, width, depth, height, bulge, error)?;
    let mut data: serde_json::Value = serde_json::from_str(&text)
        .map_err(|_| Error::InvalidInput("graph demo JSON serialization failed"))?;
    data["placement"] =
        serde_json::json!({"angle":angle,"translation":[tx,ty,tz],"axis":[0.,1.,0.]});
    data["bounds_kind"] = serde_json::json!(if graph.placement() == Transform::IDENTITY
        && graph.source_domain() == [[0., 1.], [0., 1.]]
    {
        "exact local graph bounds"
    } else {
        "conservative control hull bounds"
    });
    data["curved_walls"] =
        serde_json::json!(bulge != 0. && graph.source_domain() != [[0., 1.], [0., 1.]]);
    Ok(data.to_string())
}
/// Exact source-aligned plane partition; display one retained solid component.
#[allow(clippy::too_many_arguments)]
pub fn nurbs_graph_split_demo_json(
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
    axis: usize,
    parameter: f64,
    side: u32,
) -> Result<String> {
    if side > 1 {
        return Err(Error::InvalidInput("graph split demo side must be 0 or 1"));
    }
    let transform = Transform::translation(Vec3::new(tx, ty, tz))?
        .compose(Transform::rotation(Vec3::new(0., 1., 0.), angle)?)?;
    let source = NurbsGraphSolid::new([width, depth, height], bulge, Tolerance::default())?
        .trimmed_uv([[u0, u1], [v0, v1]], Tolerance::default())?
        .transformed(transform, Tolerance::default())?;
    let split = source.split_uv(axis, parameter, Tolerance::default())?;
    split.validate(Tolerance::default())?;
    let graph = if side == 0 {
        &split.negative
    } else {
        &split.positive
    };
    let text = serialize_graph(graph, width, depth, height, bulge, error)?;
    let mut data: serde_json::Value = serde_json::from_str(&text)
        .map_err(|_| Error::InvalidInput("graph demo JSON serialization failed"))?;
    let xyz = |p: Point3| [p.x, p.y, p.z];
    let samples = split
        .section
        .edges
        .iter()
        .map(|edge| match &edge.curve {
            Curve::Nurbs(c) => Ok(c
                .tessellate_bounded(error, 65536)?
                .points
                .iter()
                .flat_map(|p| xyz(*p))
                .collect::<Vec<_>>()),
            Curve::Line { a, b } => Ok(vec![a.x, a.y, a.z, b.x, b.y, b.z]),
            _ => Err(Error::Unsupported("graph split section curve unsupported")),
        })
        .collect::<Result<Vec<_>>>()?;
    let section_curves=split.section.edges.iter().map(|edge|match &edge.curve{
        Curve::Nurbs(c)=>Ok(serde_json::json!({"degree":c.degree(),"knots":c.knots(),"weights":c.weights(),"control_points":c.control_points().iter().map(|p|xyz(*p)).collect::<Vec<_>>()})),
        Curve::Line{a,b}=>Ok(serde_json::json!({"degree":1,"knots":[0.,0.,1.,1.],"weights":[1.,1.],"control_points":[xyz(*a),xyz(*b)]})),
        _=>Err(Error::Unsupported("graph split section curve unsupported")),
    }).collect::<Result<Vec<_>>>()?;
    let Surface::Nurbs(section_source) = &split.section.face.surface else {
        return Err(Error::InvalidTopology(
            "graph split section supporting surface unsupported",
        ));
    };
    let section_surface = serde_json::json!({"kind":"nurbs","degrees":section_source.degrees(),"knots":[section_source.knots(0)?,section_source.knots(1)?],"counts":section_source.control_counts(),"domain":section_source.domain(),"weights":section_source.weights(),"control_points":section_source.control_points().iter().map(|p|xyz(*p)).collect::<Vec<_>>()});
    let section_pcurves = split.section.face.wires[0]
        .coedges
        .iter()
        .map(|coedge| match coedge.pcurve {
            PCurve::Affine { origin, direction } => {
                Ok(serde_json::json!({"origin":origin,"direction":direction}))
            }
            _ => Err(Error::Unsupported("graph split section pcurve unsupported")),
        })
        .collect::<Result<Vec<_>>>()?;
    let Surface::Plane { origin, u, v } = &split.plane else {
        return Err(Error::InvalidTopology("graph split plane is not planar"));
    };
    let normal = u.cross(*v).normalized()?;
    data["placement"] =
        serde_json::json!({"angle":angle,"translation":[tx,ty,tz],"axis":[0.,1.,0.]});
    data["bounds_kind"] = serde_json::json!("conservative control hull bounds");
    data["curved_walls"] = serde_json::json!(bulge != 0.);
    data["split"] = serde_json::json!({"axis":axis,"parameter":parameter,"side":side,"source_domain":source.source_domain(),"source_volume":source.volume()?,"negative_volume":split.negative.volume()?,"positive_volume":split.positive.volume()?,"plane":{"origin":xyz(*origin),"normal":[normal.x,normal.y,normal.z]},"section":{"vertices":split.section.vertices.iter().map(|v|xyz(v.point)).collect::<Vec<_>>(),"edge_vertices":split.section.edges.iter().map(|e|e.vertices).collect::<Vec<_>>(),"closed":false,"boundary_samples":samples,"curves":section_curves,"surface":section_surface,"pcurves":section_pcurves,"orientation":split.section.face.orientation,"edge_forwards":split.section.face.wires[0].coedges.iter().map(|c|c.forward).collect::<Vec<_>>()}});
    Ok(data.to_string())
}
fn serialize_graph(
    graph: &NurbsGraphSolid,
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
    Ok(serde_json::json!({
        "width":width,"depth":depth,"height":height,"bulge":bulge,"error":error,"source_domain":graph.source_domain(),"volume":graph.volume()?,"bounds":{"min":xyz(bounds.min),"max":xyz(bounds.max)},
        "positions":positions,"normals":normals,"boundary_samples":boundary_samples,
        "mesh":{"positions":display.mesh.positions.iter().map(|p|xyz(*p)).collect::<Vec<_>>(),"normals":display.mesh.normals.iter().map(|n|[n.x,n.y,n.z]).collect::<Vec<_>>(),"triangles":display.mesh.triangles,"face_ids":display.mesh.face_ids},
        "vertex_nodes":display.vertex_nodes,"vertex_uv":display.vertex_uv,"vertex_faces":display.vertex_faces,"error_bounds":display.error_bounds,"subdivisions":display.subdivisions,
        "brep":{"vertices":solid.vertices.iter().map(|v|xyz(v.point)).collect::<Vec<_>>(),"edge_vertices":solid.edges.iter().map(|e|e.vertices).collect::<Vec<_>>(),"faces":solid.shell.faces.len(),"edges":solid.edges.len(),"closed":true,"face_edges":face_edges,"face_forwards":face_forwards,"curves":curves,"pcurves":pcurves,"surfaces":surfaces}
    }).to_string())
}
