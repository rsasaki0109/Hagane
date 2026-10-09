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
    let source = if crease {
        crate::nurbs_surface_crease_demo::nurbs_surface_crease_geometry(height, weight)?
    } else {
        crate::nurbs_surface::nurbs_surface_demo_geometry(height, weight)?
            .insert_knot(0, 0.35, 1)?
            .insert_knot(1, 0.7, 1)?
    };
    hole_display_json(
        source,
        height,
        weight,
        chord_error,
        hole_width,
        crease,
        false,
    )
}

/// Injective polynomial fixture whose only singular tangent is inside the hole.
pub(crate) fn nurbs_surface_singular_geometry(height: f64) -> Result<NurbsSurface> {
    if !height.is_finite() || height.abs() > 100. {
        return Err(Error::InvalidInput(
            "singular fixture height must be finite within [-100,100]",
        ));
    }
    // Exact simplified values of 120 * (cubic[j] + square[i] * linear[j]).
    // Evaluating the 1/6 coefficients first introduces a tiny false Jacobian
    // at the intended singular center; these integer Bernstein controls do not.
    let y = [
        [-30., 10., -10., 30.],
        [0., 20., -20., 0.],
        [-30., 10., -10., 30.],
    ];
    let bu = [0., 2., 0.];
    let bv = [0., 4. / 3., 4. / 3., 0.];
    let mut controls = Vec::new();
    for i in 0..3 {
        for j in 0..4 {
            controls.push(Point3::new(
                40. * i as f64 - 40.,
                y[i][j],
                height * bu[i] * bv[j],
            ));
        }
    }
    NurbsSurface::new(
        [2, 3],
        [
            vec![0., 0., 0., 1., 1., 1.],
            vec![0., 0., 0., 0., 1., 1., 1., 1.],
        ],
        [3, 4],
        controls,
        vec![1.; 12],
    )
}

/// A singularity in excluded UV space does not invalidate retained material.
pub fn nurbs_surface_singular_hole_demo_json(
    height: f64,
    chord_error: f64,
    hole_width: f64,
) -> Result<String> {
    hole_display_json(
        nurbs_surface_singular_geometry(height)?,
        height,
        1.,
        chord_error,
        hole_width,
        false,
        true,
    )
}

fn hole_display_json(
    source: NurbsSurface,
    height: f64,
    weight: f64,
    chord_error: f64,
    hole_width: f64,
    crease: bool,
    singular: bool,
) -> Result<String> {
    if !hole_width.is_finite() || hole_width <= 0. {
        return Err(Error::InvalidInput(
            "UV hole width must be finite and positive",
        ));
    }
    // This optional diagnostic is about excluded UV space. Ambiguous C0
    // derivatives or a singular center must not invalidate retained material.
    let center_normal_available = source
        .partials(0.5, 0.5)
        .and_then(|evaluation| evaluation.normal())
        .is_ok();
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
        "height":height,"weight":weight,"crease":crease,"singular_source":singular,"source_degrees":surface.degrees(),"source_center_normal_available":center_normal_available,"hole_width":hole_width,"outer":face.outer(),"holes":face.holes(),"domain":surface.domain(),"source_domain":[[0.,1.],[0.,1.]],
        "control_counts":surface.control_counts(),"surface_error":chord_error,"positions":positions,"normals":normals,
        "cells":bounded.error_bounds.len(),"triangles":bounded.mesh.triangles.len(),"cell_ranges":bounded.uv_ranges,"cell_bounds":bounded.error_bounds,
        "triangle_nodes":triangle_nodes,"triangle_uvs":triangle_uvs,"shading_vertices":bounded.mesh.positions.len(),"geometric_vertices":bounded.vertex_nodes.iter().copied().max().map_or(0,|n|n+1),
        "sections":sections,"section_chord_error":0.05,
        "brep":{"vertices":face.vertices.iter().map(|v|[v.point.x,v.point.y,v.point.z]).collect::<Vec<_>>(),"edge_vertices":face.edges.iter().map(|e|e.vertices).collect::<Vec<_>>(),"edge_ranges":face.edges.iter().map(|e|e.curve.range()).collect::<Vec<_>>(),
            "wire_edges":face.face.wires.iter().map(|w|w.coedges.iter().map(|c|c.edge).collect::<Vec<_>>()).collect::<Vec<_>>(),"wire_forward":face.face.wires.iter().map(|w|w.coedges.iter().map(|c|c.forward).collect::<Vec<_>>()).collect::<Vec<_>>(),"wire_uvs":wire_uvs,
            "orientation":face.face.orientation,"faces":1,"closed":false,"validation":"exact rectangular outer and inner UV wires; global surface regularity is not certified"}
    }).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn excluded_c0_center_diagnostic_does_not_reject_roof_material() {
        let json = nurbs_surface_hole_demo_json(35., 1., 0.1, 0.3, true).unwrap();
        let data: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(data["source_center_normal_available"], false);
        assert_eq!(data["singular_source"], false);
        assert!(data["cells"].as_u64().unwrap() > 0);
        assert_eq!(data["brep"]["wire_edges"].as_array().unwrap().len(), 2);
    }
    #[test]
    fn polynomial_center_has_no_normal_but_retained_material_is_regular() {
        let source = nurbs_surface_singular_geometry(35.).unwrap();
        assert!(source.partials(0.5, 0.5).unwrap().normal().is_err());
        assert!(source.sample_grid([2, 2]).is_err());
        for (u, v) in [
            (0.1, 0.1),
            (0.35, 0.5),
            (0.65, 0.5),
            (0.5, 0.35),
            (0.5, 0.65),
            (0.9, 0.9),
        ] {
            let e = source.partials(u, v).unwrap();
            let du = u - 0.5;
            let dv = v - 0.5;
            let expected = Point3::new(
                80. * u - 40.,
                120. * (dv.powi(3) + du.powi(2) * dv),
                35. * 16. * u * (1. - u) * v * (1. - v),
            );
            assert!((e.point - expected).norm() < 1e-12);
            assert!(e.normal().unwrap().z > 0.);
        }
    }
}
