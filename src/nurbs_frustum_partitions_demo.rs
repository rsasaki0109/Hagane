//! Display and STEP serialization of actual certified batch partition bodies.
use crate::*;

/// Nine frustum model/display values followed by one to sixteen ordered local cuts.
pub fn nurbs_frustum_partitions_demo_json(x: &[f64]) -> Result<String> {
    if !(10..=25).contains(&x.len()) || x.iter().any(|v| !v.is_finite()) {
        return Err(Error::InvalidInput(
            "frustum partitions require ten to twenty-five finite values",
        ));
    }
    let (source, policy) = crate::nurbs_frustum_demo::query_body(x)?;
    let partition = source.split_axial_many(&x[9..], policy)?;
    let parts = partition
        .parts
        .iter()
        .map(|body| crate::nurbs_frustum_demo::serialize_frustum(body, policy, x[8], x[3]))
        .collect::<Result<Vec<_>>>()?;
    let sections = partition.sections.iter().map(|section| {
        let curves = section.iter().map(|curve| {
            let Curve::Nurbs(c) = curve else {
                return Err(Error::InvalidTopology("frustum section must retain rational rim curves"));
            };
            Ok(serde_json::json!({"degree":c.degree(),"knots":c.knots(),"weights":c.weights(),"control_points":c.control_points().iter().map(|p|[p.x,p.y,p.z]).collect::<Vec<_>>()}))
        }).collect::<Result<Vec<_>>>()?;
        Ok(serde_json::json!({"curves":curves}))
    }).collect::<Result<Vec<_>>>()?;
    Ok(serde_json::json!({"units":"mm","parent_volume":source.volume(policy)?,"cut_heights":partition.cut_heights,"parts":parts,"sections":sections,"scope":"one to sixteen strictly increasing interior local axial cuts in a validated positive-radius rational frustum; actual closed partition bodies and original-source restriction certificates; oblique/general NURBS partitions unsupported"}).to_string())
}
