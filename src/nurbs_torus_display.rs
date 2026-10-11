//! Conforming display of retained rational torus patches.
use crate::*;
fn failure() -> Error {
    Error::Unsupported("torus display arithmetic or triangle budget is unresolved")
}
fn parameter(k: usize, n: usize) -> f64 {
    if k == 0 {
        return 0.;
    }
    if k == n {
        return 1.;
    }
    let q = (std::f64::consts::FRAC_PI_4 * k as f64 / n as f64).tan();
    std::f64::consts::SQRT_2 * q / (1. + (std::f64::consts::SQRT_2 - 1.) * q)
}
impl NurbsTorusSolid {
    /// Actual BRep display, with a whole-triangle angular Hessian bound and
    /// engineering floating-point reserves. This is not interval certification.
    pub fn tessellate(&self, chord: f64, policy: GeometryTolerance) -> Result<Mesh> {
        self.validate(policy)?;
        if !chord.is_finite() || chord <= 0. {
            return Err(Error::InvalidInput(
                "torus chord tolerance must be positive and finite",
            ));
        }
        let [rmajor, rminor] = self.radii();
        let axes = self.frame().axes();
        let o = self.frame().origin();
        let world = o.x.abs().max(o.y.abs()).max(o.z.abs()) + 4. * (rmajor + rminor);
        let gram = (0..3)
            .flat_map(|i| {
                (0..3).map(move |j| (axes[i].dot(axes[j]) - if i == j { 1. } else { 0. }).abs())
            })
            .fold(0., f64::max);
        // Positive tensor weights >=1/2. Reserve covers rounded controls,
        // rational inverse-angle evaluation and canonical edge replacement.
        let reserve = 262144. * f64::EPSILON * world + 16. * gram * (rmajor + rminor);
        // Each angular variance <=h²/4 and covariance <=h²/4.
        // The Taylor remainder is bounded by h²(R+4r)/8, including twist.
        let coefficient =
            (rmajor + 4. * rminor) * std::f64::consts::PI.powi(2) / 32. * (1. + 4. * gram);
        if !reserve.is_finite() || !coefficient.is_finite() || chord <= reserve {
            return Err(failure());
        }
        let mut n = 1usize;
        while coefficient / (n * n) as f64 + reserve > chord {
            n *= 2;
            if n > 32 {
                return Err(failure());
            }
        }
        let body = self.solid();
        let ts: Vec<_> = (0..=n).map(|k| parameter(k, n)).collect();
        let mut edge_points = Vec::new();
        for edge in &body.edges {
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
            edge_points.push(points);
        }
        let mut mesh = Mesh::default();
        for i in 0..4 {
            for j in 0..4 {
                let id = 4 * i + j;
                let face = &body.shell.faces[id];
                let mut points = Vec::new();
                let mut normals = Vec::new();
                for a in 0..=n {
                    for b in 0..=n {
                        let exact = face.surface.try_evaluate(ts[a], ts[b])?;
                        let point = if b == 0 {
                            edge_points[4 * i + j][a]
                        } else if b == n {
                            edge_points[4 * i + (j + 1) % 4][a]
                        } else if a == 0 {
                            edge_points[16 + 4 * i + j][b]
                        } else if a == n {
                            edge_points[16 + 4 * ((i + 1) % 4) + j][b]
                        } else {
                            exact
                        };
                        if (point - exact).norm() > reserve / 4. {
                            return Err(failure());
                        }
                        points.push(point);
                        normals.push(face.surface.normal_at(ts[a], ts[b])?);
                    }
                }
                for a in 0..n {
                    for b in 0..n {
                        let k = a * (n + 1) + b;
                        for indices in [[k, k + n + 1, k + n + 2], [k, k + n + 2, k + 1]] {
                            crate::nurbs_frustum_display::emit(
                                &mut mesh,
                                indices.map(|x| points[x]),
                                indices.map(|x| normals[x]),
                                id,
                                face.orientation,
                            )?;
                        }
                    }
                }
            }
        }
        Ok(mesh)
    }
}
