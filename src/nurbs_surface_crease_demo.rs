//! Exact C0 rational roof fixture with explicit selected normal side.
use crate::*;

/// Piecewise planar rational surface with a C0 ridge at U=0.5.
/// The middle row's positive weight changes parameterization, not planar facets.
pub(crate) fn nurbs_surface_crease_geometry(height: f64, weight: f64) -> Result<NurbsSurface> {
    if !height.is_finite() || height.abs() > 100. {
        return Err(Error::InvalidInput(
            "crease height must be finite within [-100,100]",
        ));
    }
    let mut points = Vec::new();
    let mut weights = Vec::new();
    for (x, z, w) in [(-40., 0., 1.), (0., height, weight), (40., 0., 1.)] {
        for y in [-30., 30.] {
            points.push(Point3::new(x, y, z));
            weights.push(w);
        }
    }
    NurbsSurface::new(
        [1, 1],
        [vec![0., 0., 0.5, 1., 1.], vec![0., 0., 1., 1.]],
        [3, 2],
        points,
        weights,
    )
}
/// Display the retained C0 face with one-sided normals; side 0 is Left, 1 Right.
pub fn nurbs_surface_crease_demo_json(
    height: f64,
    weight: f64,
    u: f64,
    v: f64,
    chord_error: f64,
    side: u32,
) -> Result<String> {
    let selected_side = match side {
        0 => KnotSide::Left,
        1 => KnotSide::Right,
        _ => {
            return Err(Error::InvalidInput(
                "normal side must be 0 (left) or 1 (right)",
            ))
        }
    };
    let source = nurbs_surface_crease_geometry(height, weight)?;
    let selected = source.evaluate_with_partials(u, v, [selected_side, KnotSide::Right])?;
    let normal = selected.normal()?;
    let patch_ranges: Vec<_> = source
        .bezier_patches()?
        .iter()
        .map(|p| p.parameter_ranges)
        .collect();
    let boundaries = source.boundary_edges()?;
    let mut sections = Vec::new();
    for (axis, parameter, boundary, forward) in [
        (1, 0., true, true),
        (0, 1., true, true),
        (1, 1., true, false),
        (0, 0., true, false),
        (0, u, false, true),
        (1, v, false, true),
    ] {
        let curve = if boundary {
            boundaries[sections.len()].curve.clone()
        } else {
            source.isocurve(axis, parameter)?
        };
        let polyline = curve.tessellate_bounded(0.05, 16384)?;
        sections.push(serde_json::json!({"fixed_axis":axis,"fixed_parameter":parameter,"boundary":boundary,"forward":forward,
            "samples":polyline.points.iter().flat_map(|p|[p.x,p.y,p.z]).collect::<Vec<_>>(),"parameters":polyline.parameters,"error_bounds":polyline.error_bounds,
            "degree":curve.degree(),"knots":curve.knots(),"control_points":curve.control_points().iter().map(|p|[p.x,p.y,p.z]).collect::<Vec<_>>(),"weights":curve.weights(),"parameter_range":curve.domain(),
            "pcurve_origin":if axis==0 {[parameter,0.]} else {[0.,parameter]},"pcurve_direction":if axis==0 {[0.,1.]} else {[1.,0.]} }));
    }
    let face = NurbsFace::new(source, 1, Tolerance::default())?;
    let bounded = face.tessellate_bounded(chord_error, 16384, Tolerance::default())?;
    let positions: Vec<_> = bounded
        .mesh
        .triangles
        .iter()
        .flat_map(|t| {
            t.iter().flat_map(|&i| {
                let p = bounded.mesh.positions[i];
                [p.x, p.y, p.z]
            })
        })
        .collect();
    let normals: Vec<_> = bounded
        .mesh
        .triangles
        .iter()
        .flat_map(|t| {
            t.iter().flat_map(|&i| {
                let p = bounded.mesh.normals[i];
                [p.x, p.y, p.z]
            })
        })
        .collect();
    let triangle_nodes: Vec<_> = bounded
        .mesh
        .triangles
        .iter()
        .map(|t| t.map(|i| bounded.vertex_nodes[i]))
        .collect();
    let triangle_uvs: Vec<_> = bounded
        .mesh
        .triangles
        .iter()
        .map(|t| t.map(|i| bounded.vertex_uv[i]))
        .collect();
    Ok(serde_json::json!({"height":height,"weight":weight,"u":u,"v":v,"surface_error":chord_error,"crease":true,"normal_side":side,"refined":false,"control_counts":[3,2],"refined_control_counts":[3,2],"patch_ranges":patch_ranges,
        "point":[selected.point.x,selected.point.y,selected.point.z],"du":[selected.du.x,selected.du.y,selected.du.z],"dv":[selected.dv.x,selected.dv.y,selected.dv.z],"normal":[normal.x,normal.y,normal.z],
        "positions":positions,"normals":normals,"cell_ranges":bounded.uv_ranges,"cell_bounds":bounded.error_bounds,"cells":bounded.error_bounds.len(),"triangles":bounded.mesh.triangles.len(),
        "triangle_nodes":triangle_nodes,"triangle_uvs":triangle_uvs,"shading_vertices":bounded.mesh.positions.len(),"geometric_vertices":bounded.vertex_nodes.iter().copied().max().map_or(0,|n|n+1),
        "sections":sections,"section_chord_error":0.05,
        "brep":{"vertices":face.vertices.iter().map(|v|[v.point.x,v.point.y,v.point.z]).collect::<Vec<_>>(),"edge_vertices":face.edges.iter().map(|e|e.vertices).collect::<Vec<_>>(),"edge_ranges":face.edges.iter().map(|e|e.curve.range()).collect::<Vec<_>>(),"coedge_edges":[0,1,2,3],"coedge_forward":[true,true,false,false],"orientation":1,"faces":1,"closed":false,"validation":"canonical rectangular boundary; global surface regularity is not certified"}
    }).to_string())
}
