//! Guarded vertex-contact partition for internally generated convex bevel planes.
use crate::*;
use std::collections::BTreeMap;
pub(crate) fn split_convex_with_vertex_contacts(
    s: &Solid,
    plane: &Surface,
    t: GeometryTolerance,
) -> Result<SolidPlaneSplit> {
    if s.shell.faces.len() > 512
        || s.shell
            .faces
            .iter()
            .flat_map(|f| &f.wires)
            .map(|w| w.coedges.len())
            .sum::<usize>()
            > 4096
    {
        return Err(Error::Unsupported("contact split resource budget exceeded"));
    }
    let patches = planar_face_patches(s, t.absolute())?;
    let Surface::Plane { origin, u, v } = *plane else {
        return Err(Error::Unsupported(
            "contact split requires a planar cutting surface",
        ));
    };
    Frame3::new(origin, [u, v, u.cross(v)], t.absolute())?;
    let n = u.cross(v);
    let extent = s.bounds().max - s.bounds().min;
    let scale = extent.x.hypot(extent.y).hypot(extent.z);
    let band = t.length_at_scale(scale)?;
    let world = s.vertices.iter().fold(scale, |w, p| {
        w.max(p.point.x.abs())
            .max(p.point.y.abs())
            .max(p.point.z.abs())
    });
    let world = s.shell.faces.iter().try_fold(world, |w, f| {
        let Surface::Plane { origin, u, v } = f.surface else {
            return Err(Error::Unsupported("contact split requires planes"));
        };
        Frame3::new(origin, [u, v, u.cross(v)], t.absolute())?;
        Ok::<_, Error>(
            w.max(origin.x.abs())
                .max(origin.y.abs())
                .max(origin.z.abs()),
        )
    })?;
    let world = world
        .max(origin.x.abs())
        .max(origin.y.abs())
        .max(origin.z.abs());
    let arith = 4096. * f64::EPSILON * world;
    if !arith.is_finite() || arith >= t.linear() / 8. {
        return Err(Error::Unsupported(
            "contact split world-coordinate precision is unresolved",
        ));
    }
    for face in &s.shell.faces {
        if face.wires.len() != 1 {
            return Err(Error::Unsupported(
                "contact split requires source faces without inner wires",
            ));
        }
        let Surface::Plane {
            origin: o,
            u: fu,
            v: fv,
        } = face.surface
        else {
            return Err(Error::Unsupported(
                "contact split requires planar source faces",
            ));
        };
        let outward = fu.cross(fv).normalized()? * f64::from(face.orientation);
        let own: std::collections::BTreeSet<usize> = face.wires[0]
            .coedges
            .iter()
            .flat_map(|c| s.edges[c.edge].vertices)
            .collect();
        for (i, p) in s.vertices.iter().enumerate() {
            let d = (p.point - o).dot(outward);
            if !d.is_finite()
                || (own.contains(&i) && d.abs() > t.linear() / 4.)
                || (!own.contains(&i) && d >= -10. * band - arith)
            {
                return Err(Error::Unsupported(
                    "contact split requires strict convex supporting planes",
                ));
            }
        }
    }
    let ds: Vec<f64> = s
        .vertices
        .iter()
        .map(|p| (p.point - origin).dot(n))
        .collect();
    let mut signs = Vec::new();
    for &d in &ds {
        if !d.is_finite() {
            return Err(Error::Unsupported(
                "contact split signed vertex distance is nonfinite",
            ));
        }
        signs.push(if d.abs() <= arith {
            0
        } else if d.abs() <= 10. * band + arith {
            return Err(Error::Unsupported(
                "contact split vertex is in the unresolved near-plane band",
            ));
        } else if d > 0. {
            1
        } else {
            -1
        });
    }
    if !signs.contains(&1) || !signs.contains(&-1) {
        return Err(Error::Unsupported(
            "contact split plane must cross retained material on both sides",
        ));
    }
    // A rounded on-plane point must have a resolved nearby witness on BOTH sides.
    for i in 0..ds.len() {
        if signs[i] != 0 {
            continue;
        }
        for target in [-1, 1] {
            let mut best = f64::INFINITY;
            for e in &s.edges {
                let j = if e.vertices[0] == i {
                    e.vertices[1]
                } else if e.vertices[1] == i {
                    e.vertices[0]
                } else {
                    continue;
                };
                if signs[j] != target {
                    continue;
                }
                let delta = s.vertices[j].point - s.vertices[i].point;
                let length = delta.x.hypot(delta.y).hypot(delta.z);
                let denominator = (ds[j] - ds[i]).abs() - 2. * arith;
                if denominator <= 0. {
                    continue;
                }
                let allowance = (ds[i].abs() + arith) * length / denominator + arith;
                best = best.min(allowance);
            }
            if !best.is_finite() || best >= t.linear() / 8. {
                return Err(Error::Unsupported(
                    "contact split on-plane vertex displacement is unresolved",
                ));
            }
        }
    }
    let mut points: Vec<Point3> = s.vertices.iter().map(|p| p.point).collect();
    let mut cross = BTreeMap::new();
    for e in &s.edges {
        let [a, b] = e.vertices;
        if signs[a] * signs[b] == -1 {
            let denominator = (ds[a] - ds[b]).abs() - 2. * arith;
            let delta = points[b] - points[a];
            let length = delta.x.hypot(delta.y).hypot(delta.z);
            let allowance = length * arith / denominator + arith;
            if denominator <= 0. || !allowance.is_finite() || allowance >= t.linear() / 8. {
                return Err(Error::Unsupported(
                    "contact split strict intersection conditioning",
                ));
            }
            let magnitude = ds[a].abs().max(ds[b].abs());
            let q = (ds[a] / magnitude) / ((ds[a] / magnitude) - (ds[b] / magnitude));
            let p = points[a] + (points[b] - points[a]) * q;
            if !p.finite() || (p - origin).dot(n).abs() > t.linear() / 4. {
                return Err(Error::Unsupported(
                    "contact split intersection loses cutting-plane precision",
                ));
            }
            let id = points.len();
            points.push(p);
            cross.insert((a.min(b), a.max(b)), id);
        }
    }
    let mut children: [Vec<PlanarFacePatch>; 2] = [Vec::new(), Vec::new()];
    let mut cap = BTreeMap::<(usize, usize), i32>::new();
    for (fi, f) in s.shell.faces.iter().enumerate() {
        if f.wires.len() != 1 {
            return Err(Error::Unsupported(
                "contact split requires source faces with one outer wire",
            ));
        }
        let ids: Vec<usize> = f.wires[0]
            .coedges
            .iter()
            .map(|c| s.edges[c.edge].vertices[usize::from(!c.forward)])
            .collect();
        for (side, child) in children.iter_mut().enumerate() {
            let desired = if side == 0 { -1 } else { 1 };
            let inside = |i: usize| signs.get(i).copied().unwrap_or(0) * desired >= 0;
            let mut ring = Vec::new();
            for k in 0..ids.len() {
                let a = ids[k];
                let b = ids[(k + 1) % ids.len()];
                if inside(a) {
                    ring.push(a);
                }
                if signs[a] * signs[b] == -1 {
                    ring.push(cross[&(a.min(b), a.max(b))]);
                }
            }
            ring.dedup();
            if ring.first() == ring.last() {
                ring.pop();
            }
            if ring.len() < 3 {
                continue;
            }
            let uv: Vec<[f64; 2]> = ring
                .iter()
                .map(|&i| f.surface.parameters(points[i]))
                .collect();
            let anchor = uv[0];
            let mut area = 0.0;
            let mut all_collinear = true;
            for pair in uv[1..].windows(2) {
                all_collinear &= orient2d(anchor, pair[0], pair[1])? == Orientation::Collinear;
                let a = [pair[0][0] - anchor[0], pair[0][1] - anchor[1]];
                let b = [pair[1][0] - anchor[0], pair[1][1] - anchor[1]];
                area += (a[0] * b[1] - a[1] * b[0]) / 2.0;
            }
            if all_collinear {
                continue;
            }
            if area <= 0.0 || !area.is_finite() {
                return Err(Error::Unsupported(
                    "contact split clipped face area is unresolved",
                ));
            }
            if side == 0 {
                for k in 0..ring.len() {
                    let mut a = ring[k];
                    let mut b = ring[(k + 1) % ring.len()];
                    if !inside(a) || !inside(b) {
                        unreachable!();
                    }
                    if signs.get(a).copied().unwrap_or(0) == 0
                        && signs.get(b).copied().unwrap_or(0) == 0
                    {
                        if f.orientation > 0 {
                            std::mem::swap(&mut a, &mut b);
                        }
                        let key = (a.min(b), a.max(b));
                        *cap.entry(key).or_default() += if a < b { 1 } else { -1 };
                    }
                }
            }
            child.push(PlanarFacePatch {
                surface: patches[fi].surface.clone(),
                orientation: f.orientation,
                rings: vec![ring.iter().map(|&i| points[i]).collect()],
            });
        }
    }
    let mut outgoing = BTreeMap::new();
    for ((a, b), count) in cap {
        if count == 0 {
            continue;
        }
        if count.abs() != 1 {
            return Err(Error::Unsupported(
                "contact split section edge has ambiguous multiplicity",
            ));
        }
        let (a, b) = if count == 1 { (a, b) } else { (b, a) };
        if outgoing.insert(a, b).is_some() {
            return Err(Error::Unsupported(
                "contact split section has an ambiguous vertex branch",
            ));
        }
    }
    let start = *outgoing.keys().next().ok_or(Error::Unsupported(
        "contact split has no material section boundary",
    ))?;
    let mut ring = Vec::new();
    let mut at = start;
    loop {
        ring.push(at);
        at = outgoing
            .remove(&at)
            .ok_or(Error::Unsupported("contact split section boundary is open"))?;
        if at == start {
            break;
        }
        if ring.len() > points.len() {
            return Err(Error::Unsupported(
                "contact split section cycle exceeds its node budget",
            ));
        }
    }
    if !outgoing.is_empty() {
        return Err(Error::Unsupported(
            "contact split requires one connected convex section",
        ));
    }
    let section = PlanarFacePatch {
        surface: plane.clone(),
        orientation: 1,
        rings: vec![ring.iter().map(|&i| points[i]).collect()],
    };
    children[0].push(section.clone());
    let mut other = section.clone();
    other.orientation = -1;
    children[1].push(other);
    let negative = crate::sewing::sew_generated_planar_faces(&children[0], t)?;
    let positive = crate::sewing::sew_generated_planar_faces(&children[1], t)?;
    let total = negative.volume()? + positive.volume()?;
    if (total - s.volume()?).abs() > s.volume()?.abs() * 1e-10 {
        return Err(Error::Unsupported(
            "contact split does not conserve source volume",
        ));
    }
    Ok(SolidPlaneSplit {
        negative,
        positive,
        section: vec![section],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn stock() -> Solid {
        make_box(
            BoxSpec {
                min: Point3::new(0., 0., 0.),
                size: Vec3::new(1., 1., 1.),
            },
            Tolerance::default(),
        )
        .unwrap()
    }
    #[test]
    fn shallow_on_vertex_and_strict_root_conditioning_are_rejected() {
        let s = stock();
        let t = GeometryTolerance::default();
        let u = Vec3::new(1., 1e-6, 0.).normalized().unwrap();
        let v = Vec3::new(0., 0., -1.);
        let weak = Surface::Plane {
            origin: Point3::new(0., -4e-13, 0.),
            u,
            v,
        };
        assert!(matches!(
            split_convex_with_vertex_contacts(&s, &weak, t),
            Err(Error::Unsupported(
                "contact split on-plane vertex displacement is unresolved"
            ))
        ));
        // Both endpoint gaps clear the near-plane band, but root position is ill-conditioned.
        let shallow = Surface::Plane {
            origin: Point3::new(0., -5e-7, 0.),
            u,
            v,
        };
        assert!(matches!(
            split_convex_with_vertex_contacts(&s, &shallow, t),
            Err(Error::Unsupported(
                "contact split strict intersection conditioning"
            ))
        ));
    }
    #[test]
    fn supporting_or_remote_origin_planes_are_not_fabricated_into_cuts() {
        let s = stock();
        let t = GeometryTolerance::default();
        let supporting = Surface::Plane {
            origin: Point3::new(0., 0., 0.),
            u: Vec3::new(1., 0., 0.),
            v: Vec3::new(0., 1., 0.),
        };
        assert!(split_convex_with_vertex_contacts(&s, &supporting, t).is_err());
        let remote = Surface::Plane {
            origin: Point3::new(1e12, 0., 0.5),
            u: Vec3::new(1., 0., 0.),
            v: Vec3::new(0., 1., 0.),
        };
        assert!(matches!(
            split_convex_with_vertex_contacts(&s, &remote, t),
            Err(Error::Unsupported(
                "contact split world-coordinate precision is unresolved"
            ))
        ));
    }
}
