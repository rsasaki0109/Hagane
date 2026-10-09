//! An exact rational roof path; the stock remains an unchanged solid.
use crate::*;

pub fn nurbs_graph_rational_roof_circle_demo_json(x: &[f64]) -> Result<String> {
    if x.len() != 16 || x.iter().any(|v| !v.is_finite()) {
        return Err(Error::InvalidInput(
            "roof circle needs exactly 16 finite values",
        ));
    }
    let tol = Tolerance::default();
    let source = crate::nurbs_graph_classification_demo::query_source(
        [x[0], x[1], x[2]],
        x[3],
        x[4],
        x[5],
        [x[6], x[7], x[8]],
        [[x[9], x[10]], [x[11], x[12]]],
    )?;
    let (cx, cy, r) = (x[13], x[14], x[15]);
    let scale = x.iter().fold(1_f64, |m, v| m.max(v.abs()));
    let guard = 4. * tol.linear + 4096. * f64::EPSILON * scale;
    if r <= guard
        || cx - r <= x[0] * x[9] + guard
        || cx + r >= x[0] * x[10] - guard
        || cy - r <= x[1] * x[11] + guard
        || cy + r >= x[1] * x[12] - guard
    {
        return Err(Error::InvalidInput(
            "roof circle needs resolved radius and strict source-trim clearance",
        ));
    }
    let (u, v, ru, rv) = (cx / x[0], cy / x[1], r / x[0], r / x[1]);
    let cardinal = [[u + ru, v], [u, v + rv], [u - ru, v], [u, v - rv]];
    let corners = [
        [u + ru, v + rv],
        [u - ru, v + rv],
        [u - ru, v - rv],
        [u + ru, v - rv],
    ];
    if cardinal.iter().any(|p| p[0] == u && p[1] == v) {
        return Err(Error::Unsupported("roof circle UV radius is unresolved"));
    }
    let Surface::Nurbs(surface) = &source.brep().shell.faces[1].surface else {
        return Err(Error::Unsupported(
            "roof circle needs a retained NURBS roof",
        ));
    };
    let xyz = |p: Point3| [p.x, p.y, p.z];
    let geometry = |c: &NurbsCurve| serde_json::json!({"degree":c.degree(),"knots":c.knots(),"weights":c.weights(),"control_points":c.control_points().iter().map(|p|xyz(*p)).collect::<Vec<_>>(),"parameter_range":[0.,1.]});
    let mut quarters = Vec::new();
    for i in 0..4 {
        let uv = NurbsCurve::new(
            2,
            vec![0., 0., 0., 1., 1., 1.],
            [cardinal[i], corners[i], cardinal[(i + 1) % 4]]
                .iter()
                .map(|p| Point3::new(p[0], p[1], 0.))
                .collect(),
            vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
        )?;
        let pcurve = PCurve::nurbs(uv.clone())?;
        let PCurve::Nurbs(retained_uv) = &pcurve else {
            unreachable!()
        };
        let curve = surface.parameter_curve_nurbs(retained_uv, tol)?;
        let line = curve.tessellate_bounded(x[4], 65536)?;
        let witnesses = (0..=16).map(|j| {
            let t=j as f64/16.;
            let uv=retained_uv.evaluate(t)?;
            Ok(serde_json::json!({"parameter":t,"uv":[uv.x,uv.y],"point":xyz(curve.evaluate(t)?),"surface_point":xyz(surface.evaluate(uv.x,uv.y)?)}))
        }).collect::<Result<Vec<_>>>()?;
        quarters.push(serde_json::json!({"curve":geometry(&curve),"pcurve":geometry(retained_uv),"points":line.points.iter().map(|p|xyz(*p)).collect::<Vec<_>>(),"parameters":line.parameters,"error_bounds":line.error_bounds,"witnesses":witnesses}));
    }
    let text = crate::nurbs_graph_demo::serialize_graph(&source, x[0], x[1], x[2], x[3], x[4])?;
    let mut data: serde_json::Value = serde_json::from_str(&text)
        .map_err(|_| Error::InvalidInput("roof path serialization failed"))?;
    data["placement"] =
        serde_json::json!({"angle":x[5],"translation":[x[6],x[7],x[8]],"axis":[0.,1.,0.]});
    data["bounds_kind"] = serde_json::json!(if source.placement() == Transform::IDENTITY
        && source.source_domain() == [[0., 1.], [0., 1.]]
    {
        "exact local graph bounds"
    } else {
        "conservative control hull bounds"
    });
    data["curved_walls"] =
        serde_json::json!(x[3] != 0. && source.source_domain() != [[0., 1.], [0., 1.]]);
    data["rational_roof_path"] = serde_json::json!({"center":[cx,cy],"radius":r,"units":"mm","quarters":quarters,"closed":true,"stock_unchanged":true,"bore_created":false,"scope":"exact rational circle lifted onto the checked graph roof; path only, no hole or Boolean operation"});
    Ok(data.to_string())
}
