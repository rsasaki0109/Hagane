//! Actual multi-edge planar box chamfers, including resolved unequal crossing cuts.
use crate::*;
use serde_json::Value;
fn solid_json(solid: &Solid, tol: GeometryTolerance) -> Result<Value> {
    solid.validate(tol.absolute())?;
    let mesh = solid.tessellate(0.05, tol.absolute())?;
    let xyz = |p: Point3| [p.x, p.y, p.z];
    let positions: Vec<_> = mesh
        .triangles
        .iter()
        .flat_map(|t| t.iter().flat_map(|i| xyz(mesh.positions[*i])))
        .collect();
    let normals: Vec<_> = mesh
        .triangles
        .iter()
        .flat_map(|t| {
            t.iter().flat_map(|i| {
                let n = mesh.normals[*i];
                [n.x, n.y, n.z]
            })
        })
        .collect();
    let bounds = solid.bounds();
    Ok(
        serde_json::json!({"volume":solid.volume()?,"bounds":{"min":xyz(bounds.min),"max":xyz(bounds.max)},"positions":positions,"normals":normals,"mesh":{"positions":mesh.positions.iter().map(|p|xyz(*p)).collect::<Vec<_>>(),"normals":mesh.normals.iter().map(|n|[n.x,n.y,n.z]).collect::<Vec<_>>(),"triangles":mesh.triangles,"face_ids":mesh.face_ids},"brep":crate::nurbs_graph_polygon_demo::serialize_graph_brep(solid)?}),
    )
}

fn plane_json(surface: &Surface) -> Result<Value> {
    let Surface::Plane { origin, u, v } = surface else {
        return Err(Error::Unsupported("multi chamfer needs a plane"));
    };
    Ok(
        serde_json::json!({"kind":"plane","origin":[origin.x,origin.y,origin.z],"u":[u.x,u.y,u.z],"v":[v.x,v.y,v.z]}),
    )
}
pub fn edge_chamfer_multi_demo_json(x: &[f64]) -> Result<String> {
    edge_chamfer_mode_demo_json(x, false)
}

pub(crate) fn edge_chamfer_mode_demo_json(x: &[f64], vertex_contacts: bool) -> Result<String> {
    if !(11..=33).contains(&x.len())
        || x.iter().any(|v| !v.is_finite())
        || x[8] < 1.
        || x[8] > 12.
        || x[8].fract() != 0.
        || x.len() != 9 + 2 * x[8] as usize
    {
        return Err(Error::InvalidInput("multi chamfer needs nine finite model/count values and exactly 1..12 edge/setback pairs"));
    }
    let mut selections = Vec::new();
    for pair in x[9..].as_chunks::<2>().0 {
        if pair[0] < 0. || pair[0] >= 12. || pair[0].fract() != 0. {
            return Err(Error::InvalidInput(
                "multi chamfer box edge index must be an integer in 0..12",
            ));
        }
        selections.push((pair[0] as usize, pair[1]));
    }
    let tol = GeometryTolerance::new(x[7], GeometryTolerance::default().angular(), 0.)?;
    let placement = Transform::translation(Vec3::new(x[4], x[5], x[6]))?
        .compose(Transform::rotation(Vec3::new(0., 1., 0.), x[3])?)?;
    let source = make_box(
        BoxSpec {
            min: Point3::new(0., 0., 0.),
            size: Vec3::new(x[0], x[1], x[2]),
        },
        tol.absolute(),
    )?
    .transformed(placement, tol.absolute())?;
    let result = if vertex_contacts {
        chamfer_straight_convex_edges_with_vertex_contacts(&source, &selections, tol)?
    } else {
        chamfer_straight_convex_edges(&source, &selections, tol)?
    };
    let removed = result
        .removed()
        .iter()
        .map(|s| solid_json(s, tol))
        .collect::<Result<Vec<_>>>()?;
    let removed_volume = result
        .removed()
        .iter()
        .map(Solid::volume)
        .collect::<Result<Vec<_>>>()?
        .iter()
        .sum::<f64>();
    let bevels=result.bevels().iter().map(|patch|Ok(serde_json::json!({"surface":plane_json(&patch.surface)?,"orientation":patch.orientation,"rings":patch.rings.iter().map(|ring|ring.iter().map(|p|[p.x,p.y,p.z]).collect::<Vec<_>>()).collect::<Vec<_>>()}))).collect::<Result<Vec<_>>>()?;
    let mut report = serde_json::json!({"units":"mm","scope":"1..12 original planar box edges with equal setback per edge; resolved unequal crossing cuts supported, contact, unresolved intersections, curved faces and fillets rejected","source":solid_json(&source,tol)?,"kept":solid_json(result.solid(),tol)?,"removed":removed,"removed_volume":removed_volume,"chamfers":{"selections":result.selections(),"bevel_planes":result.bevel_planes().iter().map(plane_json).collect::<Result<Vec<_>>>()?,"bevel_patches":bevels,"setback_kind":"equal physical setback on each selected original edge","patch_kind":"actual final clipped bevel faces"},"placement":{"angle":x[3],"translation":[x[4],x[5],x[6]],"axis":[0.,1.,0.]},"linear_tolerance":x[7],"step":export_step_mm(result.solid(),tol.absolute())?,"step_exact":true,"step_schema":"AUTOMOTIVE_DESIGN"});
    if vertex_contacts {
        report["mode"] = serde_json::json!("vertex_contacts");
        report["scope"]=serde_json::json!("1..12 original planar box edge chamfers with resolved vertex contacts; exact supporting planes with conservative floating-point admission, no coordinate snapping; unresolved conditioning, curved faces and fillets rejected");
    }
    Ok(report.to_string())
}
