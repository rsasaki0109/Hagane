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
    let legacy = nurbs_surface_demo_json(height, weight, u, v)?;
    let mut json: serde_json::Value = serde_json::from_str(&legacy)
        .map_err(|_| Error::InvalidInput("surface demo serialization failed"))?;
    let face = NurbsFace::new(
        crate::nurbs_surface::nurbs_surface_demo_geometry(height, weight)?,
        1,
        Tolerance::default(),
    )?;
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
