//! Actual component partition across retained openings in a normal line/arc prism.
use crate::*;
use serde_json::Value;
pub fn arc_line_prism_split_components_demo_json(x: &[f64]) -> Result<String> {
    if x.len() != 15 || x.iter().any(|v| !v.is_finite()) {
        return Err(Error::InvalidInput(
            "normal line/arc component split needs exactly 15 finite values",
        ));
    }
    let tolerance = GeometryTolerance::new(x[13], GeometryTolerance::default().angular(), 0.)?;
    let tol = tolerance.absolute();
    let stock = make_box(
        BoxSpec {
            min: Point3::new(-x[0] / 2., -x[1] / 2., 0.),
            size: Vec3::new(x[0], x[1], x[2]),
        },
        tol,
    )?;
    let rounded = fillet_parallel_box_edges(
        &stock,
        &[(8, x[3]), (9, x[3]), (10, x[3]), (11, x[3])],
        tolerance,
    )?;
    let bore = bore_normal_arc_line_prism(
        rounded.solid(),
        Point3::new(x[4], x[5], 0.),
        x[6],
        tolerance,
    )?;
    let pose = Transform::translation(Vec3::new(x[10], x[11], x[12]))?
        .compose(Transform::rotation(Vec3::new(0., 1., 0.), x[9])?)?;
    let source = bore.kept().transformed(pose, tol)?;
    let (s, c) = x[8].sin_cos();
    let origin = pose.point(Vec3::new(c, s, 0.) * x[7]);
    let u = pose.vector(Vec3::new(0., 0., 1.));
    let v = pose.vector(Vec3::new(s, -c, 0.));
    let normal = u.cross(v).normalized()?;
    let plane = Surface::Plane { origin, u, v };
    let result = split_normal_arc_line_prism_by_plane_components(&source, &plane, tolerance)?;
    let reports = |solids: &[Solid]| {
        solids
            .iter()
            .map(|s| crate::edge_fillet_demo::solid_json(s, x[14], tol))
            .collect::<Result<Vec<_>>>()
    };
    let steps = |solids: &[Solid]| {
        solids
            .iter()
            .map(|s| export_step_bounded_analytic_mm(s, x[13]))
            .collect::<Result<Vec<_>>>()
    };
    let sections=result.sections().iter().map(|patch|{let Surface::Plane{origin,u,v}=patch.surface else{return Err(Error::Unsupported("component split section must be planar"));};Ok(serde_json::json!({"surface":{"kind":"plane","origin":[origin.x,origin.y,origin.z],"u":[u.x,u.y,u.z],"v":[v.x,v.y,v.z]},"orientation":patch.orientation,"rings":patch.rings.iter().map(|ring|ring.iter().map(|p|[p.x,p.y,p.z]).collect::<Vec<_>>()).collect::<Vec<_>>()}))}).collect::<Result<Vec<Value>>>()?;
    Ok(serde_json::json!({"units":"mm","scope":"actual component partition of a certified normal line/arc prism by one axial plane, including resolved crossings of openings; contact, skew planes and unresolved conditioning remain unsupported","source":crate::edge_fillet_demo::solid_json(&source,x[14],tol)?,"negative":reports(result.negative())?,"positive":reports(result.positive())?,"negative_steps":steps(result.negative())?,"positive_steps":steps(result.positive())?,"step_exact":true,"step_schema":"AUTOMOTIVE_DESIGN","cut":{"offset":x[7],"angle":x[8],"normal":[normal.x,normal.y,normal.z],"plane":{"kind":"plane","origin":[origin.x,origin.y,origin.z],"u":[u.x,u.y,u.z],"v":[v.x,v.y,v.z]},"sections":sections},"stock":{"width":x[0],"depth":x[1],"height":x[2],"corner_radius":x[3]},"bore":{"center":[x[4],x[5]],"radius":x[6]},"placement":{"angle":x[9],"translation":[x[10],x[11],x[12]],"axis":[0.,1.,0.]},"linear_tolerance":x[13],"display_chord_tolerance":x[14]}).to_string())
}
