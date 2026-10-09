//! Exact rectangular UV restriction of retained smooth or C0 NURBS faces.
use crate::*;

/// Retain a restricted surface and its exact boundary in original UV coordinates.
pub fn nurbs_surface_trim_demo_json(
    height: f64,
    weight: f64,
    u: f64,
    v: f64,
    chord_error: f64,
    ranges: [[f64; 2]; 2],
    crease: bool,
) -> Result<String> {
    let source = if crease {
        crate::nurbs_surface_crease_demo::nurbs_surface_crease_geometry(height, weight)?
    } else {
        crate::nurbs_surface::nurbs_surface_demo_geometry(height, weight)?
            .insert_knot(0, 0.35, 1)?
            .insert_knot(1, 0.7, 1)?
    };
    let source_domain = source.domain();
    let face =
        NurbsFace::new(source, 1, Tolerance::default())?.trimmed(ranges, Tolerance::default())?;
    let Surface::Nurbs(surface) = &face.face.surface else {
        return Err(Error::Unsupported("restricted demo requires a NURBS face"));
    };
    let sides = [
        if u == ranges[0][0] {
            KnotSide::Right
        } else {
            KnotSide::Left
        },
        if v == ranges[1][0] {
            KnotSide::Right
        } else {
            KnotSide::Left
        },
    ];
    let selected = surface.evaluate_with_partials(u, v, sides)?;
    let normal = selected.normal()?;
    let boundaries = surface.boundary_edges()?;
    let mut sections = Vec::new();
    for (axis, parameter, boundary, forward) in [
        (1, ranges[1][0], true, true),
        (0, ranges[0][1], true, true),
        (1, ranges[1][1], true, false),
        (0, ranges[0][0], true, false),
        (0, u, false, true),
        (1, v, false, true),
    ] {
        let curve = if boundary {
            boundaries[sections.len()].curve.clone()
        } else {
            surface.isocurve(axis, parameter)?
        };
        let domain = curve.domain();
        let polyline = curve.tessellate_bounded(0.05, 16384)?;
        sections.push(serde_json::json!({
            "fixed_axis":axis,"fixed_parameter":parameter,"boundary":boundary,"forward":forward,
            "samples":polyline.points.iter().flat_map(|p|[p.x,p.y,p.z]).collect::<Vec<_>>(),
            "parameters":polyline.parameters,"error_bounds":polyline.error_bounds,
            "parameter_range":domain,"degree":curve.degree(),"knots":curve.knots(),
            "control_points":curve.control_points().iter().map(|p|[p.x,p.y,p.z]).collect::<Vec<_>>(),"weights":curve.weights(),
            "pcurve_origin":if axis==0 {[parameter,0.]} else {[0.,parameter]},
            "pcurve_direction":if axis==0 {[0.,1.]} else {[1.,0.]}
        }));
    }
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
                let n = bounded.mesh.normals[i];
                [n.x, n.y, n.z]
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
    let patch_ranges: Vec<_> = surface
        .bezier_patches()?
        .iter()
        .map(|p| p.parameter_ranges)
        .collect();
    Ok(serde_json::json!({
        "height":height,"weight":weight,"u":u,"v":v,"crease":crease,"trim_ranges":ranges,"domain":surface.domain(),"source_domain":source_domain,
        "control_counts":surface.control_counts(),"patch_ranges":patch_ranges,"surface_error":chord_error,
        "point":[selected.point.x,selected.point.y,selected.point.z],"du":[selected.du.x,selected.du.y,selected.du.z],"dv":[selected.dv.x,selected.dv.y,selected.dv.z],"normal":[normal.x,normal.y,normal.z],
        "normal_side":if sides[0]==KnotSide::Left {0} else {1},"positions":positions,"normals":normals,
        "cells":bounded.error_bounds.len(),"triangles":bounded.mesh.triangles.len(),"cell_ranges":bounded.uv_ranges,"cell_bounds":bounded.error_bounds,
        "triangle_nodes":triangle_nodes,"triangle_uvs":triangle_uvs,"shading_vertices":bounded.mesh.positions.len(),"geometric_vertices":bounded.vertex_nodes.iter().copied().max().map_or(0,|n|n+1),
        "sections":sections,"section_chord_error":0.05,
        "brep":{"vertices":face.vertices.iter().map(|v|[v.point.x,v.point.y,v.point.z]).collect::<Vec<_>>(),"edge_vertices":face.edges.iter().map(|e|e.vertices).collect::<Vec<_>>(),"edge_ranges":face.edges.iter().map(|e|e.curve.range()).collect::<Vec<_>>(),"coedge_edges":[0,1,2,3],"coedge_forward":[true,true,false,false],"orientation":1,"faces":1,"closed":false,"validation":"exact rectangular UV restriction; global surface regularity is not certified"}
    }).to_string())
}
