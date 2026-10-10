//! Actual isolated planar-edge chamfer geometry, exact STEP and native/WASM display.
use crate::*;
use serde_json::Value;

pub fn edge_chamfer_demo_json(x: &[f64]) -> Result<String> {
    if x.len() != 10
        || x.iter().any(|v| !v.is_finite())
        || x[3] < 0.
        || x[3].fract() != 0.
        || x[3] >= 12.
    {
        return Err(Error::InvalidInput(
            "box chamfer needs ten finite values and an integer edge index in 0..12",
        ));
    }
    let tol = GeometryTolerance::new(x[9], GeometryTolerance::default().angular(), 0.)?;
    let placement = Transform::translation(Vec3::new(x[6], x[7], x[8]))?
        .compose(Transform::rotation(Vec3::new(0., 1., 0.), x[5])?)?;
    let source = make_box(
        BoxSpec {
            min: Point3::new(0., 0., 0.),
            size: Vec3::new(x[0], x[1], x[2]),
        },
        tol.absolute(),
    )?
    .transformed(placement, tol.absolute())?;
    let result = chamfer_straight_convex_edge(&source, x[3] as usize, x[4], tol)?;
    let solid_json = |solid: &Solid| -> Result<Value> {
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
    };
    let xyz = |p: Point3| [p.x, p.y, p.z];
    let plane_json = |surface: &Surface| -> Result<Value> {
        let Surface::Plane { origin, u, v } = surface else {
            return Err(Error::Unsupported("chamfer demo requires a planar bevel"));
        };
        Ok(
            serde_json::json!({"kind":"plane","origin":xyz(*origin),"u":[u.x,u.y,u.z],"v":[v.x,v.y,v.z]}),
        )
    };
    Ok(serde_json::json!({
        "units":"mm","scope":"one isolated equal-setback straight convex edge chamfer on a planar box; interacting cuts, curved faces and fillets remain unsupported",
        "source":solid_json(&source)?,"kept":solid_json(result.solid())?,"removed":solid_json(result.removed())?,
        "chamfer":{"edge_index":result.source_edge(),"setback":result.setback(),"bevel_plane":plane_json(result.bevel_plane())?,"bevel_patch":{"surface":plane_json(&result.bevel().surface)?,"orientation":result.bevel().orientation,"rings":result.bevel().rings.iter().map(|ring|ring.iter().map(|p|xyz(*p)).collect::<Vec<_>>()).collect::<Vec<_>>()},"setback_kind":"equal physical distance on both incident faces"},
        "placement":{"angle":x[5],"translation":[x[6],x[7],x[8]],"axis":[0.,1.,0.]},"linear_tolerance":x[9],
        "step":export_step_mm(result.solid(),tol.absolute())?,"step_exact":true,"step_schema":"AUTOMOTIVE_DESIGN"
    }).to_string())
}
