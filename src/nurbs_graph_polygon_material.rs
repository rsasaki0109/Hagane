//! Checked straight-UV multiply connected material decomposition; no three-dimensional mesh Boolean.
use crate::*;
use std::collections::{BTreeMap, BTreeSet};
type EdgeUses = BTreeMap<(usize, usize), Vec<(usize, usize, usize)>>;
pub(crate) struct NurbsGraphMaterial {
    pub points: Vec<[f64; 2]>,
    pub triangles: Vec<[usize; 3]>,
}
pub(crate) fn material_triangles(
    outer: &[[f64; 2]],
    inner: &[[f64; 2]],
    physical: [f64; 2],
    margin: f64,
) -> Result<NurbsGraphMaterial> {
    material_regions(outer, &[inner.to_vec()], physical, margin)
}
pub(crate) fn material_regions(
    outer: &[[f64; 2]],
    holes: &[Vec<[f64; 2]>],
    physical: [f64; 2],
    margin: f64,
) -> Result<NurbsGraphMaterial> {
    if !(3..=16).contains(&outer.len())
        || !(1..=4).contains(&holes.len())
        || holes.iter().any(|h| !(3..=16).contains(&h.len()))
    {
        return Err(Error::Unsupported(
            "polygon material supports 1 through 4 openings with 3 through 16 corners per wire",
        ));
    }
    let n = outer.len() + holes.iter().map(Vec::len).sum::<usize>();
    if n > 64 {
        return Err(Error::Unsupported(
            "polygon material exceeds 64 total corners",
        ));
    }
    let points: Vec<_> = outer
        .iter()
        .chain(holes.iter().flatten())
        .copied()
        .collect();
    let mut offsets = Vec::new();
    let mut offset = outer.len();
    for hole in holes {
        offsets.push(offset);
        offset += hole.len();
    }
    let target = n + 2 * holes.len() - 2;
    let coords: Vec<_> = points.iter().flat_map(|p| p.iter().copied()).collect();
    let indices = earcutr::earcut(&coords, &offsets, 2)
        .map_err(|_| Error::InvalidTopology("UV annulus triangulation failed"))?;
    let bad = || {
        Error::InvalidTopology(
            "UV annulus decomposition is not a conforming connected oriented material region",
        )
    };
    if indices.len() > 3 * target || indices.len() % 3 != 0 {
        return Err(bad());
    }
    let mut triangles = Vec::new();
    let mut edges: EdgeUses = BTreeMap::new();
    for ids in indices.as_chunks::<3>().0 {
        let mut tri = [ids[0], ids[1], ids[2]];
        if tri.iter().any(|i| *i >= n) {
            return Err(bad());
        }
        match orient2d(points[tri[0]], points[tri[1]], points[tri[2]])? {
            Orientation::Clockwise => tri.swap(1, 2),
            Orientation::CounterClockwise => (),
            _ => return Err(bad()),
        };
        triangles.push(tri);
    }
    // Earcut can remove aligned bridge vertices. Split a triangle edge only
    // at an existing EXACTLY collinear input vertex in its open segment. The
    // positive subtriangles cover precisely the same triangle, without nudging
    // coordinates or creating Steiner geometry. Final embedding checks remain.
    for _ in 0..n * n {
        let mut split = None;
        'find: for (id, tri) in triangles.iter().enumerate() {
            for edge in 0..3 {
                let a = tri[edge];
                let b = tri[(edge + 1) % 3];
                let c = tri[(edge + 2) % 3];
                for (v, p) in points.iter().enumerate() {
                    if tri.contains(&v) {
                        continue;
                    }
                    if orient2d(points[a], points[b], *p)? == Orientation::Collinear
                        && p[0] >= points[a][0].min(points[b][0])
                        && p[0] <= points[a][0].max(points[b][0])
                        && p[1] >= points[a][1].min(points[b][1])
                        && p[1] <= points[a][1].max(points[b][1])
                        && *p != points[a]
                        && *p != points[b]
                    {
                        split = Some((id, [a, v, c], [v, b, c]));
                        break 'find;
                    }
                }
            }
        }
        let Some((id, first, second)) = split else {
            break;
        };
        if triangles.len() >= target
            || orient2d(points[first[0]], points[first[1]], points[first[2]])?
                != Orientation::CounterClockwise
            || orient2d(points[second[0]], points[second[1]], points[second[2]])?
                != Orientation::CounterClockwise
        {
            return Err(bad());
        }
        triangles[id] = first;
        triangles.push(second);
    }
    // Earcut may omit a rounded nearly-collinear bridge triangle. Restore
    // only a bounded, uniquely oriented triangular gap, without moving a vertex.
    for _ in triangles.len()..target {
        let mut expected_boundary = BTreeSet::new();
        for i in 0..outer.len() {
            expected_boundary.insert((i, (i + 1) % outer.len()));
        }
        for (hole, offset) in holes.iter().zip(&offsets) {
            for i in 0..hole.len() {
                expected_boundary.insert((offset + (i + 1) % hole.len(), offset + i));
            }
        }
        let mut uses: EdgeUses = BTreeMap::new();
        for (id, tri) in triangles.iter().enumerate() {
            for i in 0..3 {
                let a = tri[i];
                let b = tri[(i + 1) % 3];
                uses.entry((a.min(b), a.max(b)))
                    .or_default()
                    .push((a, b, id));
            }
        }
        let mut missing = BTreeSet::new();
        for usage in uses.values() {
            if usage.len() == 1 {
                let (a, b, _) = usage[0];
                if !expected_boundary.remove(&(a, b)) {
                    missing.insert((b, a));
                }
            } else if usage.len() != 2 {
                return Err(bad());
            }
        }
        missing.extend(expected_boundary);
        let Some(&(a, b)) = missing.first() else {
            return Err(bad());
        };
        let candidates: Vec<_> = missing
            .iter()
            .filter_map(|(x, c)| {
                if *x == b && missing.contains(&(*c, a)) {
                    Some(*c)
                } else {
                    None
                }
            })
            .collect();
        if candidates.len() != 1 {
            return Err(bad());
        }
        let c = candidates[0];
        if orient2d(points[a], points[b], points[c])? != Orientation::CounterClockwise {
            return Err(Error::Unsupported(
                "UV annulus omitted bridge cannot be restored with a positive exact triangle",
            ));
        }
        triangles.push([a, b, c]);
    }
    // Repair only internal diagonals, never input geometry. A strict convex
    // quadrilateral flip must improve the worst physical altitude of its pair.
    let mut flip_attempts = 0;
    'repair: for _ in 0..n * n {
        let mut changed = false;
        'search: for i in 0..triangles.len() {
            if quality(triangles[i], &points, physical) > margin {
                continue;
            }
            for edge in 0..3 {
                let a = triangles[i][edge];
                let b = triangles[i][(edge + 1) % 3];
                let c = triangles[i][(edge + 2) % 3];
                for j in 0..triangles.len() {
                    if i == j || !triangles[j].contains(&a) || !triangles[j].contains(&b) {
                        continue;
                    }
                    let Some(d) = triangles[j].iter().copied().find(|v| *v != a && *v != b) else {
                        continue;
                    };
                    if flip_attempts >= n * n {
                        break 'repair;
                    }
                    flip_attempts += 1;
                    let s1 = orient2d(points[a], points[b], points[c])?;
                    let s2 = orient2d(points[a], points[b], points[d])?;
                    let s3 = orient2d(points[c], points[d], points[a])?;
                    let s4 = orient2d(points[c], points[d], points[b])?;
                    if s1 == Orientation::Collinear
                        || s2 == Orientation::Collinear
                        || s3 == Orientation::Collinear
                        || s4 == Orientation::Collinear
                        || s1 == s2
                        || s3 == s4
                    {
                        continue;
                    }
                    let mut first = [c, d, a];
                    let mut second = [d, c, b];
                    if orient2d(points[first[0]], points[first[1]], points[first[2]])?
                        == Orientation::Clockwise
                    {
                        first.swap(1, 2);
                    }
                    if orient2d(points[second[0]], points[second[1]], points[second[2]])?
                        == Orientation::Clockwise
                    {
                        second.swap(1, 2);
                    }
                    let old = quality(triangles[i], &points, physical).min(quality(
                        triangles[j],
                        &points,
                        physical,
                    ));
                    let next =
                        quality(first, &points, physical).min(quality(second, &points, physical));
                    if next > old * 1.01 && next.is_finite() {
                        triangles[i] = first;
                        triangles[j] = second;
                        changed = true;
                        break 'search;
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }
    if triangles
        .iter()
        .any(|tri| quality(*tri, &points, physical) <= margin)
    {
        return Err(Error::Unsupported(
            "polygon annulus material triangles have unresolved physical altitude",
        ));
    }
    for (id, tri) in triangles.iter().enumerate() {
        for i in 0..3 {
            let a = tri[i];
            let b = tri[(i + 1) % 3];
            edges
                .entry((a.min(b), a.max(b)))
                .or_default()
                .push((a, b, id));
        }
    }
    for tri in &triangles {
        for (i, p) in points.iter().enumerate() {
            if !tri.contains(&i)
                && orient2d(points[tri[0]], points[tri[1]], *p)? == Orientation::CounterClockwise
                && orient2d(points[tri[1]], points[tri[2]], *p)? == Orientation::CounterClockwise
                && orient2d(points[tri[2]], points[tri[0]], *p)? == Orientation::CounterClockwise
            {
                return Err(bad());
            }
        }
    }
    let mut boundary = BTreeMap::new();
    for i in 0..outer.len() {
        let j = (i + 1) % outer.len();
        boundary.insert((i.min(j), i.max(j)), (i, j));
    }
    for (hole, offset) in holes.iter().zip(&offsets) {
        for i in 0..hole.len() {
            let a = offset + i;
            let b = offset + (i + 1) % hole.len();
            boundary.insert((a.min(b), a.max(b)), (b, a));
        }
    }
    let mut adjacency = vec![Vec::new(); triangles.len()];
    for (edge, uses) in &edges {
        if let Some(direction) = boundary.get(edge) {
            if uses.len() != 1 || (uses[0].0, uses[0].1) != *direction {
                return Err(bad());
            }
        } else {
            if uses.len() != 2 || uses[0].0 != uses[1].1 || uses[0].1 != uses[1].0 {
                return Err(bad());
            }
            adjacency[uses[0].2].push(uses[1].2);
            adjacency[uses[1].2].push(uses[0].2);
        }
    }
    if boundary.keys().any(|e| !edges.contains_key(e))
        || n + triangles.len() + holes.len() - 1 != edges.len()
    {
        return Err(bad());
    }
    let keys: Vec<_> = edges.keys().copied().collect();
    for (i, (a, b)) in keys.iter().copied().enumerate() {
        for (c, d) in keys.iter().copied().skip(i + 1) {
            if a != c
                && a != d
                && b != c
                && b != d
                && segments_intersect2d(points[a], points[b], points[c], points[d])?
            {
                return Err(bad());
            }
        }
        for (j, p) in points.iter().enumerate() {
            if j != a
                && j != b
                && orient2d(points[a], points[b], *p)? == Orientation::Collinear
                && p[0] >= points[a][0].min(points[b][0])
                && p[0] <= points[a][0].max(points[b][0])
                && p[1] >= points[a][1].min(points[b][1])
                && p[1] <= points[a][1].max(points[b][1])
            {
                return Err(bad());
            }
        }
    }
    let mut visited = BTreeSet::new();
    let mut stack = vec![0];
    while let Some(i) = stack.pop() {
        if visited.insert(i) {
            stack.extend(&adjacency[i]);
        }
    }
    if visited.len() != triangles.len() {
        return Err(bad());
    }
    Ok(NurbsGraphMaterial { points, triangles })
}

fn quality(tri: [usize; 3], points: &[[f64; 2]], physical: [f64; 2]) -> f64 {
    let p = tri.map(|i| points[i]);
    let mut minimum = f64::INFINITY;
    for i in 0..3 {
        let a = p[i];
        let b = p[(i + 1) % 3];
        let c = p[(i + 2) % 3];
        let d = [physical[0] * (b[0] - a[0]), physical[1] * (b[1] - a[1])];
        let length = d[0].hypot(d[1]);
        let altitude = ((d[0] / length) * (physical[1] * (c[1] - a[1]))
            - (d[1] / length) * (physical[0] * (c[0] - a[0])))
            .abs();
        if !length.is_finite() || !altitude.is_finite() {
            return 0.;
        }
        minimum = minimum.min(length).min(altitude);
    }
    minimum
}
