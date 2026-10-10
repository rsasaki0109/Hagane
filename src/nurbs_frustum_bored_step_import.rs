//! Strict retained-geometry admission of millimetre coaxially bored frusta.
//! Only translated right-handed signed-cardinal frames are admitted.
use crate::*;
use std::collections::BTreeSet;
fn unsupported() -> Error {
    Error::Unsupported("STEP is not an exact supported millimetre canonical bored rational frustum")
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
/// Import an exact millimetre coaxially bored rational frustum with a
/// right-handed signed-cardinal frame and representable translation.
/// All actual rational coefficients, wires and orientations are retained.
/// Analytic LINE/VECTOR and affine UV records require the exact writer preimage.
/// General rotations, oblique children, unit conversion and fitting reject.
pub fn import_step_nurbs_bored_frustum_cardinal_mm(
    input: &str,
    policy: GeometryTolerance,
) -> Result<NurbsBoredFrustumSolid> {
    let parsed = crate::nurbs_frustum_step_read::read_bored_frustum_cardinal_solid(
        input,
        policy.absolute(),
    )?;
    if parsed.solid.vertices.len() != 16
        || parsed.solid.edges.len() != 24
        || parsed.solid.shell.faces.len() != 10
    {
        return Err(unsupported());
    }
    let actual = &parsed.solid;
    let bottom = actual
        .shell
        .faces
        .iter()
        .find(|f| matches!(f.surface, Surface::Plane { .. }) && f.orientation == -1)
        .ok_or_else(unsupported)?;
    let Surface::Plane {
        origin: bottom_origin,
        u,
        v,
    } = bottom.surface
    else {
        return Err(unsupported());
    };
    let axis = u.cross(v);
    let top = actual
        .shell
        .faces
        .iter()
        .find(|f| {
            matches!(f.surface,Surface::Plane{origin,u:top_u,v:top_v} if top_u==u&&top_v==v && { let d=origin-bottom_origin; let h=d.dot(axis); h>0.&&d==axis*h })
                && f.orientation == 1
        })
        .ok_or_else(unsupported)?;
    let radius = |f: &Face, wire: usize| -> Result<f64> {
        let PCurve::Nurbs(c) = &f.wires[wire].coedges[0].pcurve else {
            return Err(unsupported());
        };
        let p = c.control_points()[0];
        Ok(p.x.abs().max(p.y.abs()))
    };
    if bottom.wires.len() != 2 || top.wires.len() != 2 {
        return Err(unsupported());
    }
    let radii = [radius(bottom, 0)?, radius(top, 0)?];
    let hole = radius(bottom, 1)?;
    if radius(top, 1)? != hole {
        return Err(unsupported());
    }
    let Surface::Plane { origin, .. } = top.surface else {
        return Err(unsupported());
    };
    let height = (origin - bottom_origin).dot(axis);
    let frame = Frame3::new(bottom_origin, [u, v, axis], policy.absolute())?;
    let source = NurbsFrustumSolid::new(frame, radii, height, policy)?;
    let candidate = source.bore_coaxial(hole, policy)?;
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
        if f.wires.len() != target.wires.len() {
            return Err(unsupported());
        }
        // The isolated decoder puts FACE_OUTER_BOUND first. Inner and outer
        // wires retain their roles; only their cyclic starting edge is normalized.
        for (wire, expected_wire) in f.wires.iter_mut().zip(&target.wires) {
            if wire.coedges.len() != expected_wire.coedges.len() {
                return Err(unsupported());
            }
            let mut used = BTreeSet::new();
            for c in &mut wire.coedges {
                let original = c.edge;
                c.edge = edge_map[original];
                if !used.insert(c.edge) {
                    return Err(unsupported());
                }
                let expected_use = expected_wire
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
            let start = wire
                .coedges
                .iter()
                .position(|c| c.edge == expected_wire.coedges[0].edge)
                .ok_or_else(unsupported)?;
            wire.coedges.rotate_left(start);
        }
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
    NurbsBoredFrustumSolid::from_brep(solid, &source, hole, policy)
}
