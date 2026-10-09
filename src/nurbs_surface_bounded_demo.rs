//! Bounded display of the exact retained rectangular rational B-rep demo face.
use crate::*;

/// Same surface fixture as the compatibility demo, with bounded face triangles.
pub fn nurbs_surface_bounded_demo_json(
    height: f64,
    weight: f64,
    u: f64,
    v: f64,
    chord_error: f64,
) -> Result<String> {
    surface_display_json(height, weight, u, v, chord_error, false)
}
/// Shape-preserving insertion creates four C1 spans, meshed as one retained face.
pub fn nurbs_surface_multispan_demo_json(
    height: f64,
    weight: f64,
    u: f64,
    v: f64,
    chord_error: f64,
) -> Result<String> {
    surface_display_json(height, weight, u, v, chord_error, true)
}
fn surface_display_json(
    height: f64,
    weight: f64,
    u: f64,
    v: f64,
    chord_error: f64,
    refine: bool,
) -> Result<String> {
    let legacy = nurbs_surface_demo_json(height, weight, u, v)?;
    let mut json: serde_json::Value = serde_json::from_str(&legacy)
        .map_err(|_| Error::InvalidInput("surface demo serialization failed"))?;
    let source = crate::nurbs_surface::nurbs_surface_demo_geometry(height, weight)?;
    let source = if refine {
        source.insert_knot(0, 0.35, 1)?.insert_knot(1, 0.7, 1)?
    } else {
        source
    };
    let patch_ranges: Vec<_> = source
        .bezier_patches()?
        .iter()
        .map(|p| p.parameter_ranges)
        .collect();
    json["control_counts"] = serde_json::json!(source.control_counts());
    json["patch_ranges"] = serde_json::json!(patch_ranges);
    json["refined"] = serde_json::json!(refine);
    let face = NurbsFace::new(source, 1, Tolerance::default())?;
    json["brep"]["vertices"] = serde_json::json!(face
        .vertices
        .iter()
        .map(|v| [v.point.x, v.point.y, v.point.z])
        .collect::<Vec<_>>());
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
    json["positions"] = serde_json::json!(positions);
    json["normals"] = serde_json::json!(normals);
    json["surface_error"] = serde_json::json!(chord_error);
    json["cell_ranges"] = serde_json::json!(bounded.uv_ranges);
    json["cell_bounds"] = serde_json::json!(bounded.error_bounds);
    json["cells"] = serde_json::json!(bounded.error_bounds.len());
    json["triangles"] = serde_json::json!(bounded.mesh.triangles.len());
    Ok(json.to_string())
}
