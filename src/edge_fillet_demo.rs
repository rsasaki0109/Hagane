//! Exact parallel box-edge fillets and display derived from their actual analytic B-rep.
use crate::*;
use serde_json::Value;
fn xyz(p: Point3) -> [f64; 3] {
    [p.x, p.y, p.z]
}
fn frame_json(frame: Frame3) -> Value {
    serde_json::json!({"origin":xyz(frame.origin()),"axes":frame.axes().iter().map(|a|[a.x,a.y,a.z]).collect::<Vec<_>>()})
}
fn surface_json(s: &Surface) -> Result<Value> {
    match s {
        Surface::Plane { origin, u, v } => Ok(
            serde_json::json!({"kind":"plane","origin":xyz(*origin),"u":[u.x,u.y,u.z],"v":[v.x,v.y,v.z]}),
        ),
        Surface::FramedCylinder {
            frame,
            radius,
            height,
        } => Ok(
            serde_json::json!({"kind":"framed_cylinder","frame":frame_json(*frame),"radius":radius,"height":height}),
        ),
        _ => Err(Error::Unsupported("fillet demo surface unsupported")),
    }
}
fn curve_json(c: &Curve) -> Result<Value> {
    match c {
        Curve::Line { a, b } => {
            Ok(serde_json::json!({"kind":"line","a":xyz(*a),"b":xyz(*b),"parameter_range":[0.,1.]}))
        }
        Curve::Arc {
            frame,
            radius,
            sweep,
        } => Ok(
            serde_json::json!({"kind":"arc","frame":frame_json(*frame),"radius":radius,"sweep":sweep,"parameter_range":[0.,sweep]}),
        ),
        _ => Err(Error::Unsupported("fillet demo edge unsupported")),
    }
}
fn pcurve_json(c: &PCurve) -> Result<Value> {
    match c {
        PCurve::Affine { origin, direction } => {
            Ok(serde_json::json!({"kind":"affine","origin":origin,"direction":direction}))
        }
        PCurve::Arc {
            center,
            radius,
            start_angle,
            sweep,
        } => Ok(
            serde_json::json!({"kind":"arc","center":center,"radius":radius,"start_angle":start_angle,"sweep":sweep}),
        ),
        _ => Err(Error::Unsupported("fillet demo pcurve unsupported")),
    }
}
fn brep_json(solid: &Solid) -> Result<Value> {
    let curves = solid
        .edges
        .iter()
        .map(|e| curve_json(&e.curve))
        .collect::<Result<Vec<_>>>()?;
    let surfaces = solid
        .shell
        .faces
        .iter()
        .map(|f| surface_json(&f.surface))
        .collect::<Result<Vec<_>>>()?;
    let mut wires = Vec::new();
    let mut tangencies = Vec::new();
    for (face_index, face) in solid.shell.faces.iter().enumerate() {
        let mut face_wires = Vec::new();
        for wire in &face.wires {
            let mut coedges = Vec::new();
            for coedge in &wire.coedges {
                let edge = &solid.edges[coedge.edge];
                let end = match edge.curve {
                    Curve::Arc { sweep, .. } => sweep,
                    _ => 1.,
                };
                let witnesses=(0..=8).map(|j|{let t=end*j as f64/8.;let uv=coedge.pcurve.try_evaluate(t)?;Ok(serde_json::json!({"parameter":t,"uv":uv,"point":xyz(edge.curve.try_evaluate(t)?),"surface_point":xyz(face.surface.try_evaluate(uv[0],uv[1])?)}))}).collect::<Result<Vec<_>>>()?;
                coedges.push(serde_json::json!({"edge":coedge.edge,"forward":if face.orientation>0{coedge.forward}else{!coedge.forward},"pcurve":pcurve_json(&coedge.pcurve)?,"witnesses":witnesses}));
                if matches!(face.surface, Surface::FramedCylinder { .. })
                    && matches!(edge.curve, Curve::Line { .. })
                {
                    for (other_index, other) in solid.shell.faces.iter().enumerate() {
                        if matches!(other.surface, Surface::Plane { .. })
                            && other
                                .wires
                                .iter()
                                .flat_map(|w| &w.coedges)
                                .any(|c| c.edge == coedge.edge)
                        {
                            let p = edge.curve.try_evaluate(0.5)?;
                            let uv = face.surface.try_parameters(p)?;
                            let ov = other.surface.try_parameters(p)?;
                            let n = face.surface.normal_at(uv[0], uv[1])?;
                            let m = other.surface.normal_at(ov[0], ov[1])?;
                            tangencies.push(serde_json::json!({"edge":coedge.edge,"faces":[face_index,other_index],"point":xyz(p),"normal_dot":n.dot(m).abs()}));
                        }
                    }
                }
            }
            face_wires.push(coedges);
        }
        wires.push(face_wires);
    }
    Ok(
        serde_json::json!({"vertices":solid.vertices.iter().map(|v|xyz(v.point)).collect::<Vec<_>>(),"edge_vertices":solid.edges.iter().map(|e|e.vertices).collect::<Vec<_>>(),"edges":solid.edges.len(),"faces":solid.shell.faces.len(),"closed":true,"curves":curves,"surfaces":surfaces,"face_orientations":solid.shell.faces.iter().map(|f|f.orientation).collect::<Vec<_>>(),"wires":wires,"tangency_witnesses":tangencies}),
    )
}
fn solid_json(solid: &Solid, error: f64, tol: Tolerance) -> Result<Value> {
    solid.validate(tol)?;
    let mesh = solid.tessellate(error, tol)?;
    let bounds = solid.bounds();
    let roundoff = solid.tessellation_roundoff_budget(tol)?;
    let error_bounds = mesh
        .triangles
        .iter()
        .zip(&mesh.face_ids)
        .map(|(triangle, face)| -> Result<f64> {
            let mut sagitta = 0_f64;
            if let Surface::FramedCylinder { frame, radius, .. } = solid.shell.faces[*face].surface
            {
                let axes = frame.axes();
                for i in 0..3 {
                    let a = mesh.positions[triangle[i]] - frame.origin();
                    let b = mesh.positions[triangle[(i + 1) % 3]] - frame.origin();
                    let (ax, ay, bx, by) = (
                        a.dot(axes[0]),
                        a.dot(axes[1]),
                        b.dot(axes[0]),
                        b.dot(axes[1]),
                    );
                    let angle = (ax * by - ay * bx).atan2(ax * bx + ay * by).abs();
                    sagitta = sagitta.max(2. * radius * (angle / 4.).sin().powi(2));
                }
            }
            let bound = sagitta + roundoff;
            if !bound.is_finite() || bound > error {
                return Err(Error::Tessellation(
                    "fillet display chord bound is unresolved",
                ));
            }
            Ok(bound)
        })
        .collect::<Result<Vec<_>>>()?;
    let positions: Vec<_> = mesh
        .triangles
        .iter()
        .flat_map(|t| t.iter().flat_map(|i| xyz(mesh.positions[*i])))
        .collect();
    let normals: Vec<_> = mesh
        .triangles
        .iter()
        .flat_map(|t| {
            t.iter().flat_map(|i| {
                let n = mesh.normals[*i];
                [n.x, n.y, n.z]
            })
        })
        .collect();
    Ok(
        serde_json::json!({"volume":solid.volume()?,"bounds":{"min":xyz(bounds.min),"max":xyz(bounds.max)},"positions":positions,"normals":normals,"mesh":{"positions":mesh.positions.iter().map(|p|xyz(*p)).collect::<Vec<_>>(),"normals":mesh.normals.iter().map(|n|[n.x,n.y,n.z]).collect::<Vec<_>>(),"triangles":mesh.triangles,"face_ids":mesh.face_ids},"error_bounds":error_bounds,"display_bound_kind":"analytic cylindrical chord sagitta plus conservative coordinate arithmetic allowance","brep":brep_json(solid)?}),
    )
}
pub fn edge_fillet_demo_json(x: &[f64]) -> Result<String> {
    if !(12..=18).contains(&x.len())
        || x.iter().any(|v| !v.is_finite())
        || x[9] < 1.
        || x[9] > 4.
        || x[9].fract() != 0.
        || x.len() != 10 + 2 * x[9] as usize
    {
        return Err(Error::InvalidInput("parallel box fillet needs ten finite model/count values and exactly 1..4 edge/radius pairs"));
    }
    let mut selections = Vec::new();
    for pair in x[10..].as_chunks::<2>().0 {
        if pair[0] < 0. || pair[0] >= 12. || pair[0].fract() != 0. {
            return Err(Error::InvalidInput(
                "fillet edge index must be integer in 0..12",
            ));
        }
        selections.push((pair[0] as usize, pair[1]));
    }
    let tol = GeometryTolerance::new(x[7], GeometryTolerance::default().angular(), 0.)?;
    let pose = Transform::translation(Vec3::new(x[4], x[5], x[6]))?
        .compose(Transform::rotation(Vec3::new(0., 1., 0.), x[3])?)?;
    let source = make_box(
        BoxSpec {
            min: Point3::new(0., 0., 0.),
            size: Vec3::new(x[0], x[1], x[2]),
        },
        tol.absolute(),
    )?
    .transformed(pose, tol.absolute())?;
    let result = fillet_parallel_box_edges(&source, &selections, tol)?;
    Ok(serde_json::json!({"units":"mm","scope":"1..4 parallel edges of a checked rigid box with disjoint exact circular fillets; mixed axes, contact, general fillets and unresolved precision remain unsupported","source":solid_json(&source,x[8],tol.absolute())?,"kept":solid_json(result.solid(),x[8],tol.absolute())?,"removed_volume":result.removed_volume(),"fillets":{"selections":result.selections(),"faces":result.fillet_faces(),"geometry":"exact circular cylindrical surfaces tangent to incident planes"},"display_chord_tolerance":x[8],"placement":{"angle":x[3],"translation":[x[4],x[5],x[6]],"axis":[0.,1.,0.]},"linear_tolerance":x[7],"step":export_step_bounded_analytic_mm(result.solid(),x[7])?,"step_exact":true,"step_schema":"AUTOMOTIVE_DESIGN"}).to_string())
}
