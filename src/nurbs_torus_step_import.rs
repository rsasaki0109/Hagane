//! Strict millimetre cardinal torus recognition retaining actual STEP geometry.
//! The first encoded shell patch anchors major/minor parameter phase. No fitting,
//! unit conversion, arbitrary-frame recovery or general rational import is used.
use crate::*;
use std::collections::BTreeSet;
fn unsupported() -> Error {
    Error::Unsupported("STEP is not an exact resolved millimetre cardinal ring torus with supported phase and representation")
}
fn component(p: Point3, axis: Vec3) -> f64 {
    if axis.x != 0. {
        p.x
    } else if axis.y != 0. {
        p.y
    } else {
        p.z
    }
}
fn direction(delta: Vec3) -> Result<Vec3> {
    let a = [delta.x, delta.y, delta.z];
    if !delta.finite() || a.iter().filter(|&&x| x != 0.).count() != 1 {
        return Err(unsupported());
    }
    Ok(Vec3::new(
        if delta.x == 0. { 0. } else { delta.x.signum() },
        if delta.y == 0. { 0. } else { delta.y.signum() },
        if delta.z == 0. { 0. } else { delta.z.signum() },
    ))
}
fn neighbors(value: f64) -> Result<Vec<f64>> {
    if !value.is_normal() || value <= 0. {
        return Err(unsupported());
    }
    let bits = value.to_bits();
    let offsets = std::iter::once(0i64).chain((1..=8).flat_map(|i| [-i, i]));
    Ok(offsets
        .filter_map(|offset| {
            let bits = bits.checked_add_signed(offset)?;
            let v = f64::from_bits(bits);
            (v.is_normal() && v > 0.).then_some(v)
        })
        .collect())
}
/// Import one canonical positive ring torus in millimetres with a translated,
/// right-handed signed-cardinal frame. Its first shell patch must encode the
/// outer-equator-to-upper-minor quarter; other faces may be reindexed bijectively.
/// Radii are inferred represented preimages, not recovered nominal input bits.
/// The arithmetic seed precedes bounded neighboring floats; every admitted
/// preimage reproduces all actual coefficients exactly. Missing preimages refuse.
pub fn import_step_nurbs_torus_cardinal_mm(
    input: &str,
    policy: GeometryTolerance,
) -> Result<NurbsTorusSolid> {
    let parsed =
        crate::nurbs_frustum_step_read::read_torus_cardinal_solid(input, policy.absolute())?;
    let Surface::Nurbs(first) = &parsed.solid.shell.faces[0].surface else {
        return Err(unsupported());
    };
    if first.degrees() != [2, 2]
        || first.control_counts() != [3, 3]
        || first.knots(0)? != [0., 0., 0., 1., 1., 1.]
        || first.knots(1)? != [0., 0., 0., 1., 1., 1.]
    {
        return Err(unsupported());
    }
    let cp = first.control_points();
    let u = direction(cp[1] - cp[2])?;
    let w = direction(cp[1] - cp[0])?;
    if u.dot(w) != 0. {
        return Err(unsupported());
    }
    let v = w.cross(u);
    let mut origin = [0.; 3];
    let a = [u.x, u.y, u.z];
    let b = [v.x, v.y, v.z];
    let zero0 = [cp[0].x, cp[0].y, cp[0].z];
    let zero8 = [cp[8].x, cp[8].y, cp[8].z];
    for i in 0..3 {
        origin[i] = if a[i] != 0. { zero8[i] } else { zero0[i] };
        if a[i] == 0. && b[i] == 0. && [w.x, w.y, w.z][i] == 0. {
            return Err(unsupported());
        }
    }
    let origin = Point3::new(origin[0], origin[1], origin[2]);
    let frame = Frame3::new_with_tolerance(origin, [u, v, w], policy)?;
    let r = (component(cp[2], u) - component(origin, u))
        * u.dot(Vec3::new(u.x.abs(), u.y.abs(), u.z.abs()));
    let minor = (cp[1] - cp[0]).dot(w);
    for major in neighbors(r)? {
        for minor in neighbors(minor)? {
            let point = |x, y, z| frame.point(Point3::new(x, y, z));
            let expected = [
                point(major + minor, 0., 0.),
                point(major + minor, 0., minor),
                point(major, 0., minor),
                point(major + minor, major + minor, 0.),
                point(major + minor, major + minor, minor),
                point(major, major, minor),
                point(0., major + minor, 0.),
                point(0., major + minor, minor),
                point(0., major, minor),
            ];
            if cp != expected {
                continue;
            }
            let Ok(candidate) = NurbsTorusSolid::new(frame, [major, minor], policy) else {
                continue;
            };
            if let Ok(body) = admit(&parsed, &candidate, policy) {
                return Ok(body);
            }
        }
    }
    Err(unsupported())
}
fn admit(
    parsed: &crate::nurbs_frustum_step_read::ParsedFrustumStep,
    candidate: &NurbsTorusSolid,
    policy: GeometryTolerance,
) -> Result<NurbsTorusSolid> {
    let actual = &parsed.solid;
    let expected = candidate.solid();
    let mut seen = BTreeSet::new();
    let mut vertices = Vec::new();
    for v in &actual.vertices {
        let i = expected
            .vertices
            .iter()
            .position(|x| x.point == v.point)
            .ok_or_else(unsupported)?;
        if !seen.insert(i) {
            return Err(unsupported());
        }
        vertices.push(i);
    }
    let mut faces = Vec::new();
    seen.clear();
    for face in &actual.shell.faces {
        let mut found = None;
        for (i, e) in expected.shell.faces.iter().enumerate() {
            if crate::nurbs_graph_step_import::same_surface(&face.surface, &e.surface)?
                && found.replace(i).is_some()
            {
                return Err(unsupported());
            }
        }
        let i = found.ok_or_else(unsupported)?;
        if !seen.insert(i) {
            return Err(unsupported());
        }
        faces.push(i);
    }
    let mut restored = actual.clone();
    let mut edges = Vec::new();
    seen.clear();
    for edge in &mut restored.edges {
        edge.vertices = edge.vertices.map(|i| vertices[i]);
        let i = expected
            .edges
            .iter()
            .position(|e| e.vertices == edge.vertices)
            .ok_or_else(unsupported)?;
        if !seen.insert(i)
            || !crate::nurbs_graph_step_import::same_curve(&edge.curve, &expected.edges[i].curve)
        {
            return Err(unsupported());
        }
        edges.push(i);
    }
    for (fi, face) in restored.shell.faces.iter_mut().enumerate() {
        let target = &expected.shell.faces[faces[fi]];
        if face.wires.len() != 1 {
            return Err(unsupported());
        }
        for coedge in &mut face.wires[0].coedges {
            let old = coedge.edge;
            coedge.edge = edges[old];
            let target = target.wires[0]
                .coedges
                .iter()
                .find(|c| c.edge == coedge.edge)
                .ok_or_else(unsupported)?;
            let PCurve::Affine { origin, direction } = target.pcurve else {
                return Err(unsupported());
            };
            let raw = parsed.affine.get(&(fi, old)).ok_or_else(unsupported)?;
            let (ratios, magnitude) = crate::nurbs_graph_step::decompose_affine(origin, direction)?;
            if raw.origin != origin || raw.direction != ratios || raw.magnitude != magnitude {
                return Err(unsupported());
            }
            coedge.pcurve = target.pcurve.clone();
        }
        let start = face.wires[0]
            .coedges
            .iter()
            .position(|c| c.edge == target.wires[0].coedges[0].edge)
            .ok_or_else(unsupported)?;
        face.wires[0].coedges.rotate_left(start);
    }
    let mut solid = expected.clone();
    for (i, v) in restored.vertices.into_iter().enumerate() {
        solid.vertices[vertices[i]] = v;
    }
    for (i, e) in restored.edges.into_iter().enumerate() {
        solid.edges[edges[i]] = e;
    }
    for (i, f) in restored.shell.faces.into_iter().enumerate() {
        solid.shell.faces[faces[i]] = f;
    }
    NurbsTorusSolid::from_brep(solid, candidate.frame(), candidate.radii(), policy)
}
