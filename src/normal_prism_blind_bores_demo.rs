//! Multiple disjoint exact flat-bottom cavities in certified rounded stock.
use crate::*;
/// Ten stock/placement/tolerance values followed by five values per pocket.
pub fn normal_prism_blind_bores_demo_json(x: &[f64]) -> Result<String> {
    if x.len() < 15
        || x.len() > 90
        || !(x.len() - 10).is_multiple_of(5)
        || x.iter().any(|v| !v.is_finite())
    {
        return Err(Error::InvalidInput(
            "multi blind bore requires 10 header values and 1..16 finite five-value pockets",
        ));
    }
    let tolerance = GeometryTolerance::new(x[8], GeometryTolerance::default().angular(), 0.)?;
    let tol = tolerance.absolute();
    let raw = make_box(
        BoxSpec {
            min: Point3::new(-x[0] / 2., -x[1] / 2., 0.),
            size: Vec3::new(x[0], x[1], x[2]),
        },
        tol,
    )?;
    let rounded = fillet_parallel_box_edges(
        &raw,
        &[(8, x[3]), (9, x[3]), (10, x[3]), (11, x[3])],
        tolerance,
    )?;
    let pose = Transform::translation(Vec3::new(x[5], x[6], x[7]))?
        .compose(Transform::rotation(Vec3::new(0., 1., 0.), x[4])?)?;
    let source = rounded.solid().transformed(pose, tol)?;
    let axis = pose.vector(Vec3::new(0., 0., 1.));
    let specs: Vec<_> = x[10..]
        .as_chunks::<5>()
        .0
        .iter()
        .map(|p| {
            let entry = match p[4] {
                0. => NormalPrismBoreEntry::Positive,
                1. => NormalPrismBoreEntry::Negative,
                _ => return Err(Error::InvalidInput("blind pocket entry must be 0 or 1")),
            };
            Ok(NormalPrismBlindBoreSpec {
                center: pose.point(Point3::new(p[0], p[1], if p[4] == 0. { x[2] } else { 0. })),
                radius: p[2],
                depth: p[3],
                entry,
            })
        })
        .collect::<Result<_>>()?;
    let result = blind_bores_normal_prism(&source, &specs, axis, tolerance)?;
    let removed = result
        .removed()
        .iter()
        .map(|s| crate::edge_fillet_demo::solid_json(s, x[9], tol))
        .collect::<Result<Vec<_>>>()?;
    let removed_steps = result
        .removed()
        .iter()
        .map(|s| export_step_bounded_analytic_mm(s, x[8]))
        .collect::<Result<Vec<_>>>()?;
    let pockets:Vec<_> = specs.iter().enumerate().map(|(i,s)| {
        let p=&x[10+i*5..15+i*5];
        serde_json::json!({"center":[p[0],p[1]],"world_center":[s.center.x,s.center.y,s.center.z],
            "radius":s.radius,"depth":s.depth,"entry":if p[4]==0. {"positive"} else {"negative"},
            "hole_faces":result.hole_faces()[i],"floor_face":result.floor_faces()[i]})
    }).collect();
    Ok(serde_json::json!({
        "units":"mm","scope":"1..16 strictly projection-disjoint flat-bottom pockets on certified normal stock; source already containing blind floors, overlapping projected tools, and workflow nodes remain unsupported; bounded analytic STEP import certifies supported batch results",
        "source":crate::edge_fillet_demo::solid_json(&source,x[9],tol)?,
        "kept":crate::edge_fillet_demo::solid_json(result.kept(),x[9],tol)?,
        "removed":removed,"removed_volume":result.direct_removed_volume(),
        "bore":{"axis":[axis.x,axis.y,axis.z],"pockets":pockets},
        "source_step":export_step_bounded_analytic_mm(&source,x[8])?,
        "kept_step":export_step_bounded_analytic_mm(result.kept(),x[8])?,
        "removed_steps":removed_steps,"step_exact":true,"step_schema":"AUTOMOTIVE_DESIGN",
        "stock":{"width":x[0],"depth":x[1],"height":x[2],"corner_radius":x[3]},
        "placement":{"angle":x[4],"translation":[x[5],x[6],x[7]],"axis":[0.,1.,0.]},
        "linear_tolerance":x[8],"display_chord_tolerance":x[9]
    }).to_string())
}
