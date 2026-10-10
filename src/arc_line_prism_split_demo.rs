//! Exact axial-plane partition of checked normal line/arc prism stock.
use crate::*;
pub fn arc_line_prism_split_demo_json(x: &[f64]) -> Result<String> {
    if x.len() != 12 || x.iter().any(|v| !v.is_finite()) {
        return Err(Error::InvalidInput(
            "normal arc/line prism split needs exactly 12 finite values",
        ));
    }
    let tolerance = GeometryTolerance::new(x[10], GeometryTolerance::default().angular(), 0.)?;
    let tol = tolerance.absolute();
    let source = make_box(
        BoxSpec {
            min: Point3::new(-x[0] / 2., -x[1] / 2., 0.),
            size: Vec3::new(x[0], x[1], x[2]),
        },
        tol,
    )?;
    let rounded = fillet_parallel_box_edges(
        &source,
        &[(8, x[3]), (9, x[3]), (10, x[3]), (11, x[3])],
        tolerance,
    )?;
    let pose = Transform::translation(Vec3::new(x[7], x[8], x[9]))?
        .compose(Transform::rotation(Vec3::new(0., 1., 0.), x[6])?)?;
    let source = rounded.solid().transformed(pose, tol)?;
    let (s, c) = x[5].sin_cos();
    let local_normal = Vec3::new(c, s, 0.);
    let origin = pose.point(local_normal * x[4]);
    let u = pose.vector(Vec3::new(0., 0., 1.));
    let v = pose.vector(Vec3::new(s, -c, 0.));
    let plane = Surface::Plane { origin, u, v };
    let normal = u.cross(v).normalized()?;
    let result = split_normal_arc_line_prism_by_plane(&source, &plane, tolerance)?;
    let patch = result.section();
    let Surface::Plane {
        origin: section_origin,
        u: section_u,
        v: section_v,
    } = patch.surface
    else {
        return Err(Error::Unsupported(
            "normal prism split section must be planar",
        ));
    };
    Ok(serde_json::json!({
        "units":"mm","scope":"one axial plane partition of a certified normal line/arc prism with two resolved outer crossings and no opening crossing; contact, skew planes, disconnected sections and general Booleans remain unsupported",
        "source":crate::edge_fillet_demo::solid_json(&source,x[11],tol)?,"negative":crate::edge_fillet_demo::solid_json(result.negative(),x[11],tol)?,"positive":crate::edge_fillet_demo::solid_json(result.positive(),x[11],tol)?,
        "cut":{"offset":x[4],"angle":x[5],"normal":[normal.x,normal.y,normal.z],"plane":{"kind":"plane","origin":[origin.x,origin.y,origin.z],"u":[u.x,u.y,u.z],"v":[v.x,v.y,v.z]},"section":{"surface":{"kind":"plane","origin":[section_origin.x,section_origin.y,section_origin.z],"u":[section_u.x,section_u.y,section_u.z],"v":[section_v.x,section_v.y,section_v.z]},"orientation":patch.orientation,"rings":patch.rings.iter().map(|ring|ring.iter().map(|p|[p.x,p.y,p.z]).collect::<Vec<_>>()).collect::<Vec<_>>()},"negative_halfspace":"signed distance <= 0","positive_halfspace":"signed distance >= 0"},
        "stock":{"width":x[0],"depth":x[1],"height":x[2],"corner_radius":x[3]},"placement":{"angle":x[6],"translation":[x[7],x[8],x[9]],"axis":[0.,1.,0.]},"linear_tolerance":x[10],"display_chord_tolerance":x[11],
        "negative_step":export_step_bounded_analytic_mm(result.negative(),x[10])?,"positive_step":export_step_bounded_analytic_mm(result.positive(),x[10])?,"step_exact":true,"step_schema":"AUTOMOTIVE_DESIGN"
    }).to_string())
}
