//! Actual-face display of a circular graph bore. Cap trim chords approximate the
//! rational circular boundary; every triangle bound includes that deviation.
use crate::*;
use std::collections::BTreeMap;
type Key = (usize, usize);
#[derive(Clone, Copy)]
struct Boundary {
    bottom: usize,
    top: usize,
    wall: usize,
    parameters: [f64; 2],
}
#[derive(Default)]
struct Cache {
    nodes: BTreeMap<Key, (usize, Point3)>,
    vertices: BTreeMap<(usize, Key), usize>,
}
impl NurbsGraphCircularHoledSolid {
    /// Conforming actual B-rep sampling with a chordal approximation of cap trim.
    /// Bounds include trim deviation and engineering arithmetic allowances; they
    /// are not interval certificates or exact circular-domain mesh coverage.
    pub fn tessellate_bounded(
        &self,
        error: f64,
        max_triangles: usize,
        tol: Tolerance,
    ) -> Result<NurbsGraphMesh> {
        self.validate(tol)?;
        if !error.is_finite() || error <= 0. || !(32..=65536).contains(&max_triangles) {
            return Err(Error::InvalidInput(
                "circular bore display error or triangle budget is invalid",
            ));
        }
        let source = self.source();
        let [width, depth, _] = source.dimensions();
        let b = source.bulge().abs();
        let r = self.radius();
        let physical_to_uv = width.recip().hypot(depth.recip());
        // A standard rational quadratic quarter has |XY'|<=3r and |XY''|<=7r.
        // h gradient components <=|b| and ||D²h||<=6|b| on the source rectangle.
        let circle_second = 7. * r
            + 54. * b * r * r * physical_to_uv * physical_to_uv
            + 7. * b * r * (width.recip() + depth.recip());
        let gradient = b * physical_to_uv;
        let mut scale = 0_f64;
        let mut ratio = 1_f64;
        for face in &self.brep().shell.faces {
            let Surface::Nurbs(s) = &face.surface else {
                return Err(Error::InvalidTopology("circular bore requires NURBS faces"));
            };
            scale = scale.max(
                s.control_points()
                    .iter()
                    .flat_map(|p| [p.x.abs(), p.y.abs(), p.z.abs()])
                    .fold(0., f64::max),
            );
            ratio = ratio.max(
                s.weights().iter().copied().fold(0., f64::max)
                    / s.weights().iter().copied().fold(f64::INFINITY, f64::min),
            );
        }
        let arithmetic =
            source.arithmetic_budget()? + 65536. * f64::EPSILON * scale * ratio * ratio * 12.;
        if !arithmetic.is_finite() || arithmetic >= error / 8. {
            return Err(Error::Tessellation(
                "circular bore display precision cannot resolve error",
            ));
        }
        let edge_limit = if b == 0. {
            f64::INFINITY
        } else {
            ((error / 2. - arithmetic) / b).sqrt()
        };
        if [circle_second, gradient, physical_to_uv]
            .iter()
            .any(|x| !x.is_finite())
        {
            return Err(Error::Tessellation(
                "circular bore curvature exceeds finite range",
            ));
        }
        let mut m = 4usize;
        loop {
            let dt = 1. / m as f64;
            let trim = 7. * r * dt * dt / 8. * (1. + gradient);
            let mut actual_second = 0_f64;
            for q in 0..4 {
                for i in 0..m {
                    let edge = Boundary {
                        bottom: 12 + q,
                        top: 16 + q,
                        wall: 6 + q,
                        parameters: [i as f64 / m as f64, (i + 1) as f64 / m as f64],
                    };
                    actual_second = actual_second.max(wall_derivatives(self.brep(), edge)?[0]);
                }
            }
            if actual_second / 2. <= error / 4.
                && trim <= error / 4.
                && circle_second * dt * dt / 2. <= error / 4.
                && 3. * r * physical_to_uv * dt <= edge_limit
            {
                break;
            }
            if m >= 4096 || 8 * m > max_triangles {
                return Err(Error::Tessellation(
                    "circular bore boundary budget exceeded",
                ));
            }
            m *= 2;
        }
        let trim = 7. * r / (8. * (m * m) as f64) * (1. + gradient);
        let solid = self.brep();
        let domain = source.source_domain();
        let center_uv = [self.center()[0] / width, self.center()[1] / depth];
        let corners = [
            [domain[0][0], domain[1][0]],
            [domain[0][1], domain[1][0]],
            [domain[0][1], domain[1][1]],
            [domain[0][0], domain[1][1]],
        ];
        let mut partitions: Vec<Vec<(f64, Option<[f64; 2]>)>> = (0..4)
            .map(|_| (0..m).map(|i| (i as f64 / m as f64, None)).collect())
            .collect();
        // Include every rectangle corner ray. Invert the standard rational
        // quarter parameter analytically; these are display partitions only.
        for corner in corners {
            let dx = width * corner[0] - self.center()[0];
            let dy = depth * corner[1] - self.center()[1];
            let (q, x, y) = if dx >= 0. && dy >= 0. {
                (0, dx, dy)
            } else if dx < 0. && dy >= 0. {
                (1, dy, -dx)
            } else if dx < 0. && dy < 0. {
                (2, -dx, -dy)
            } else {
                (3, -dy, dx)
            };
            let half = y / (x.hypot(y) + x);
            let t = std::f64::consts::SQRT_2 * half / (1. + (std::f64::consts::SQRT_2 - 1.) * half);
            if !t.is_finite() || !(0. ..=1.).contains(&t) {
                return Err(Error::Tessellation(
                    "circular bore corner ray is unresolved",
                ));
            }
            // Omit a redundant grid partition only when its displacement is
            // inside the arithmetic allowance; preserve the actual corner ray.
            partitions[q].retain(|(old, _)| *old == 0. || 3. * r * (*old - t).abs() > arithmetic);
            partitions[q].push((t, Some(corner)));
        }
        let mut samples = Vec::new();
        for (q, part) in partitions.iter_mut().enumerate() {
            part.sort_by(|a, b| a.0.total_cmp(&b.0));
            // Removing a redundant display partition must never enlarge the
            // declared maximum quarter interval beyond 1/M.
            for pass in 0..=16 {
                let mut added = Vec::new();
                for i in 0..part.len() {
                    let end = if i + 1 == part.len() {
                        1.
                    } else {
                        part[i + 1].0
                    };
                    if end - part[i].0 > 1. / m as f64 {
                        added.push((closed_lerp(part[i].0, end, 0.5), None));
                    }
                }
                if added.is_empty() {
                    break;
                }
                if pass == 16 || part.len() + added.len() > max_triangles {
                    return Err(Error::Tessellation(
                        "circular bore angular partition budget exceeded",
                    ));
                }
                part.extend(added);
                part.sort_by(|a, b| a.0.total_cmp(&b.0));
            }
            for &(t, corner) in part.iter() {
                let c = solid.shell.faces[0].wires[1]
                    .coedges
                    .iter()
                    .find(|c| c.edge == 12 + q)
                    .ok_or(Error::InvalidTopology("circular cap quarter is absent"))?;
                let inner = c.pcurve.try_evaluate(t)?;
                let outer = if let Some(p) = corner {
                    p
                } else {
                    let direction = [inner[0] - center_uv[0], inner[1] - center_uv[1]];
                    let mut ray = f64::INFINITY;
                    let mut hit = 0;
                    for axis in 0..2 {
                        if direction[axis] == 0. {
                            continue;
                        }
                        let side = usize::from(direction[axis] > 0.);
                        let distance = (domain[axis][side] - center_uv[axis]) / direction[axis];
                        if distance < ray {
                            ray = distance;
                            hit = 2 * axis + side;
                        }
                    }
                    if !ray.is_finite() || ray <= 1. {
                        return Err(Error::Tessellation(
                            "circular bore radial material is unresolved",
                        ));
                    }
                    let mut p = std::array::from_fn(|axis| {
                        (center_uv[axis] + ray * direction[axis])
                            .clamp(domain[axis][0], domain[axis][1])
                    });
                    p[hit / 2] = domain[hit / 2][hit % 2];
                    p
                };
                samples.push((q, t, inner, outer));
            }
        }
        let count = samples.len();
        let mut edges = BTreeMap::new();
        for i in 0..count {
            let next = (i + 1) % count;
            let (q, t, _, outer) = samples[i];
            let end = if samples[next].0 == q {
                samples[next].1
            } else {
                1.
            };
            edges.insert(
                (0, i),
                Boundary {
                    bottom: 12 + q,
                    top: 16 + q,
                    wall: 6 + q,
                    parameters: [t, end],
                },
            );
            let outer_next = samples[next].3;
            let mut found = None;
            for (k, c) in solid.shell.faces[0].wires[0].coedges.iter().enumerate() {
                let PCurve::Affine { origin, direction } = c.pcurve else {
                    return Err(Error::InvalidTopology(
                        "stock cap outer pcurve must be affine",
                    ));
                };
                let axis = usize::from(direction[0] == 0.);
                let fixed = 1 - axis;
                if outer[fixed] != origin[fixed] || outer_next[fixed] != origin[fixed] {
                    continue;
                }
                let range = solid.edges[c.edge].curve.range();
                let parameters = [outer[axis], outer_next[axis]]
                    .map(|x| ((x - origin[axis]) / direction[axis]).clamp(range[0], range[1]));
                let wall = (2..6)
                    .find(|&f| {
                        solid.shell.faces[f].wires[0]
                            .coedges
                            .iter()
                            .any(|use_| use_.edge == c.edge)
                    })
                    .ok_or(Error::InvalidTopology("stock wall is absent"))?;
                found = Some(Boundary {
                    bottom: c.edge,
                    top: solid.shell.faces[1].wires[0].coedges[k].edge,
                    wall,
                    parameters,
                });
                break;
            }
            edges.insert(
                (1, i),
                found.ok_or(Error::Tessellation(
                    "circular bore outer radial edge crosses a stock corner",
                ))?,
            );
        }
        let mut layers = 1usize;
        let (uv, triangles) = loop {
            if 4 * count * layers > max_triangles {
                return Err(Error::Tessellation(
                    "circular bore cap triangle budget exceeded",
                ));
            }
            let mut uv = Vec::with_capacity(count * (layers + 1));
            for radial in 0..=layers {
                for &(_, _, inner, outer) in &samples {
                    uv.push(std::array::from_fn(|axis| {
                        closed_lerp(inner[axis], outer[axis], radial as f64 / layers as f64)
                    }));
                }
            }
            let mut triangles = Vec::with_capacity(2 * count * layers);
            let mut resolved = true;
            for radial in 0..layers {
                for i in 0..count {
                    let next = (i + 1) % count;
                    let a = radial * count + i;
                    let z = radial * count + next;
                    let c = (radial + 1) * count + i;
                    let d = (radial + 1) * count + next;
                    for t in [[a, c, d], [a, d, z]] {
                        let diameter = triangle_diameter(t, &uv);
                        if b * diameter * diameter + trim + arithmetic > error {
                            resolved = false;
                        }
                        triangles.push(t);
                    }
                }
            }
            if resolved {
                break (uv, triangles);
            }
            if layers >= 4096 {
                return Err(Error::Tessellation(
                    "circular bore cap refinement is unresolved",
                ));
            }
            layers *= 2;
        };
        for &triangle in &triangles {
            if orient2d(uv[triangle[0]], uv[triangle[1]], uv[triangle[2]])?
                != Orientation::CounterClockwise
                || triangle_quality(triangle, &uv, [width, depth]) <= 4. * tol.linear + arithmetic
            {
                return Err(Error::Tessellation(
                    "circular bore cap orientation or altitude is unresolved",
                ));
            }
        }
        let mut wall_bounds = BTreeMap::new();
        let mut height_required = 1_f64;
        for (&key, &edge) in &edges {
            let derivatives = wall_derivatives(solid, edge)?;
            let remaining = error - arithmetic - derivatives[0] / 2.;
            if !remaining.is_finite() || remaining <= 0. {
                return Err(Error::Tessellation(
                    "circular bore wall curvature is unresolved",
                ));
            }
            height_required = height_required.max((2. * derivatives[1] / remaining).ceil());
            wall_bounds.insert(key, derivatives);
        }
        if !height_required.is_finite() || height_required > 65536. {
            return Err(Error::Tessellation(
                "circular bore wall height budget exceeded",
            ));
        }
        let height = height_required as usize;
        if 4 * count * (layers + height) > max_triangles {
            return Err(Error::Tessellation(
                "circular bore wall triangle budget exceeded",
            ));
        }
        let mut out = NurbsGraphMesh {
            mesh: Mesh::default(),
            vertex_nodes: vec![],
            vertex_uv: vec![],
            vertex_faces: vec![],
            error_bounds: vec![],
            subdivisions: m,
        };
        let mut cache = Cache::default();
        let mut cap_positions: BTreeMap<Key, Point3> = BTreeMap::new();
        for (&(outer, i), edge) in &edges {
            for (index, t) in [
                (i, edge.parameters[0]),
                ((i + 1) % count, edge.parameters[1]),
            ] {
                let id = outer * layers * count + index;
                for cap in 0..2 {
                    let point = solid.edges[if cap == 0 { edge.bottom } else { edge.top }]
                        .curve
                        .try_evaluate(t)?;
                    if let Some(previous) = cap_positions.get(&(id, cap * height)) {
                        if (*previous - point).norm() > arithmetic {
                            return Err(Error::Tessellation(
                                "circular bore boundary endpoints disagree",
                            ));
                        }
                    } else {
                        cap_positions.insert((id, cap * height), point);
                    }
                }
            }
        }
        for cap in 0..2 {
            for &t in &triangles {
                let bound = if cap == 0 {
                    trim + arithmetic
                } else {
                    b * triangle_diameter(t, &uv).powi(2) + trim + arithmetic
                };
                let nodes = t.map(|id| {
                    (
                        (id, cap * height),
                        uv[id],
                        cap_positions.get(&(id, cap * height)).copied(),
                    )
                });
                emit(
                    &mut out,
                    &mut cache,
                    &solid.shell.faces[cap],
                    cap,
                    nodes,
                    bound,
                    arithmetic,
                )?;
            }
        }
        for (&(outer, i), edge) in &edges {
            let face = &solid.shell.faces[edge.wall];
            let coedge = face.wires[0]
                .coedges
                .iter()
                .find(|c| c.edge == edge.bottom)
                .ok_or(Error::InvalidTopology("wall bottom pcurve is absent"))?;
            let a = outer * layers * count + i;
            let z = outer * layers * count + (i + 1) % count;
            let va = coedge.pcurve.try_evaluate(edge.parameters[0])?;
            let vz = coedge.pcurve.try_evaluate(edge.parameters[1])?;
            let (a, z, va, vz) = if va[0] <= vz[0] {
                (a, z, va, vz)
            } else {
                (z, a, vz, va)
            };
            let derivatives = wall_bounds[&(outer, i)];
            let bound = derivatives[0] / 2. + derivatives[1] / height as f64 + arithmetic;
            let node = |id, p: [f64; 2], j| {
                (
                    (id, j),
                    [p[0], j as f64 / height as f64],
                    cap_positions.get(&(id, j)).copied(),
                )
            };
            for j in 0..height {
                let v = [
                    node(a, va, j),
                    node(z, vz, j),
                    node(z, vz, j + 1),
                    node(a, va, j + 1),
                ];
                emit(
                    &mut out,
                    &mut cache,
                    face,
                    edge.wall,
                    [v[0], v[1], v[2]],
                    bound,
                    arithmetic,
                )?;
                emit(
                    &mut out,
                    &mut cache,
                    face,
                    edge.wall,
                    [v[0], v[2], v[3]],
                    bound,
                    arithmetic,
                )?;
            }
        }
        Ok(out)
    }
}
fn closed_lerp(a: f64, b: f64, t: f64) -> f64 {
    if t == 0. {
        a
    } else if t == 1. {
        b
    } else {
        (a + (b - a) * t).clamp(a.min(b), a.max(b))
    }
}
#[allow(clippy::too_many_arguments)]
fn emit(
    out: &mut NurbsGraphMesh,
    cache: &mut Cache,
    face: &Face,
    fi: usize,
    nodes: [(Key, [f64; 2], Option<Point3>); 3],
    bound: f64,
    arithmetic: f64,
) -> Result<()> {
    let Surface::Nurbs(s) = &face.surface else {
        return Err(Error::InvalidTopology(
            "circular bore display requires NURBS",
        ));
    };
    let mut triangle = [0; 3];
    for (i, (key, uv, preferred)) in nodes.into_iter().enumerate() {
        if let Some(&id) = cache.vertices.get(&(fi, key)) {
            triangle[i] = id;
            continue;
        }
        let jet = s.evaluate_with_partials(uv[0], uv[1], [KnotSide::Right; 2])?;
        let next = cache.nodes.len();
        let (node, point) = *cache
            .nodes
            .entry(key)
            .or_insert((next, preferred.unwrap_or(jet.point)));
        if (point - jet.point).norm() > arithmetic {
            return Err(Error::Tessellation(
                "circular bore shared face evaluations disagree",
            ));
        }
        let id = out.mesh.positions.len();
        cache.vertices.insert((fi, key), id);
        triangle[i] = id;
        out.mesh.positions.push(point);
        out.mesh
            .normals
            .push(jet.normal()? * face.orientation as f64);
        out.vertex_nodes.push(node);
        out.vertex_uv.push(uv);
        out.vertex_faces.push(fi);
    }
    if face.orientation < 0 {
        triangle.swap(1, 2);
    }
    let normal = (out.mesh.positions[triangle[1]] - out.mesh.positions[triangle[0]])
        .cross(out.mesh.positions[triangle[2]] - out.mesh.positions[triangle[0]])
        .normalized()
        .map_err(|_| Error::Tessellation("circular bore triangle area is unresolved"))?;
    if triangle
        .iter()
        .any(|&v| normal.dot(out.mesh.normals[v]) <= 0.)
    {
        return Err(Error::Tessellation(
            "circular bore triangle orientation is unresolved",
        ));
    }
    out.mesh.triangles.push(triangle);
    out.mesh.face_ids.push(fi);
    out.error_bounds.push(bound);
    Ok(())
}

// Actual retained rational Bezier control bounds on a parameter subinterval.
// Homogeneous subdivision preserves positive raw weights. The derivative
// recurrence follows H=W*C; translate controls before forming differences.
fn restricted_controls(curve: &Curve, a: f64, b: f64) -> Result<(Vec<Point3>, Vec<f64>)> {
    let Curve::Nurbs(curve) = curve else {
        return Err(Error::InvalidTopology(
            "circular bore boundary requires NURBS",
        ));
    };
    if curve.control_points().len() != curve.degree() + 1 {
        return Err(Error::Unsupported(
            "circular bore display requires Bezier boundary curves",
        ));
    }
    let range = curve.domain();
    let lo = (a.min(b) - range[0]) / (range[1] - range[0]);
    let hi = (a.max(b) - range[0]) / (range[1] - range[0]);
    if !lo.is_finite() || !hi.is_finite() || lo < 0. || hi > 1. || lo >= hi {
        return Err(Error::Tessellation(
            "circular bore curve interval is unresolved",
        ));
    }
    let origin = curve.control_points()[0];
    let mut h: Vec<[f64; 4]> = curve
        .control_points()
        .iter()
        .zip(curve.weights())
        .map(|(p, &w)| {
            let p = *p - origin;
            [p.x * w, p.y * w, p.z * w, w]
        })
        .collect();
    if hi < 1. {
        h = split_homogeneous(&h, hi).0;
    }
    if lo > 0. {
        h = split_homogeneous(&h, lo / hi).1;
    }
    let mut points = Vec::new();
    let mut weights = Vec::new();
    for p in h {
        let point = origin + Vec3::new(p[0] / p[3], p[1] / p[3], p[2] / p[3]);
        if p[3] <= 0. || !p[3].is_finite() || !point.finite() {
            return Err(Error::Tessellation(
                "circular bore homogeneous subdivision is unresolved",
            ));
        }
        points.push(point);
        weights.push(p[3]);
    }
    Ok((points, weights))
}
fn split_homogeneous(h: &[[f64; 4]], t: f64) -> (Vec<[f64; 4]>, Vec<[f64; 4]>) {
    let mut work = h.to_vec();
    let n = h.len();
    let mut left = vec![h[0]];
    let mut right = vec![h[n - 1]];
    for count in (1..n).rev() {
        for i in 0..count {
            let next = work[i + 1];
            for (value, next) in work[i].iter_mut().zip(next) {
                *value = (1. - t) * *value + t * next;
            }
        }
        left.push(work[0]);
        right.push(work[count - 1]);
    }
    right.reverse();
    (left, right)
}
fn curve_derivatives(points: &[Point3], weights: &[f64]) -> Result<[f64; 2]> {
    let maximum = weights.iter().copied().fold(0., f64::max);
    let w: Vec<_> = weights.iter().map(|x| x / maximum).collect();
    let minimum = w.iter().copied().fold(f64::INFINITY, f64::min);
    let local: Vec<_> = points.iter().map(|p| *p - points[0]).collect();
    let diameter = local.iter().map(|p| p.norm()).fold(0., f64::max);
    let h: Vec<_> = local.iter().zip(&w).map(|(p, w)| *p * *w).collect();
    let degree = (points.len() - 1) as f64;
    let h1 = degree
        * h.windows(2)
            .map(|x| (x[1] - x[0]).norm())
            .fold(0., f64::max);
    let w1 = degree * w.windows(2).map(|x| (x[1] - x[0]).abs()).fold(0., f64::max);
    let h2 = degree
        * (degree - 1.)
        * h.windows(3)
            .map(|x| (x[2] - x[1] * 2. + x[0]).norm())
            .fold(0., f64::max);
    let w2 = degree
        * (degree - 1.)
        * w.windows(3)
            .map(|x| (x[2] - 2. * x[1] + x[0]).abs())
            .fold(0., f64::max);
    let first = (h1 + diameter * w1) / minimum;
    let second = (h2 + diameter * w2 + 2. * w1 * first) / minimum;
    if !first.is_finite() || !second.is_finite() {
        return Err(Error::Tessellation(
            "circular bore rational derivative bound is unresolved",
        ));
    }
    Ok([first, second])
}
fn wall_derivatives(solid: &Solid, edge: Boundary) -> Result<[f64; 2]> {
    let (bottom, bw) = restricted_controls(
        &solid.edges[edge.bottom].curve,
        edge.parameters[0],
        edge.parameters[1],
    )?;
    let (top, tw) = restricted_controls(
        &solid.edges[edge.top].curve,
        edge.parameters[0],
        edge.parameters[1],
    )?;
    if bw != tw || bottom.len() != top.len() {
        return Err(Error::Tessellation(
            "circular bore ruled boundary weights disagree",
        ));
    }
    let second = curve_derivatives(&bottom, &bw)?[1].max(curve_derivatives(&top, &tw)?[1]);
    let difference: Vec<_> = top.iter().zip(&bottom).map(|(a, b)| *a - *b).collect();
    let mixed = curve_derivatives(&difference, &tw)?[0];
    Ok([second, mixed])
}
fn triangle_quality(t: [usize; 3], uv: &[[f64; 2]], physical: [f64; 2]) -> f64 {
    let p = t.map(|i| [uv[i][0] * physical[0], uv[i][1] * physical[1]]);
    let area = ((p[1][0] - p[0][0]) * (p[2][1] - p[0][1])
        - (p[1][1] - p[0][1]) * (p[2][0] - p[0][0]))
        .abs();
    let mut quality = f64::INFINITY;
    for i in 0..3 {
        let length = (p[i][0] - p[(i + 1) % 3][0]).hypot(p[i][1] - p[(i + 1) % 3][1]);
        quality = quality.min(length).min(area / length);
    }
    quality
}

fn triangle_diameter(t: [usize; 3], uv: &[[f64; 2]]) -> f64 {
    (0..3)
        .map(|i| (uv[t[i]][0] - uv[t[(i + 1) % 3]][0]).hypot(uv[t[i]][1] - uv[t[(i + 1) % 3]][1]))
        .fold(0., f64::max)
}
