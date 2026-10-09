//! Actual circular through-bore geometry and bounded display, without polygon substitution.
use crate::*;

pub fn nurbs_graph_circular_hole_demo_json(x: &[f64]) -> Result<String> {
    if x.len() != 16 || x.iter().any(|v| !v.is_finite()) {
        return Err(Error::InvalidInput(
            "circular hole needs exactly 16 finite values",
        ));
    }
    let tol = Tolerance::default();
    let source = crate::nurbs_graph_classification_demo::query_source(
        [x[0], x[1], x[2]],
        x[3],
        x[4],
        x[5],
        [x[6], x[7], x[8]],
        [[x[9], x[10]], [x[11], x[12]]],
    )?;
    let body = source.through_xy_circle([x[13], x[14]], x[15], tol)?;
    body.validate(tol)?;
    let display = body.tessellate_bounded(x[4], 65536, tol)?;
    let mass = body.mass_properties(tol)?;
    let (inertia_properties, inertia_error) =
        crate::nurbs_graph_polygon_demo::serialize_polygon_inertia(body.inertia_properties(tol));
    let bounds = body.bounds()?;
    let xyz = |p: Point3| [p.x, p.y, p.z];
    let positions: Vec<_> = display
        .mesh
        .triangles
        .iter()
        .flat_map(|t| t.iter().flat_map(|id| xyz(display.mesh.positions[*id])))
        .collect();
    let normals: Vec<_> = display
        .mesh
        .triangles
        .iter()
        .flat_map(|t| {
            t.iter().flat_map(|id| {
                let n = display.mesh.normals[*id];
                [n.x, n.y, n.z]
            })
        })
        .collect();
    let boundary_samples = body
        .brep()
        .edges
        .iter()
        .map(|edge| match &edge.curve {
            Curve::Line { a, b } => Ok(vec![a.x, a.y, a.z, b.x, b.y, b.z]),
            Curve::Nurbs(c) => Ok(c
                .tessellate_bounded(x[4], 65536)?
                .points
                .iter()
                .flat_map(|p| xyz(*p))
                .collect()),
            _ => Err(Error::Unsupported(
                "circular hole demo boundary curve unsupported",
            )),
        })
        .collect::<Result<Vec<_>>>()?;
    let source_volume = source.volume()?;
    let removed_volume = body.removed_volume(tol)?;
    Ok(serde_json::json!({
        "center":body.center(),"radius":body.radius(),"genus":1,"units":"mm",
        "scope":"one exact source-XY circular through bore in rectangular graph stock; no generic Boolean or STEP import",
        "width":x[0],"depth":x[1],"height":x[2],"bulge":x[3],"error":x[4],"source_domain":source.source_domain(),
        "placement":{"angle":x[5],"translation":[x[6],x[7],x[8]],"axis":[0.,1.,0.]},
        "volume":mass.volume,"source_volume":source_volume,"removed_volume":removed_volume,
        "mass_properties":{"volume":mass.volume,"centroid":xyz(mass.centroid),"units":"mm","volume_units":"mm3","centroid_units":"mm","density":"uniform","method":"real-exact polynomial quadrature over rectangle minus disk"},
        "inertia_properties":inertia_properties,"inertia_error":inertia_error,
        "display_trim_kind":"bounded chordal approximation of exact rational circular B-rep trim","bounds_kind":"conservative control hull bounds","bounds":{"min":xyz(bounds.min),"max":xyz(bounds.max)},
        "positions":positions,"normals":normals,"boundary_samples":boundary_samples,
        "mesh":{"positions":display.mesh.positions.iter().map(|p|xyz(*p)).collect::<Vec<_>>(),"normals":display.mesh.normals.iter().map(|n|[n.x,n.y,n.z]).collect::<Vec<_>>(),"triangles":display.mesh.triangles,"face_ids":display.mesh.face_ids},
        "vertex_nodes":display.vertex_nodes,"vertex_uv":display.vertex_uv,"vertex_faces":display.vertex_faces,"error_bounds":display.error_bounds,"subdivisions":display.subdivisions,
        "brep":crate::nurbs_graph_polygon_demo::serialize_graph_brep(body.brep())?
    }).to_string())
}
