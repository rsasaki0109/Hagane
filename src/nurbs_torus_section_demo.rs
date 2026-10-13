//! Actual horizontal torus section curves; no solid or accepted-state modification.
use crate::*;
pub(crate) fn serialize_section(
    source: &NurbsTorusSolid,
    height: f64,
    chord: f64,
    policy: GeometryTolerance,
) -> Result<serde_json::Value> {
    if !height.is_finite() || !chord.is_finite() || chord <= 0. {
        return Err(Error::InvalidInput(
            "torus section height must be finite and chord positive finite",
        ));
    }
    let section = source.section_by_local_z(height, policy)?;
    section.validate(policy)?;
    let xyz = |p: Point3| [p.x, p.y, p.z];
    let geometry = |c: &NurbsCurve| serde_json::json!({"kind":"nurbs","degree":c.degree(),"knots":c.knots(),"weights":c.weights(),"control_points":c.control_points().iter().copied().map(xyz).collect::<Vec<_>>(),"parameter_range":c.domain()});
    let pcurve = |pc: &PCurve| -> Result<serde_json::Value> {
        match pc {
            PCurve::Affine { origin, direction } => {
                Ok(serde_json::json!({"kind":"affine","origin":origin,"direction":direction}))
            }
            PCurve::Nurbs(c) => Ok(geometry(c)),
            _ => Err(Error::Unsupported(
                "torus section pcurve representation unsupported",
            )),
        }
    };
    let mut curves = Vec::with_capacity(8);
    for (i, edge) in section.edges().iter().enumerate() {
        let Curve::Nurbs(curve) = &edge.curve else {
            return Err(Error::InvalidTopology(
                "torus section requires actual rational primary curves",
            ));
        };
        let display = curve.tessellate_bounded(chord, 16384)?;
        let uses = section
            .source_uses()
            .iter()
            .filter(|u| u.edge == i)
            .map(|u| Ok(serde_json::json!({"face":u.face,"pcurve":pcurve(&u.pcurve)?})))
            .collect::<Result<Vec<_>>>()?;
        curves.push(serde_json::json!({"vertices":edge.vertices,"curve":geometry(curve),"source_uses":uses,"points":display.points.iter().copied().map(xyz).collect::<Vec<_>>(),"parameters":display.parameters,"error_bounds":display.error_bounds}));
    }
    let loops=section.loops().iter().enumerate().map(|(i,w)|{let coedges=w.coedges.iter().map(|c|Ok(serde_json::json!({"edge":c.edge,"forward":c.forward,"pcurve":pcurve(&c.pcurve)?}))).collect::<Result<Vec<_>>>()?;Ok(serde_json::json!({"coedges":coedges,"orientation":"increasing source major U","role":if i==0 {"outer"}else{"inner"}}))}).collect::<Result<Vec<_>>>()?;
    let Surface::Plane { origin, u, v } = section.plane() else {
        return Err(Error::InvalidTopology(
            "torus horizontal section plane missing",
        ));
    };
    let frame = source.frame();
    Ok(
        serde_json::json!({"kind":"nurbs_torus_horizontal_section","units":"mm","cut_height":height,"source_radii":source.radii(),"source_placement":{"translation":xyz(frame.origin()),"axes":frame.axes().map(|a|[a.x,a.y,a.z])},"plane":{"origin":xyz(*origin),"u":[u.x,u.y,u.z],"v":[v.x,v.y,v.z]},"vertices":section.vertices().iter().map(|v|xyz(v.point)).collect::<Vec<_>>(),"curves":curves,"loops":loops,"display_kind":"actual rational curve bounded polylines","display_chord_tolerance":chord,"closed":true,"mesh_free":true,"stock_unchanged":true,"cut_created":false,"scope":"two exact closed rational horizontal section loops with all retained source-face pcurves and bounded curve polylines; both loops follow increasing major U, not annular-face orientation; no new solid or section STEP export"}),
    )
}
