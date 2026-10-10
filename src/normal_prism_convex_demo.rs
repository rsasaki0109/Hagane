//! Exact prismatic partition crossing actual cylindrical stock walls.
use crate::*;

/// Stock W/D/H/radius; rectangular tool W/D/center XY/angle; shared Y pose
/// angle/translation XYZ; linear/display tolerances. Angles are radians.
pub fn normal_prism_convex_partition_demo_json(x: &[f64]) -> Result<String> {
    if x.len() != 15 || x.iter().any(|v| !v.is_finite()) {
        return Err(Error::InvalidInput(
            "prism convex partition requires exactly 15 finite values",
        ));
    }
    let policy = GeometryTolerance::new(x[13], GeometryTolerance::default().angular(), 0.)?;
    let tol = policy.absolute();
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
        policy,
    )?;
    let raw_tool = make_box(
        BoxSpec {
            min: Point3::new(-x[4] / 2., -x[5] / 2., 0.),
            size: Vec3::new(x[4], x[5], x[2]),
        },
        tol,
    )?;
    let local_tool = Transform::translation(Vec3::new(x[6], x[7], 0.))?
        .compose(Transform::rotation(Vec3::new(0., 0., 1.), x[8])?)?;
    let pose = Transform::translation(Vec3::new(x[10], x[11], x[12]))?
        .compose(Transform::rotation(Vec3::new(0., 1., 0.), x[9])?)?;
    let source = rounded.solid().transformed(pose, tol)?;
    let tool = raw_tool.transformed(pose.compose(local_tool)?, tol)?;
    let axis = pose.vector(Vec3::new(0., 0., 1.));
    let result = partition_normal_prism_by_convex_tool(&source, &tool, axis, policy)?;
    let bodies = |solids: &[Solid]| -> Result<Vec<serde_json::Value>> {
        solids
            .iter()
            .map(|s| crate::edge_fillet_demo::solid_json(s, x[14], tol))
            .collect()
    };
    let steps = |solids: &[Solid]| -> Result<Vec<String>> {
        solids
            .iter()
            .map(|s| export_step_bounded_analytic_mm(s, x[13]))
            .collect()
    };
    Ok(serde_json::json!({
        "units":"mm",
        "scope":"exact same-axis, same-cap-interval normal-prism partition by a convex all-line tool; bounded line/subquarter-circle source; transverse resolved crossings only; contacts, tangency, vertex/coincident overlays, skew tools and general curved Booleans remain unsupported",
        "source":crate::edge_fillet_demo::solid_json(&source,x[14],tol)?,
        "tool":crate::edge_fillet_demo::solid_json(&tool,x[14],tol)?,
        "difference":bodies(result.difference())?,
        "intersection":bodies(result.intersection())?,
        "source_step":export_step_bounded_analytic_mm(&source,x[13])?,
        "tool_step":export_step_bounded_analytic_mm(&tool,x[13])?,
        "difference_steps":steps(result.difference())?,
        "intersection_steps":steps(result.intersection())?,
        "stock":{"width":x[0],"depth":x[1],"height":x[2],"corner_radius":x[3]},
        "tool_spec":{"width":x[4],"depth":x[5],"center":[x[6],x[7]],"angle":x[8]},
        "placement":{"angle":x[9],"translation":[x[10],x[11],x[12]],"axis":[0.,1.,0.]},
        "axis":[axis.x,axis.y,axis.z],
        "linear_tolerance":x[13],"display_chord_tolerance":x[14],
        "step_exact":true,"step_schema":"AUTOMOTIVE_DESIGN"
    }).to_string())
}
