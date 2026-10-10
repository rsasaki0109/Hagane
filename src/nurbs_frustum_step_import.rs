//! Strict exact-representation recognition of an identity-axis millimetre frustum.
use crate::*;
use std::collections::BTreeSet;
fn unsupported() -> Error {
    Error::Unsupported(
        "STEP is not an exact supported identity-axis millimetre canonical rational frustum",
    )
}
fn same_surface(a: &Surface, b: &Surface) -> Result<bool> {
    match (a, b) {
        (
            Surface::Plane {
                origin: a,
                u: b,
                v: c,
            },
            Surface::Plane {
                origin: d,
                u: e,
                v: f,
            },
        ) => Ok(a == d && b == e && c == f),
        _ => crate::nurbs_graph_step_import::same_surface(a, b),
    }
}
fn line_preimage(
    a: Point3,
    b: Point3,
    raw: &crate::nurbs_frustum_step_read::SerializedLine,
) -> bool {
    // Exactly the elementary operation order of the private writer. Product
    // decoding is only a placeholder, never an admission or snapping rule.
    let delta = b - a;
    let scale = delta.x.abs().max(delta.y.abs()).max(delta.z.abs());
    if !scale.is_normal() {
        return false;
    }
    let unit = Vec3::new(delta.x / scale, delta.y / scale, delta.z / scale);
    let length = scale * (unit.x * unit.x + unit.y * unit.y + unit.z * unit.z).sqrt();
    raw.origin == a
        && raw.magnitude == length
        && raw.direction == Vec3::new(delta.x / length, delta.y / length, delta.z / length)
}
/// Import one exact canonical identity-frame frustum in millimetres.
/// Rational spline coefficients are retained. Analytic LINE/VECTOR and affine
/// UV records are accepted only as exact serialized preimages of the retained
/// canonical primary parameter. Placement, unit conversion, other lofts and
/// arbitrary parameter bases remain explicitly unsupported.
pub fn import_step_nurbs_frustum_mm(
    input: &str,
    policy: GeometryTolerance,
) -> Result<NurbsFrustumSolid> {
    import_frustum(input, policy, false)
}

/// Import an exact identity-axis frustum with a representable translation.
/// Rotation and unit conversion remain unsupported. Every actual coefficient
/// must exactly reproduce the inferred translated canonical representation.
pub fn import_step_nurbs_frustum_translated_mm(
    input: &str,
    policy: GeometryTolerance,
) -> Result<NurbsFrustumSolid> {
    import_frustum(input, policy, true)
}

fn import_frustum(
    input: &str,
    policy: GeometryTolerance,
    translated: bool,
) -> Result<NurbsFrustumSolid> {
    let parsed = crate::nurbs_frustum_step_read::read_frustum_solid(input, policy.absolute())?;
    let actual = &parsed.solid;
    let bottom = actual
        .shell
        .faces
        .iter()
        .find(|f| {
            matches!(f.surface,Surface::Plane{origin,..} if translated || origin==Point3::new(0.,0.,0.))
                && f.orientation == -1
        })
        .ok_or_else(unsupported)?;
    let Surface::Plane {
        origin: bottom_origin,
        ..
    } = bottom.surface
    else {
        return Err(unsupported());
    };
    let top = actual
        .shell
        .faces
        .iter()
        .find(|f| {
            matches!(f.surface,Surface::Plane{origin,..} if origin.x==bottom_origin.x&&origin.y==bottom_origin.y&&origin.z>bottom_origin.z)
                && f.orientation == 1
        })
        .ok_or_else(unsupported)?;
    let radius = |f: &Face| -> Result<f64> {
        if translated {
            let PCurve::Nurbs(c) = &f.wires[0].coedges[0].pcurve else {
                return Err(unsupported());
            };
            let p = c.control_points()[0];
            return Ok(p.x.abs().max(p.y.abs()));
        }
        let edge = &actual.edges[f.wires[0].coedges[0].edge];
        let Curve::Nurbs(c) = &edge.curve else {
            return Err(unsupported());
        };
        let p = c.control_points()[0];
        Ok(p.x.abs().max(p.y.abs()))
    };
    let radii = [radius(bottom)?, radius(top)?];
    let Surface::Plane { origin, .. } = top.surface else {
        return Err(unsupported());
    };
    let height = origin.z - bottom_origin.z;
    let frame = Frame3::new(bottom_origin, Frame3::IDENTITY.axes(), policy.absolute())?;
    let candidate = NurbsFrustumSolid::new(frame, radii, height, policy)?;
    let expected = candidate.solid();
    let mut vertex_map = Vec::new();
    let mut seen = BTreeSet::new();
    for v in &actual.vertices {
        let i = expected
            .vertices
            .iter()
            .position(|e| e.point == v.point)
            .ok_or_else(unsupported)?;
        if !seen.insert(i) {
            return Err(unsupported());
        }
        vertex_map.push(i);
    }
    let mut face_map = Vec::new();
    seen.clear();
    for f in &actual.shell.faces {
        let mut found = None;
        for (i, e) in expected.shell.faces.iter().enumerate() {
            if same_surface(&f.surface, &e.surface)? && found.replace(i).is_some() {
                return Err(unsupported());
            }
        }
        let i = found.ok_or_else(unsupported)?;
        if !seen.insert(i) {
            return Err(unsupported());
        }
        face_map.push(i);
    }
    let mut edge_map = Vec::new();
    let mut restored = parsed.solid.clone();
    seen.clear();
    for (index, e) in restored.edges.iter_mut().enumerate() {
        e.vertices = e.vertices.map(|i| vertex_map[i]);
        let i = expected
            .edges
            .iter()
            .position(|x| x.vertices == e.vertices)
            .ok_or_else(unsupported)?;
        if !seen.insert(i) {
            return Err(unsupported());
        }
        match (&e.curve, &expected.edges[i].curve) {
            (Curve::Nurbs(_), Curve::Nurbs(_)) => {
                if !crate::nurbs_graph_step_import::same_curve(&e.curve, &expected.edges[i].curve) {
                    return Err(unsupported());
                }
            }
            (Curve::Line { .. }, Curve::Line { a, b }) => {
                let raw = parsed.lines.get(&index).ok_or_else(unsupported)?;
                if !line_preimage(*a, *b, raw) {
                    return Err(unsupported());
                }
                e.curve = Curve::Line { a: *a, b: *b };
            }
            _ => return Err(unsupported()),
        }
        edge_map.push(i);
    }
    for (fi, f) in restored.shell.faces.iter_mut().enumerate() {
        let target = &expected.shell.faces[face_map[fi]];
        if f.wires.len() != 1 || target.wires.len() != 1 {
            return Err(unsupported());
        }
        for c in &mut f.wires[0].coedges {
            let original = c.edge;
            c.edge = edge_map[original];
            let expected_use = target.wires[0]
                .coedges
                .iter()
                .find(|e| e.edge == c.edge)
                .ok_or_else(unsupported)?;
            match (&c.pcurve, &expected_use.pcurve) {
                (PCurve::Affine { .. }, PCurve::Affine { origin, direction }) => {
                    let raw = parsed.affine.get(&(fi, original)).ok_or_else(unsupported)?;
                    let (ratios, magnitude) =
                        crate::nurbs_graph_step::decompose_affine(*origin, *direction)?;
                    if raw.origin != *origin
                        || raw.direction != ratios
                        || raw.magnitude != magnitude
                    {
                        return Err(unsupported());
                    }
                    c.pcurve = expected_use.pcurve.clone();
                }
                (PCurve::Nurbs(_), PCurve::Nurbs(_)) => (),
                _ => return Err(unsupported()),
            }
        }
        let start = f.wires[0]
            .coedges
            .iter()
            .position(|c| c.edge == target.wires[0].coedges[0].edge)
            .ok_or_else(unsupported)?;
        f.wires[0].coedges.rotate_left(start);
    }
    let mut solid = expected.clone();
    for (i, v) in restored.vertices.into_iter().enumerate() {
        solid.vertices[vertex_map[i]] = v;
    }
    for (i, e) in restored.edges.into_iter().enumerate() {
        solid.edges[edge_map[i]] = e;
    }
    for (i, f) in restored.shell.faces.into_iter().enumerate() {
        solid.shell.faces[face_map[i]] = f;
    }
    NurbsFrustumSolid::from_brep(solid, frame, radii, height, policy)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cyclic_loops_and_shell_order_preserve_exact_body_not_parser_order() {
        let p = GeometryTolerance::default();
        let original = NurbsFrustumSolid::new(Frame3::IDENTITY, [7., 2.], 11., p).unwrap();
        let text = original.export_step_mm(p).unwrap();
        let changed = text
            .lines()
            .map(|line| {
                for kind in ["EDGE_LOOP", "CLOSED_SHELL"] {
                    let prefix = format!("{kind}('',(");
                    if let Some(begin) = line.find(&prefix) {
                        let begin = begin + prefix.len();
                        let end = begin + line[begin..].find(')').unwrap();
                        let mut ids: Vec<_> = line[begin..end].split(',').collect();
                        if kind == "EDGE_LOOP" {
                            ids.rotate_left(1);
                        } else {
                            ids.reverse();
                        }
                        return format!("{}{}{}", &line[..begin], ids.join(","), &line[end..]);
                    }
                }
                line.to_owned()
            })
            .collect::<Vec<_>>()
            .join("\n");
        let actual = import_step_nurbs_frustum_mm(&changed, p).unwrap();
        assert_eq!(actual.export_step_mm(p).unwrap(), text);
    }
    #[test]
    fn both_raw_analytic_parameterizations_are_required_for_admission() {
        let p = GeometryTolerance::default();
        let original = NurbsFrustumSolid::new(Frame3::IDENTITY, [7., 2.], 11., p).unwrap();
        let text = original.export_step_mm(p).unwrap();
        // Coherent scaling of direction and inverse VECTOR length represents
        // the same mathematical line. It is nevertheless not the supported
        // exact serialization preimage, and must not silently be normalized.
        let parsed =
            crate::nurbs_frustum_step_read::read_frustum_solid(&text, p.absolute()).unwrap();
        let raw = parsed.lines.values().next().unwrap();
        let expected = original
            .solid()
            .edges
            .iter()
            .find_map(|e| match e.curve {
                Curve::Line { a, b } if a == raw.origin => Some((a, b)),
                _ => None,
            })
            .unwrap();
        let mut changed = raw.clone();
        changed.direction = changed.direction * 2.;
        changed.magnitude /= 2.;
        assert!(!line_preimage(expected.0, expected.1, &changed));
        assert!(line_preimage(expected.0, expected.1, raw));
        assert_eq!(parsed.affine.len(), 16);
        assert_eq!(parsed.lines.len(), 4);
    }
}
