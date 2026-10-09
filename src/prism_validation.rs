//! Structural certificate for a translation of a simple planar polygon region.
use crate::{Curve, Error, Face, Result, Solid, Surface, Tolerance, Vec3};
use std::collections::{BTreeMap, BTreeSet};

/// A geometric translation certificate, not recovered modeling history.
#[derive(Debug, Clone, PartialEq)]
pub struct PlanarPrismCertificate {
    pub cap_faces: [usize; 2],
    pub translation: Vec3,
    pub profile_corners: usize,
    pub profile_holes: usize,
}
const DOMAIN: Error = Error::Unsupported("solid is not a certified straight polygon prism");
fn edges(face: &Face) -> BTreeSet<usize> {
    face.wires
        .iter()
        .flat_map(|w| w.coedges.iter().map(|c| c.edge))
        .collect()
}
fn vertices(s: &Solid, face: &Face) -> BTreeSet<usize> {
    edges(face)
        .iter()
        .flat_map(|&e| s.edges[e].vertices)
        .collect()
}
fn outward(face: &Face) -> Result<Vec3> {
    Ok(face.surface.normal(0.).normalized()? * f64::from(face.orientation))
}
fn on_plane(face: &Face, p: Vec3, budget: f64) -> Result<bool> {
    let Surface::Plane { origin, .. } = face.surface else {
        return Err(DOMAIN);
    };
    let distance = (p - origin).dot(outward(face)?).abs();
    Ok(distance.is_finite() && distance <= budget)
}
/// Certify an unsubdivided planar polygon extrusion, including concavity/holes.
/// Cap rings must correspond bijectively; every side must be the exact translated
/// quadrilateral of one profile edge. Agreement uses bounded arithmetic roundoff,
/// never the modeling tolerance as permission to deform or snap the input.
pub fn certify_planar_prism(s: &Solid, t: Tolerance) -> Result<PlanarPrismCertificate> {
    s.validate(t)?;
    certify_validated_planar_prism(s, t)
}
pub(crate) fn certify_validated_planar_prism(
    s: &Solid,
    t: Tolerance,
) -> Result<PlanarPrismCertificate> {
    let count = s.shell.faces.len();
    if !(5..=128).contains(&count) {
        return Err(DOMAIN);
    }
    let n = count - 2;
    if s.vertices.len() != 2 * n
        || s.edges.len() != 3 * n
        || s.edges
            .iter()
            .any(|e| !matches!(e.curve, Curve::Line { .. }))
        || s.shell
            .faces
            .iter()
            .any(|f| !matches!(f.surface, Surface::Plane { .. }))
    {
        return Err(DOMAIN);
    }
    let diagonal = (s.bounds().max - s.bounds().min).norm();
    let budget = (64. * f64::EPSILON * diagonal).min(t.linear / 1024.);
    if !diagonal.is_finite() || !s.volume()?.is_finite() || budget <= 0. {
        return Err(DOMAIN);
    }
    let angular = 64. * f64::EPSILON;
    for i in 0..count {
        let a = &s.shell.faces[i];
        let av = vertices(s, a);
        let ae = edges(a);
        if av.len() != n || ae.len() != n || a.wires.len() > 65 {
            continue;
        }
        for j in i + 1..count {
            let b = &s.shell.faces[j];
            let bv = vertices(s, b);
            let be = edges(b);
            if bv.len() != n
                || be.len() != n
                || b.wires.len() != a.wires.len()
                || !av.is_disjoint(&bv)
                || !ae.is_disjoint(&be)
            {
                continue;
            }
            let an = outward(a)?;
            let bn = outward(b)?;
            if an.dot(bn) >= 0. || an.cross(bn).norm() > angular {
                continue;
            }
            let mut mapping = BTreeMap::new();
            let mut links = BTreeMap::new();
            let mut valid = true;
            for (e, edge) in s.edges.iter().enumerate() {
                if ae.contains(&e) || be.contains(&e) {
                    continue;
                }
                let [x, y] = edge.vertices;
                let pair = if av.contains(&x) && bv.contains(&y) {
                    Some((x, y))
                } else if av.contains(&y) && bv.contains(&x) {
                    Some((y, x))
                } else {
                    None
                };
                if let Some((x, y)) = pair {
                    if mapping.insert(x, y).is_some() {
                        valid = false;
                        break;
                    }
                    links.insert(x, e);
                } else {
                    valid = false;
                    break;
                }
            }
            if !valid
                || mapping.len() != n
                || mapping.values().copied().collect::<BTreeSet<_>>() != bv
            {
                continue;
            }
            let (&x, &y) = mapping.first_key_value().unwrap();
            let delta = s.vertices[y].point - s.vertices[x].point;
            if !delta.finite()
                || delta.dot(an) >= -10. * t.linear
                || delta.dot(bn) <= 10. * t.linear
            {
                continue;
            }
            if mapping
                .iter()
                .any(|(&x, &y)| (s.vertices[y].point - s.vertices[x].point - delta).norm() > budget)
            {
                continue;
            }
            if av
                .iter()
                .any(|&v| !on_plane(a, s.vertices[v].point, budget).unwrap_or(false))
                || bv
                    .iter()
                    .any(|&v| !on_plane(b, s.vertices[v].point, budget).unwrap_or(false))
            {
                continue;
            }
            let top_edges: BTreeMap<_, _> = be
                .iter()
                .map(|&e| {
                    let mut ends = s.edges[e].vertices;
                    ends.sort();
                    (ends, e)
                })
                .collect();
            let mut corresponding = BTreeMap::new();
            for &e in &ae {
                let [x, y] = s.edges[e].vertices;
                let mut ends = [mapping[&x], mapping[&y]];
                ends.sort();
                if let Some(&top) = top_edges.get(&ends) {
                    corresponding.insert(e, top);
                } else {
                    valid = false;
                    break;
                }
            }
            if !valid {
                continue;
            }
            let bottom_rings: Vec<BTreeSet<_>> = a
                .wires
                .iter()
                .map(|w| w.coedges.iter().map(|c| corresponding[&c.edge]).collect())
                .collect();
            let top_rings: Vec<BTreeSet<_>> = b
                .wires
                .iter()
                .map(|w| w.coedges.iter().map(|c| c.edge).collect())
                .collect();
            if bottom_rings[0] != top_rings[0]
                || bottom_rings[1..]
                    .iter()
                    .any(|r| !top_rings[1..].contains(r))
            {
                continue;
            }
            let mut quads = BTreeMap::new();
            for wire in &a.wires {
                for c in &wire.coedges {
                    let [x, y] = s.edges[c.edge].vertices;
                    let set =
                        BTreeSet::from([c.edge, corresponding[&c.edge], links[&x], links[&y]]);
                    let tangent = (s.vertices[y].point - s.vertices[x].point)
                        * if c.forward { 1. } else { -1. };
                    let normal = tangent.cross(delta).normalized()? * -f64::from(a.orientation);
                    quads.insert(set, (normal, s.vertices[x].point));
                }
            }
            for (k, face) in s.shell.faces.iter().enumerate() {
                if k == i || k == j {
                    continue;
                }
                if face.wires.len() != 1 || face.wires[0].coedges.len() != 4 {
                    valid = false;
                    break;
                }
                let Some((normal, point)) = quads.remove(&edges(face)) else {
                    valid = false;
                    break;
                };
                let actual = outward(face)?;
                if normal.dot(actual) <= 0.
                    || normal.cross(actual).norm() > angular
                    || !on_plane(face, point, budget)?
                {
                    valid = false;
                    break;
                }
            }
            if valid && quads.is_empty() {
                return Ok(PlanarPrismCertificate {
                    cap_faces: [i, j],
                    translation: delta,
                    profile_corners: n,
                    profile_holes: a.wires.len() - 1,
                });
            }
        }
    }
    Err(DOMAIN)
}
