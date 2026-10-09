//! Exact convex graph stock with one convex polygon through opening.
use crate::*;
/// Safe payload: model13, outer count (0 or 3..16), opening count (3..16),
/// followed by outer UV pairs and opening UV pairs, at most 79 finite values.
pub fn nurbs_graph_polygon_hole_demo_json(x: &[f64]) -> Result<String> {
    if !(21..=79).contains(&x.len())
        || x.len().is_multiple_of(2)
        || x.iter().any(|v| !v.is_finite())
    {
        return Err(Error::InvalidInput(
            "polygon opening payload requires 21..79 finite values",
        ));
    }
    let count = |v: f64, outer: bool| -> Result<usize> {
        if v.fract() != 0. || !((3. ..=16.).contains(&v) || (outer && v == 0.)) {
            return Err(Error::InvalidInput(
                "polygon opening counts must be integral 3..16; outer count may be zero",
            ));
        }
        Ok(v as usize)
    };
    let no = count(x[13], true)?;
    let ni = count(x[14], false)?;
    if x.len() != 15 + 2 * (no + ni) {
        return Err(Error::InvalidInput(
            "polygon opening payload count does not match UV pairs",
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
    let polygon = if no == 0 {
        vec![[x[9], x[11]], [x[10], x[11]], [x[10], x[12]], [x[9], x[12]]]
    } else {
        x[15..15 + 2 * no].as_chunks::<2>().0.to_vec()
    };
    let opening = x[15 + 2 * no..].as_chunks::<2>().0.to_vec();
    let outer = NurbsGraphPolygonSolid::new(&source, polygon.clone(), tol)?;
    let removed_volume = NurbsGraphPolygonSolid::new(&source, opening.clone(), tol)?.volume()?;
    let body = NurbsGraphPolygonHoledSolid::new(&outer, opening.clone(), tol)?;
    body.validate(tol)?;
    let display = body.tessellate_bounded(x[4], 65536, tol)?;
    let mass = body.mass_properties(tol)?;
    let bounds = body.bounds()?;
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
    let boundary_samples = body
        .brep()
        .edges
        .iter()
        .map(|e| match &e.curve {
            Curve::Nurbs(c) => Ok(c
                .tessellate_bounded(x[4], 65536)?
                .points
                .iter()
                .flat_map(|p| xyz(*p))
                .collect::<Vec<_>>()),
            Curve::Line { a, b } => Ok(vec![a.x, a.y, a.z, b.x, b.y, b.z]),
            _ => Err(Error::Unsupported(
                "polygon opening demo boundary curve unsupported",
            )),
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(serde_json::json!({"outer_polygon":body.outer_polygon(),"opening":body.opening(),"genus":1,"scope":"one strictly convex CCW polygon through opening in convex graph stock; polygon STEP import, inertia and generic Booleans remain unsupported","width":x[0],"depth":x[1],"height":x[2],"bulge":x[3],"error":x[4],"source_domain":source.source_domain(),"placement":{"angle":x[5],"translation":[x[6],x[7],x[8]],"axis":[0.,1.,0.]},"volume":mass.volume,"source_volume":outer.volume()?,"removed_volume":removed_volume,"mass_properties":{"volume":mass.volume,"centroid":xyz(mass.centroid),"units":"mm","volume_units":"mm3","centroid_units":"mm","density":"uniform","method":"polynomial fan integration"},"bounds_kind":"conservative control hull bounds","bounds":{"min":xyz(bounds.min),"max":xyz(bounds.max)},"positions":positions,"normals":normals,"boundary_samples":boundary_samples,"mesh":{"positions":display.mesh.positions.iter().map(|p|xyz(*p)).collect::<Vec<_>>(),"normals":display.mesh.normals.iter().map(|n|[n.x,n.y,n.z]).collect::<Vec<_>>(),"triangles":display.mesh.triangles,"face_ids":display.mesh.face_ids},"vertex_nodes":display.vertex_nodes,"vertex_uv":display.vertex_uv,"vertex_faces":display.vertex_faces,"error_bounds":display.error_bounds,"subdivisions":display.subdivisions,"brep":crate::nurbs_graph_polygon_demo::serialize_graph_brep(body.brep())?}).to_string())
}
