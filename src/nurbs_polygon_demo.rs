//! Native and WASM share retained polygon B-rep construction and display.
use crate::*;
pub fn nurbs_polygon_demo_json(
    height: f64,
    weight: f64,
    error: f64,
    mode: u32,
    corners: Vec<[f64; 2]>,
) -> Result<String> {
    if !height.is_finite() || height.abs() > 100. {
        return Err(Error::InvalidInput(
            "polygon demo height must be finite and within [-100,100]",
        ));
    }
    if mode > 2 {
        return Err(Error::Unsupported("polygon demo mode must be 0, 1 or 2"));
    }
    let surface = if mode == 2 {
        let mut points = Vec::new();
        let mut weights = Vec::new();
        for i in 0..3 {
            for j in 0..3 {
                points.push(Point3::new(
                    i as f64 * 40.,
                    j as f64 * 40.,
                    height * ((if i == 1 { 1. } else { 0. }) + (if j == 1 { 0.6 } else { 0. })),
                ));
                weights.push(if i == 1 && j == 1 { weight } else { 1. });
            }
        }
        let knots = vec![0., 0., 0.5, 1., 1.];
        NurbsSurface::new([1, 1], [knots.clone(), knots], [3, 3], points, weights)?
    } else {
        let source = crate::nurbs_surface::nurbs_surface_demo_geometry(height, weight)?;
        if mode == 1 {
            source.insert_knot(0, 0.35, 1)?.insert_knot(1, 0.7, 1)?
        } else {
            source
        }
    };
    let face = NurbsPolygonFace::new(surface, corners.clone(), 1, Tolerance::default())?;
    let display = if mode == 2 {
        face.tessellate_crease_bounded(error, 65536, Tolerance::default())?
    } else {
        face.tessellate_bounded(error, 65536, Tolerance::default())?
    };
    let boundary_samples = face
        .boundary
        .tessellate_boundary(error, 65536, Tolerance::default())?
        .iter()
        .map(|line| {
            line.points
                .iter()
                .flat_map(|p| [p.x, p.y, p.z])
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let positions = display
        .mesh
        .triangles
        .iter()
        .flat_map(|t| {
            t.iter().flat_map(|id| {
                let p = display.mesh.positions[*id];
                [p.x, p.y, p.z]
            })
        })
        .collect::<Vec<_>>();
    let normals = display
        .mesh
        .triangles
        .iter()
        .flat_map(|t| {
            t.iter().flat_map(|id| {
                let p = display.mesh.normals[*id];
                [p.x, p.y, p.z]
            })
        })
        .collect::<Vec<_>>();
    let source = &face.boundary.surface;
    let pcurves = face.face.wires[0]
        .coedges
        .iter()
        .map(|c| match c.pcurve {
            PCurve::Affine { origin, direction } => {
                Ok(serde_json::json!({"origin":origin,"direction":direction}))
            }
            _ => Err(Error::InvalidTopology(
                "polygon demo requires affine pcurves",
            )),
        })
        .collect::<Result<Vec<_>>>()?;
    let curves=face.boundary.edges.iter().map(|edge|match &edge.curve {
        Curve::Nurbs(c)=>Ok(serde_json::json!({"degree":c.degree(),"knots":c.knots(),"weights":c.weights(),"control_points":c.control_points().iter().map(|p|[p.x,p.y,p.z]).collect::<Vec<_>>() })),
        _=>Err(Error::InvalidTopology("polygon demo requires rational curves")),
    }).collect::<Result<Vec<_>>>()?;
    Ok(serde_json::json!({
        "height":height,"weight":weight,"error":error,"mode":mode,"corners":corners,
        "positions":positions,"normals":normals,"boundary_samples":boundary_samples,
        "mesh":{"positions":display.mesh.positions.iter().map(|p|[p.x,p.y,p.z]).collect::<Vec<_>>(),"normals":display.mesh.normals.iter().map(|p|[p.x,p.y,p.z]).collect::<Vec<_>>(),"triangles":display.mesh.triangles},
        "vertex_uv":display.vertex_uv,"vertex_nodes":display.vertex_nodes,"normal_sides":display.normal_sides.iter().map(|s|s.map(|side|if side==KnotSide::Left {0}else {1})).collect::<Vec<_>>(),"error_bounds":display.error_bounds,
        "source":{"degrees":source.degrees(),"knots":[source.knots(0)?,source.knots(1)?],"counts":source.control_counts(),"weights":source.weights(),"control_points":source.control_points().iter().map(|p|[p.x,p.y,p.z]).collect::<Vec<_>>(),"domain":source.domain()},
        "brep":{"vertices":face.boundary.vertices.iter().map(|p|[p.point.x,p.point.y,p.point.z]).collect::<Vec<_>>(),"edge_vertices":face.boundary.edges.iter().map(|e|e.vertices).collect::<Vec<_>>(),"pcurves":pcurves,"curves":curves,"edges":face.boundary.edges.len(),"wires":1,"orientation":face.face.orientation,"closed":false}
    }).to_string())
}
