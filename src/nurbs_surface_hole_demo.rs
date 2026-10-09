//! Display an exact retained face with an oriented rectangular inner UV wire.
use crate::*;

/// The UV opening is retained in B-rep topology before display tessellation.
pub fn nurbs_surface_hole_demo_json(
    height: f64,
    weight: f64,
    chord_error: f64,
    hole_width: f64,
    crease: bool,
) -> Result<String> {
    if !hole_width.is_finite() || hole_width <= 0. {
        return Err(Error::InvalidInput(
            "UV hole width must be finite and positive",
        ));
    }
    let source = if crease {
        crate::nurbs_surface_crease_demo::nurbs_surface_crease_geometry(height, weight)?
    } else {
        crate::nurbs_surface::nurbs_surface_demo_geometry(height, weight)?
            .insert_knot(0, 0.35, 1)?
            .insert_knot(1, 0.7, 1)?
    };
    let outer = [[0.1, 0.9]; 2];
    let holes = vec![[[0.5 - hole_width / 2., 0.5 + hole_width / 2.]; 2]];
    let face = NurbsHoledFace::new(source, outer, holes, 1, Tolerance::default())?;
    let Surface::Nurbs(surface) = &face.face.surface else {
        return Err(Error::Unsupported(
            "UV hole demo requires retained NURBS surface",
        ));
    };
    let mut sections = Vec::new();
    let mut wire_uvs = Vec::new();
    for (wire_index, wire) in face.face.wires.iter().enumerate() {
        let mut corners = Vec::new();
        for coedge in &wire.coedges {
            let Curve::Nurbs(curve) = &face.edges[coedge.edge].curve else {
                return Err(Error::Unsupported(
                    "UV hole boundary requires exact NURBS edge",
                ));
            };
            let domain = curve.domain();
            corners.push(coedge.pcurve.evaluate(domain[usize::from(!coedge.forward)]));
            let PCurve::Affine { origin, direction } = &coedge.pcurve else {
                return Err(Error::Unsupported(
                    "UV hole boundary requires affine pcurve",
                ));
            };
            let polyline = curve.tessellate_bounded(0.05, 16384)?;
            sections.push(serde_json::json!({
                "wire":wire_index,"edge":coedge.edge,"forward":coedge.forward,
                "samples":polyline.points.iter().flat_map(|p|[p.x,p.y,p.z]).collect::<Vec<_>>(),
                "parameters":polyline.parameters,"error_bounds":polyline.error_bounds,"parameter_range":domain,
                "degree":curve.degree(),"knots":curve.knots(),"control_points":curve.control_points().iter().map(|p|[p.x,p.y,p.z]).collect::<Vec<_>>(),"weights":curve.weights(),
                "pcurve_origin":origin,"pcurve_direction":direction
            }));
        }
        wire_uvs.push(corners);
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
    Ok(serde_json::json!({
        "height":height,"weight":weight,"crease":crease,"hole_width":hole_width,"outer":face.outer(),"holes":face.holes(),"domain":surface.domain(),"source_domain":[[0.,1.],[0.,1.]],
        "control_counts":surface.control_counts(),"surface_error":chord_error,"positions":positions,"normals":normals,
        "cells":bounded.error_bounds.len(),"triangles":bounded.mesh.triangles.len(),"cell_ranges":bounded.uv_ranges,"cell_bounds":bounded.error_bounds,
        "triangle_nodes":triangle_nodes,"triangle_uvs":triangle_uvs,"shading_vertices":bounded.mesh.positions.len(),"geometric_vertices":bounded.vertex_nodes.iter().copied().max().map_or(0,|n|n+1),
        "sections":sections,"section_chord_error":0.05,
        "brep":{"vertices":face.vertices.iter().map(|v|[v.point.x,v.point.y,v.point.z]).collect::<Vec<_>>(),"edge_vertices":face.edges.iter().map(|e|e.vertices).collect::<Vec<_>>(),"edge_ranges":face.edges.iter().map(|e|e.curve.range()).collect::<Vec<_>>(),
            "wire_edges":face.face.wires.iter().map(|w|w.coedges.iter().map(|c|c.edge).collect::<Vec<_>>()).collect::<Vec<_>>(),"wire_forward":face.face.wires.iter().map(|w|w.coedges.iter().map(|c|c.forward).collect::<Vec<_>>()).collect::<Vec<_>>(),"wire_uvs":wire_uvs,
            "orientation":face.face.orientation,"faces":1,"closed":false,"validation":"exact rectangular outer and inner UV wires; global surface regularity is not certified"}
    }).to_string())
}
