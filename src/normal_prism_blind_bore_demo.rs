//! A flat-bottom circular pocket in actual certified rounded normal stock.
use crate::*;

/// W, D, H, corner radius, X, Y, bore radius, depth, entry (0 positive/1 negative),
/// Y-rotation, translation XYZ, linear tolerance and display chord tolerance.
pub fn normal_prism_blind_bore_demo_json(x: &[f64]) -> Result<String> {
    if x.len() != 15 || x.iter().any(|v| !v.is_finite()) {
        return Err(Error::InvalidInput(
            "normal blind bore requires exactly 15 finite values",
        ));
    }
    let entry = match x[8] {
        0. => NormalPrismBoreEntry::Positive,
        1. => NormalPrismBoreEntry::Negative,
        _ => return Err(Error::InvalidInput("blind bore entry must be 0 or 1")),
    };
    let tolerance = GeometryTolerance::new(x[13], GeometryTolerance::default().angular(), 0.)?;
    let tol = tolerance.absolute();
    let box_stock = make_box(
        BoxSpec {
            min: Point3::new(-x[0] / 2., -x[1] / 2., 0.),
            size: Vec3::new(x[0], x[1], x[2]),
        },
        tol,
    )?;
    let rounded = fillet_parallel_box_edges(
        &box_stock,
        &[(8, x[3]), (9, x[3]), (10, x[3]), (11, x[3])],
        tolerance,
    )?;
    let pose = Transform::translation(Vec3::new(x[10], x[11], x[12]))?
        .compose(Transform::rotation(Vec3::new(0., 1., 0.), x[9])?)?;
    let source = rounded.solid().transformed(pose, tol)?;
    let center = pose.point(Point3::new(x[4], x[5], if x[8] == 0. { x[2] } else { 0. }));
    let axis = pose.vector(Vec3::new(0., 0., 1.));
    let result = blind_bore_normal_prism(&source, center, x[6], x[7], axis, entry, tolerance)?;
    Ok(serde_json::json!({
        "units":"mm", "scope":"one disjoint flat-bottom normal circular blind bore on certified normal prism stock; general booleans, repeated blind machining, blind-body STEP import and workflow nodes remain unsupported",
        "source":crate::edge_fillet_demo::solid_json(&source,x[14],tol)?,
        "kept":crate::edge_fillet_demo::solid_json(result.kept(),x[14],tol)?,
        "removed":crate::edge_fillet_demo::solid_json(result.removed(),x[14],tol)?,
        "removed_volume":result.direct_removed_volume(),
        "bore":{"center":[x[4],x[5]],"world_center":[center.x,center.y,center.z],
            "axis":[axis.x,axis.y,axis.z],"radius":x[6],"depth":x[7],
            "entry":if x[8]==0. {"positive"} else {"negative"},
            "hole_faces":result.hole_faces(),"floor_face":result.floor_face(),"mode":"normal_blind"},
        "stock":{"width":x[0],"depth":x[1],"height":x[2],"corner_radius":x[3]},
        "placement":{"angle":x[9],"translation":[x[10],x[11],x[12]],"axis":[0.,1.,0.]},
        "linear_tolerance":x[13],"display_chord_tolerance":x[14],
        "kept_step":export_step_bounded_analytic_mm(result.kept(),x[13])?,
        "removed_step":export_step_bounded_analytic_mm(result.removed(),x[13])?,
        "step_exact":true,"step_schema":"AUTOMOTIVE_DESIGN"
    }).to_string())
}
