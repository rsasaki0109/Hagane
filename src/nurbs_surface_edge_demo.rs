//! Exact rational edge lifting an affine UV path on a retained NURBS surface.
use crate::*;

/// The selected edge is independent exact B-rep geometry, not a trim-loop face.
pub fn nurbs_surface_edge_demo_json(
    height: f64,
    weight: f64,
    start: [f64; 2],
    end: [f64; 2],
    chord_error: f64,
    crease: bool,
) -> Result<String> {
    let source = if crease {
        crate::nurbs_surface_crease_demo::nurbs_surface_crease_geometry(height, weight)?
    } else {
        crate::nurbs_surface::nurbs_surface_demo_geometry(height, weight)?
            .insert_knot(0, 0.35, 1)?
            .insert_knot(1, 0.7, 1)?
    };
    let edge = NurbsSurfaceEdge::new(source, start, end, Tolerance::default())?;
    let Curve::Nurbs(curve) = &edge.edge.curve else {
        return Err(Error::Unsupported(
            "surface edge demo requires exact NURBS edge",
        ));
    };
    let (point, tangent) = curve.evaluate_with_derivative(0.5, KnotSide::Left)?;
    let polyline = edge.tessellate_bounded(chord_error, 16384, Tolerance::default())?;
    let background = NurbsFace::new(edge.surface.clone(), 1, Tolerance::default())?
        .tessellate_bounded(0.2, 16384, Tolerance::default())?;
    let positions: Vec<_> = background
        .mesh
        .triangles
        .iter()
        .flat_map(|t| {
            t.iter().flat_map(|&i| {
                let p = background.mesh.positions[i];
                [p.x, p.y, p.z]
            })
        })
        .collect();
    let normals: Vec<_> = background
        .mesh
        .triangles
        .iter()
        .flat_map(|t| {
            t.iter().flat_map(|&i| {
                let n = background.mesh.normals[i];
                [n.x, n.y, n.z]
            })
        })
        .collect();
    let span_ranges: Vec<_> = curve
        .bezier_spans()?
        .iter()
        .map(|s| s.parameter_range)
        .collect();
    let PCurve::Affine { origin, direction } = &edge.pcurve else {
        return Err(Error::Unsupported(
            "surface edge demo requires affine pcurve",
        ));
    };
    Ok(serde_json::json!({
        "height":height,"weight":weight,"crease":crease,"start":edge.endpoints()[0],"end":edge.endpoints()[1],"source_domain":edge.surface.domain(),"source_control_counts":edge.surface.control_counts(),
        "selected_parameter":0.5,"tangent_side":0,"point":[point.x,point.y,point.z],"tangent":[tangent.x,tangent.y,tangent.z],
        "samples":polyline.points.iter().flat_map(|p|[p.x,p.y,p.z]).collect::<Vec<_>>(),"parameters":polyline.parameters,"error_bounds":polyline.error_bounds,"edge_error":chord_error,
        "curve":{"degree":curve.degree(),"knots":curve.knots(),"weights":curve.weights(),"control_points":curve.control_points().iter().map(|p|[p.x,p.y,p.z]).collect::<Vec<_>>(),"parameter_range":curve.domain()},
        "span_ranges":span_ranges,"pcurve_origin":origin,"pcurve_direction":direction,
        "brep":{"vertices":edge.vertices.iter().map(|v|[v.point.x,v.point.y,v.point.z]).collect::<Vec<_>>(),"edge_vertices":edge.edge.vertices,"edge_range":edge.edge.curve.range(),"edges":1,"closed":false,"validation":"exact lifted affine UV edge; not a general trimmed face"},
        "positions":positions,"normals":normals,"background_error":0.2,"background_cells":background.error_bounds.len(),"background_bounds":background.error_bounds
    }).to_string())
}
