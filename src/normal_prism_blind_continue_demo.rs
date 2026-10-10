//! STEP import followed by exact additional blind-pocket machining.
use crate::*;

/// Signed world axis, linear/display tolerances, then six values per new pocket:
/// world XYZ entry center, radius, depth, and entry (0 positive, 1 negative).
pub fn normal_prism_blind_continue_demo_json(input_step: &str, x: &[f64]) -> Result<String> {
    if x.len() < 11
        || x.len() > 101
        || !(x.len() - 5).is_multiple_of(6)
        || x.iter().any(|v| !v.is_finite())
    {
        return Err(Error::InvalidInput(
            "continued blind bore requires 5 header values and 1..16 finite six-value pockets",
        ));
    }
    let tolerance = GeometryTolerance::new(x[3], GeometryTolerance::default().angular(), 0.)?;
    let tol = tolerance.absolute();
    let source = import_step_bounded_analytic_mm(input_step, tol)?;
    let raw_axis = Vec3::new(x[0], x[1], x[2]);
    let scale = x[0].abs().max(x[1].abs()).max(x[2].abs());
    if scale == 0. {
        return Err(Error::InvalidInput(
            "continued blind bore axis must be nonzero",
        ));
    }
    let axis = Vec3::new(x[0] / scale, x[1] / scale, x[2] / scale).normalized()?;
    let specs: Vec<_> = x[5..]
        .as_chunks::<6>()
        .0
        .iter()
        .map(|p| {
            Ok(NormalPrismBlindBoreSpec {
                center: Point3::new(p[0], p[1], p[2]),
                radius: p[3],
                depth: p[4],
                entry: match p[5] {
                    0. => NormalPrismBoreEntry::Positive,
                    1. => NormalPrismBoreEntry::Negative,
                    _ => return Err(Error::InvalidInput("blind pocket entry must be 0 or 1")),
                },
            })
        })
        .collect::<Result<_>>()?;
    let result = append_blind_bores_normal_prism(&source, &specs, raw_axis, tolerance)?;
    let removed = result
        .removed()
        .iter()
        .map(|s| crate::edge_fillet_demo::solid_json(s, x[4], tol))
        .collect::<Result<Vec<_>>>()?;
    let removed_steps = result
        .removed()
        .iter()
        .map(|s| export_step_bounded_analytic_mm(s, x[3]))
        .collect::<Result<Vec<_>>>()?;
    let pockets: Vec<_> = specs.iter().enumerate().map(|(i, s)| {
        serde_json::json!({"world_center":[s.center.x,s.center.y,s.center.z],
            "radius":s.radius,"depth":s.depth,
            "entry":match s.entry { NormalPrismBoreEntry::Positive=>"positive",NormalPrismBoreEntry::Negative=>"negative" },
            "hole_faces":result.hole_faces()[i],"floor_face":result.floor_faces()[i]})
    }).collect();
    Ok(serde_json::json!({
        "units":"mm",
        "scope":"additional projection-disjoint blind pockets on uniform or certified blind normal stock; all original geometry is retained; at most 16 total pockets/openings and 128 profile segments; overlapping projected tools, subsequent plane cuts and workflow nodes remain unsupported",
        "source":crate::edge_fillet_demo::solid_json(&source,x[4],tol)?,
        "kept":crate::edge_fillet_demo::solid_json(result.kept(),x[4],tol)?,
        "removed":removed,"removed_volume":result.direct_removed_volume(),
        "bore":{"axis":[axis.x,axis.y,axis.z],"pockets":pockets},
        "source_step":export_step_bounded_analytic_mm(&source,x[3])?,
        "kept_step":export_step_bounded_analytic_mm(result.kept(),x[3])?,
        "removed_steps":removed_steps,"step_exact":true,"step_schema":"AUTOMOTIVE_DESIGN",
        "linear_tolerance":x[3],"display_chord_tolerance":x[4]
    }).to_string())
}
