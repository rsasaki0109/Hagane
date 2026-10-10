//! Scoped exact normal circular through-bore in retained rounded line/arc prism stock.
use crate::*;
pub fn arc_line_prism_bore_demo_json(x: &[f64]) -> Result<String> {
    if x.len() != 13 || x.iter().any(|v| !v.is_finite()) {
        return Err(Error::InvalidInput(
            "normal arc/line prism bore needs exactly 13 finite values",
        ));
    }
    let tolerance = GeometryTolerance::new(x[11], GeometryTolerance::default().angular(), 0.)?;
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
    let pose = Transform::translation(Vec3::new(x[8], x[9], x[10]))?
        .compose(Transform::rotation(Vec3::new(0., 1., 0.), x[7])?)?;
    let source = rounded.solid().transformed(pose, tol)?;
    let center = pose.point(Point3::new(x[4], x[5], 0.));
    let axis = pose.vector(Vec3::new(0., 0., 1.));
    let result = bore_normal_arc_line_prism(&source, center, x[6], tolerance)?;
    Ok(serde_json::json!({
        "units":"mm","scope":"one normal circular through bore strictly inside a certified normal line/arc prism; general booleans, skew axes, contact and unresolved precision remain unsupported",
        "source":crate::edge_fillet_demo::solid_json(&source,x[12],tol)?,"kept":crate::edge_fillet_demo::solid_json(result.kept(),x[12],tol)?,"removed":crate::edge_fillet_demo::solid_json(result.removed(),x[12],tol)?,
        "removed_volume":result.direct_removed_volume(),"bore":{"center":[x[4],x[5]],"world_center":[center.x,center.y,center.z],"axis":[axis.x,axis.y,axis.z],"radius":x[6],"hole_faces":result.hole_faces(),"mode":"normal_through"},
        "stock":{"width":x[0],"depth":x[1],"height":x[2],"corner_radius":x[3]},"placement":{"angle":x[7],"translation":[x[8],x[9],x[10]],"axis":[0.,1.,0.]},"linear_tolerance":x[11],"display_chord_tolerance":x[12],
        "kept_step":export_step_bounded_analytic_mm(result.kept(),x[11])?,"removed_step":export_step_bounded_analytic_mm(result.removed(),x[11])?,"step_exact":true,"step_schema":"AUTOMOTIVE_DESIGN"
    }).to_string())
}
