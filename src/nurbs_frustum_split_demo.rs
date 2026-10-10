//! Display and STEP serialization of actual certified axial split children.
use crate::*;

/// Nine frustum model/display values, followed by local axial cut height.
pub fn nurbs_frustum_split_demo_json(x: &[f64]) -> Result<String> {
    if x.len() != 10 || x.iter().any(|v| !v.is_finite()) {
        return Err(Error::InvalidInput(
            "axial frustum split requires exactly ten finite values",
        ));
    }
    let (source, policy) = crate::nurbs_frustum_demo::query_body(x)?;
    let split = source.split_axial(x[9], policy)?;
    let lower = crate::nurbs_frustum_demo::serialize_frustum(&split.lower, policy, x[8], x[3])?;
    let upper = crate::nurbs_frustum_demo::serialize_frustum(&split.upper, policy, x[8], x[3])?;
    let curves = split.section.iter().map(|curve| {
        let Curve::Nurbs(c) = curve else { return Err(Error::InvalidTopology("frustum section must retain rational rim curves")); };
        Ok(serde_json::json!({"degree":c.degree(),"knots":c.knots(),"weights":c.weights(),"control_points":c.control_points().iter().map(|p|[p.x,p.y,p.z]).collect::<Vec<_>>()}))
    }).collect::<Result<Vec<_>>>()?;
    Ok(serde_json::json!({"units":"mm","cut_height":split.cut_height,"cut_radius":split.radius,"parent_volume":source.volume(policy)?,"lower":lower,"upper":upper,"section":{"curves":curves},"scope":"strictly interior local axial plane in validated positive-radius rational frustum; two closed children, actual restricted-support certificate; oblique/general NURBS splits unsupported"}).to_string())
}
