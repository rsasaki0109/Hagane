//! Conforming display of actual restricted torus walls and their annular cap.
use crate::*;
fn failure() -> Error {
    Error::Unsupported("horizontal torus display precision or triangle budget is unresolved")
}
fn angle(t: f64) -> f64 {
    if t == 0. {
        0.
    } else if t == 1. {
        std::f64::consts::FRAC_PI_2
    } else {
        2. * (std::f64::consts::FRAC_1_SQRT_2 * t
            / (1. - (1. - std::f64::consts::FRAC_1_SQRT_2) * t))
            .atan()
    }
}
fn parameter(a: f64) -> f64 {
    let q = (a / 2.).tan();
    std::f64::consts::SQRT_2 * q / (1. + (std::f64::consts::SQRT_2 - 1.) * q)
}
impl NurbsTorusHorizontalSolid {
    /// Actual retained BRep tessellation. Angular Hessian and trim chord bounds
    /// include floating/frame/cache reserves; these are engineering bounds.
    pub fn tessellate(&self, chord: f64, policy: GeometryTolerance) -> Result<Mesh> {
        self.validate(policy)?;
        if !chord.is_finite() || chord <= 0. {
            return Err(Error::InvalidInput(
                "horizontal torus chord tolerance must be positive and finite",
            ));
        }
        let source = self.source();
        let [rmajor, rminor] = source.radii();
        let frame = source.frame();
        let axes = frame.axes();
        let o = frame.origin();
        let world = o.x.abs().max(o.y.abs()).max(o.z.abs()) + 4. * (rmajor + rminor);
        let gram = (0..3)
            .flat_map(|i| {
                (0..3).map(move |j| (axes[i].dot(axes[j]) - if i == j { 1. } else { 0. }).abs())
            })
            .fold(0., f64::max);
        let reserve = 524288. * f64::EPSILON * world + 32. * gram * (rmajor + rminor);
        let coefficient =
            (rmajor + 4. * rminor) * std::f64::consts::PI.powi(2) / 32. * (1. + 4. * gram);
        if !reserve.is_finite() || !coefficient.is_finite() || chord <= reserve {
            return Err(failure());
        }
        let strips = self.strip_ranges();
        let m = strips.len();
        let mut n = 1usize;
        while coefficient / (n * n) as f64 + reserve > chord {
            n *= 2;
            if n > 32 {
                return Err(failure());
            }
        }
        if 8 * m * n * n + 8 * n > 65536 {
            return Err(failure());
        }
        let major: Vec<_> = (0..=n)
            .map(|a| {
                if a == 0 {
                    0.
                } else if a == n {
                    1.
                } else {
                    parameter(std::f64::consts::FRAC_PI_2 * a as f64 / n as f64)
                }
            })
            .collect();
        let mut minor = Vec::new();
        let mut edge_t = Vec::new();
        for &(_, [va, vb]) in strips {
            let aa = angle(va);
            let ab = angle(vb);
            let mut v = Vec::new();
            let mut t = Vec::new();
            for b in 0..=n {
                let value = if b == 0 {
                    va
                } else if b == n {
                    vb
                } else {
                    parameter(aa + (ab - aa) * b as f64 / n as f64)
                };
                let q = if b == 0 {
                    0.
                } else if b == n {
                    1.
                } else {
                    (value - va) / (vb - va)
                };
                if !value.is_finite()
                    || !q.is_finite()
                    || !(va..=vb).contains(&value)
                    || !(0. ..=1.).contains(&q)
                {
                    return Err(failure());
                }
                v.push(value);
                t.push(q);
            }
            minor.push(v);
            edge_t.push(t);
        }
        let body = self.solid();
        let major_count = 4 * (m + 1);
        let mut edges = Vec::new();
        for (id, edge) in body.edges.iter().enumerate() {
            let ts = if id < major_count {
                &major
            } else {
                &edge_t[(id - major_count) % m]
            };
            let mut points = Vec::new();
            for (k, &t) in ts.iter().enumerate() {
                points.push(if k == 0 {
                    body.vertices[edge.vertices[0]].point
                } else if k == n {
                    body.vertices[edge.vertices[1]].point
                } else {
                    edge.curve.try_evaluate(t)?
                });
            }
            edges.push(points);
        }
        let mut mesh = Mesh::default();
        for i in 0..4 {
            for (k, vs) in minor.iter().enumerate() {
                let id = i * m + k;
                let face = &body.shell.faces[id];
                let mut points = Vec::new();
                let mut normals = Vec::new();
                for (a, &u) in major.iter().enumerate() {
                    for (b, &v) in vs.iter().enumerate() {
                        let exact = face.surface.try_evaluate(u, v)?;
                        let point = if b == 0 {
                            edges[i * (m + 1) + k][a]
                        } else if b == n {
                            edges[i * (m + 1) + k + 1][a]
                        } else if a == 0 {
                            edges[major_count + i * m + k][b]
                        } else if a == n {
                            edges[major_count + ((i + 1) % 4) * m + k][b]
                        } else {
                            exact
                        };
                        if (point - exact).norm() > reserve / 4. {
                            return Err(failure());
                        }
                        points.push(point);
                        normals.push(face.surface.normal_at(u, v)?);
                    }
                }
                for a in 0..n {
                    for b in 0..n {
                        let z = a * (n + 1) + b;
                        for tri in [[z, z + n + 1, z + n + 2], [z, z + n + 2, z + 1]] {
                            crate::nurbs_frustum_display::emit(
                                &mut mesh,
                                tri.map(|x| points[x]),
                                tri.map(|x| normals[x]),
                                id,
                                face.orientation,
                            )?;
                        }
                    }
                }
            }
        }
        let capid = 4 * m;
        let cap = &body.shell.faces[capid];
        let normal = axes[2];
        let radial = |point: Point3| {
            let q = frame.local_point(point);
            q.x.hypot(q.y)
        };
        for i in 0..4 {
            let first = &edges[i * (m + 1)];
            let last = &edges[i * (m + 1) + m];
            let (outer, inner) = if radial(first[0]) > radial(last[0]) {
                (first, last)
            } else {
                (last, first)
            };
            for a in 0..n {
                for tri in [
                    [outer[a], outer[a + 1], inner[a + 1]],
                    [outer[a], inner[a + 1], inner[a]],
                ] {
                    crate::nurbs_frustum_display::emit(
                        &mut mesh,
                        tri,
                        [normal; 3],
                        capid,
                        cap.orientation,
                    )?;
                }
            }
        }
        Ok(mesh)
    }
}
