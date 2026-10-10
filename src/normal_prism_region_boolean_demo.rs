//! Exact prismatic partition crossing actual cylindrical stock walls.
use crate::*;

/// Stock W/D/H/corner radius; annular tool outer/inner radius, center XY/phase; shared Y
/// pose angle/translation XYZ; linear/display tolerances. Angles are radians.
pub fn normal_prism_region_boolean_demo_json(x: &[f64]) -> Result<String> {
    if x.len() != 15 || x.iter().any(|v| !v.is_finite()) {
        return Err(Error::InvalidInput(
            "prism region Boolean requires exactly 15 finite values",
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
    let outer = (0..4)
        .map(|i| PlanarSegment::Arc {
            center: [x[6], x[7]],
            radius: x[4],
            start_angle: x[8] + i as f64 * std::f64::consts::FRAC_PI_2,
            sweep: std::f64::consts::FRAC_PI_2,
        })
        .collect();
    let raw_tool = extrude_arc_line_region(
        &ArcLineRegion {
            origin: Point3::new(0., 0., 0.),
            outer,
            holes: vec![(0..4)
                .map(|i| PlanarSegment::Arc {
                    center: [x[6], x[7]],
                    radius: x[5],
                    start_angle: x[8] - i as f64 * std::f64::consts::FRAC_PI_2,
                    sweep: -std::f64::consts::FRAC_PI_2,
                })
                .collect()],
        },
        x[2],
        tol,
    )?;
    let pose = Transform::translation(Vec3::new(x[10], x[11], x[12]))?
        .compose(Transform::rotation(Vec3::new(0., 1., 0.), x[9])?)?;
    let source = rounded.solid().transformed(pose, tol)?;
    let tool = raw_tool.transformed(pose, tol)?;
    let axis = pose.vector(Vec3::new(0., 0., 1.));
    let result = boolean_normal_prism_regions(&source, &tool, axis, policy)?;
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
        "scope":"exact same-axis, same-cap-interval normal-prism partition by a line/subquarter-circle tool with holes; bounded line/subquarter-circle source; transverse resolved crossings only; contacts, tangency, vertex/coincident overlays, skew tools and general 3D Booleans remain unsupported",
        "source":crate::edge_fillet_demo::solid_json(&source,x[14],tol)?,
        "tool":crate::edge_fillet_demo::solid_json(&tool,x[14],tol)?,
        "difference":bodies(result.difference())?,
        "intersection":bodies(result.intersection())?,
        "union":bodies(result.union())?,
        "source_step":export_step_bounded_analytic_mm(&source,x[13])?,
        "tool_step":export_step_bounded_analytic_mm(&tool,x[13])?,
        "difference_steps":steps(result.difference())?,
        "intersection_steps":steps(result.intersection())?,
        "union_steps":steps(result.union())?,
        "stock":{"width":x[0],"depth":x[1],"height":x[2],"corner_radius":x[3]},
        "tool_spec":{"outer_radius":x[4],"inner_radius":x[5],"center":[x[6],x[7]],"phase":x[8]},
        "placement":{"angle":x[9],"translation":[x[10],x[11],x[12]],"axis":[0.,1.,0.]},
        "axis":[axis.x,axis.y,axis.z],
        "linear_tolerance":x[13],"display_chord_tolerance":x[14],
        "step_exact":true,"step_schema":"AUTOMOTIVE_DESIGN"
    }).to_string())
}
