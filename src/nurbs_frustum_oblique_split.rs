//! Certified closed frustum pieces with an exact oblique rational cap.
use crate::*;
#[derive(Clone, Debug)]
pub struct NurbsObliqueFrustumSolid {
    source: NurbsFrustumSolid,
    section: NurbsFrustumPlaneSection,
    lower: bool,
    solid: Solid,
}
#[derive(Clone, Debug)]
pub struct NurbsFrustumPlaneSplit {
    pub lower: NurbsObliqueFrustumSolid,
    pub upper: NurbsObliqueFrustumSolid,
    pub section: NurbsFrustumPlaneSection,
}
fn unresolved() -> Error {
    Error::Unsupported(
        "oblique frustum split geometry, volume or representation precision is unresolved",
    )
}
fn norm(v: Vec3) -> f64 {
    v.x.hypot(v.y).hypot(v.z)
}
impl NurbsObliqueFrustumSolid {
    pub fn solid(&self) -> &Solid {
        &self.solid
    }
    pub fn source(&self) -> &NurbsFrustumSolid {
        &self.source
    }
    pub fn section(&self) -> &NurbsFrustumPlaneSection {
        &self.section
    }
    pub fn is_lower(&self) -> bool {
        self.lower
    }
    pub fn validate(&self, policy: GeometryTolerance) -> Result<()> {
        self.source.validate(policy)?;
        self.section.validate(policy)?;
        let expected = build(&self.source, &self.section, self.lower, policy)?;
        if !crate::nurbs_frustum::same_body(&self.solid, &expected) {
            return Err(Error::InvalidTopology(
                "oblique frustum differs from its complete actual B-rep certificate",
            ));
        }
        let mut counts = [(0usize, 0i32); 12];
        for face in &self.solid.shell.faces {
            for coedge in &face.wires[0].coedges {
                if matches!(face.surface, Surface::Plane { .. }) {
                    let (PCurve::Nurbs(uv), Curve::Nurbs(curve)) =
                        (&coedge.pcurve, &self.solid.edges[coedge.edge].curve)
                    else {
                        return Err(unresolved());
                    };
                    if uv.degree() != curve.degree()
                        || uv.knots() != curve.knots()
                        || uv.weights() != curve.weights()
                    {
                        return Err(unresolved());
                    }
                    // Identical positive rational bases make the maximum
                    // control residual a bound for the entire cap edge.
                    for (p, q) in uv.control_points().iter().zip(curve.control_points()) {
                        if norm(face.surface.try_evaluate(p.x, p.y)? - *q) > policy.linear() / 16. {
                            return Err(unresolved());
                        }
                    }
                }
                counts[coedge.edge].0 += 1;
                counts[coedge.edge].1 +=
                    i32::from(face.orientation) * if coedge.forward { 1 } else { -1 };
                for t in [0., 0.125, 0.5, 0.875, 1.] {
                    let uv = coedge.pcurve.try_evaluate(t)?;
                    let actual = face.surface.try_evaluate(uv[0], uv[1])?;
                    let edge = self.solid.edges[coedge.edge].curve.try_evaluate(t)?;
                    if norm(actual - edge) > policy.linear() / 8. {
                        return Err(unresolved());
                    }
                }
            }
        }
        if counts.iter().any(|&(n, s)| n != 2 || s != 0) {
            return Err(Error::InvalidTopology(
                "oblique frustum is not closed with opposed edge uses",
            ));
        }
        Ok(())
    }
    pub fn volume(&self, policy: GeometryTolerance) -> Result<f64> {
        self.validate(policy)?;
        let lower = lower_volume(&self.source, self.section.plane(), policy)?;
        let total = self.source.volume(policy)?;
        let v = if self.lower { lower } else { total - lower };
        if !v.is_normal() || v <= 8192. * f64::EPSILON * total {
            return Err(unresolved());
        }
        Ok(v)
    }
    /// Conservative positive-weight control-hull enclosure, not tight extrema.
    pub fn bounds(&self, policy: GeometryTolerance) -> Result<Bounds> {
        self.validate(policy)?;
        let mut min = Point3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
        let mut max = Point3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
        for edge in &self.solid.edges {
            let points: Vec<_> = match &edge.curve {
                Curve::Nurbs(c) => c.control_points().to_vec(),
                Curve::Line { a, b } => vec![*a, *b],
                _ => return Err(unresolved()),
            };
            for p in points {
                min = Point3::new(min.x.min(p.x), min.y.min(p.y), min.z.min(p.z));
                max = Point3::new(max.x.max(p.x), max.y.max(p.y), max.z.max(p.z));
            }
        }
        Ok(Bounds { min, max })
    }
    pub fn export_step_mm(&self, policy: GeometryTolerance) -> Result<String> {
        self.validate(policy)?;
        crate::nurbs_graph_step::write(&self.solid, policy.absolute())
    }
}
impl NurbsFrustumSolid {
    /// Partition into two actual closed B-reps, only for a closed section
    /// strictly between both source caps. Lower follows source-positive Z.
    pub fn split_by_plane(
        &self,
        plane: &Surface,
        policy: GeometryTolerance,
    ) -> Result<NurbsFrustumPlaneSplit> {
        let section = self.section_by_plane(plane, policy)?;
        lower_volume(self, plane, policy)?;
        let lower = NurbsObliqueFrustumSolid {
            source: self.clone(),
            solid: build(self, &section, true, policy)?,
            section: section.clone(),
            lower: true,
        };
        let upper = NurbsObliqueFrustumSolid {
            source: self.clone(),
            solid: build(self, &section, false, policy)?,
            section: section.clone(),
            lower: false,
        };
        lower.validate(policy)?;
        upper.validate(policy)?;
        Ok(NurbsFrustumPlaneSplit {
            lower,
            upper,
            section,
        })
    }
}
fn cap(
    source: &NurbsFrustumSolid,
    section: &NurbsFrustumPlaneSection,
    policy: GeometryTolerance,
) -> Result<Surface> {
    let Surface::Plane { u, v, .. } = *section.plane() else {
        return Err(unresolved());
    };
    let mut n = u.cross(v).normalized()?;
    let z = source.frame().axes()[2];
    if n.dot(z) < 0. {
        n = n * -1.;
    }
    let x = source.frame().axes()[0];
    let u = (x - n * x.dot(n)).normalized()?;
    let v = n.cross(u).normalized()?;
    let anchor = section.vertices()[0].point;
    let sum = section
        .vertices()
        .iter()
        .fold(Vec3::new(0., 0., 0.), |s, p| s + (p.point - anchor) * 0.25);
    let origin = anchor + sum;
    if !origin.finite() || !crate::geometry::plane_basis_valid(u, v, policy) {
        return Err(unresolved());
    }
    Ok(Surface::Plane { origin, u, v })
}
fn build(
    source: &NurbsFrustumSolid,
    section: &NurbsFrustumPlaneSection,
    lower: bool,
    policy: GeometryTolerance,
) -> Result<Solid> {
    let mut curves = vec![];
    for layer in 0..2 {
        for q in 0..4 {
            let curve = if (lower && layer == 0) || (!lower && layer == 1) {
                &source.solid().edges[layer * 4 + q].curve
            } else {
                &section.edges()[q].curve
            };
            let Curve::Nurbs(c) = curve else {
                return Err(unresolved());
            };
            curves.push(c.as_ref().clone());
        }
    }
    let vertices = (0..8)
        .map(|i| Vertex {
            point: curves[i].control_points()[0],
        })
        .collect::<Vec<_>>();
    let mut edges = vec![];
    for (i, c) in curves.iter().enumerate() {
        let layer = i / 4;
        let q = i % 4;
        edges.push(Edge {
            vertices: [i, layer * 4 + (q + 1) % 4],
            curve: Curve::Nurbs(Box::new(c.clone())),
        });
    }
    for q in 0..4 {
        edges.push(Edge {
            vertices: [q, 4 + q],
            curve: Curve::Line {
                a: vertices[q].point,
                b: vertices[4 + q].point,
            },
        });
    }
    let cut = cap(source, section, policy)?;
    let mut faces = vec![];
    for layer in 0..2 {
        let surface = if (lower && layer == 0) || (!lower && layer == 1) {
            source.solid().shell.faces[layer].surface.clone()
        } else {
            cut.clone()
        };
        let Surface::Plane { origin, u, v } = surface else {
            return Err(unresolved());
        };
        let mut coedges = vec![];
        for q in 0..4 {
            if (lower && layer == 0) || (!lower && layer == 1) {
                coedges.push(source.solid().shell.faces[layer].wires[0].coedges[q].clone());
                continue;
            }
            let c = &curves[layer * 4 + q];
            let points = c
                .control_points()
                .iter()
                .map(|p| {
                    let d = *p - origin;
                    Point3::new(d.dot(u), d.dot(v), 0.)
                })
                .collect();
            let uv = NurbsCurve::new(2, c.knots().to_vec(), points, c.weights().to_vec())?;
            coedges.push(Coedge {
                edge: layer * 4 + q,
                forward: true,
                pcurve: PCurve::nurbs(uv)?,
            });
        }
        faces.push(Face {
            surface: Surface::Plane { origin, u, v },
            orientation: if layer == 0 { -1 } else { 1 },
            wires: vec![Wire { coedges }],
        });
    }
    for q in 0..4 {
        let a = &curves[q];
        let b = &curves[4 + q];
        let mut points = vec![];
        let mut weights = vec![];
        for i in 0..3 {
            points.extend([a.control_points()[i], b.control_points()[i]]);
            weights.extend([a.weights()[i], b.weights()[i]]);
        }
        let surface = NurbsSurface::new(
            [2, 1],
            [a.knots().to_vec(), vec![0., 0., 1., 1.]],
            [3, 2],
            points,
            weights,
        )?;
        let affine = |edge, forward, origin, direction| Coedge {
            edge,
            forward,
            pcurve: PCurve::Affine { origin, direction },
        };
        let generator = |edge, forward, u, w0, w1| -> Result<Coedge> {
            Ok(Coedge {
                edge,
                forward,
                pcurve: PCurve::nurbs(NurbsCurve::new(
                    1,
                    vec![0., 0., 1., 1.],
                    vec![Point3::new(u, 0., 0.), Point3::new(u, 1., 0.)],
                    vec![w1, w0],
                )?)?,
            })
        };
        faces.push(Face {
            surface: Surface::Nurbs(Box::new(surface)),
            orientation: 1,
            wires: vec![Wire {
                coedges: vec![
                    affine(q, true, [0., 0.], [1., 0.]),
                    generator(8 + (q + 1) % 4, true, 1., a.weights()[2], b.weights()[2])?,
                    affine(4 + q, false, [0., 1.], [1., 0.]),
                    generator(8 + q, false, 0., a.weights()[0], b.weights()[0])?,
                ],
            }],
        });
    }
    Ok(Solid {
        vertices,
        edges,
        shell: Shell { faces },
    })
}
fn lower_volume(
    source: &NurbsFrustumSolid,
    plane: &Surface,
    policy: GeometryTolerance,
) -> Result<f64> {
    let Surface::Plane { origin, u, v } = *plane else {
        return Err(unresolved());
    };
    let n = source.frame().local_vector(u.cross(v).normalized()?);
    let d = u
        .cross(v)
        .normalized()?
        .dot(origin - source.frame().origin());
    if n.z == 0. {
        return Err(unresolved());
    }
    let c = d / n.z;
    let a = n.x / n.z;
    let b = n.y / n.z;
    let [r0, r1] = source.radii();
    let height = source.height();
    let scale = r0.max(r1).max(height);
    let r = r0 / scale;
    let s = (r1 - r0) / height;
    let c = c / scale;
    let l2 = a * a + b * b;
    let beta = s * s * l2;
    let q = (1. - beta).sqrt();
    let rr = r + s * c;
    let first = c * (rr * rr + rr * r + r * r);
    let second = r * r * r * s * l2 * (1. + q + q * q) / (1. + q);
    let sum = first + second;
    if !q.is_normal()
        || !sum.is_normal()
        || sum <= 8192. * f64::EPSILON * (first.abs() + second.abs())
    {
        return Err(unresolved());
    }
    let fraction = (std::f64::consts::PI / 3.) * (sum / (q * q * q))
        / ((std::f64::consts::PI / 3.)
            * (height / scale)
            * (r * r + r * (r1 / scale) + (r1 / scale) * (r1 / scale)));
    let total = source.volume(policy)?;
    let lower = total * fraction;
    if !fraction.is_finite()
        || fraction <= 8192. * f64::EPSILON
        || 1. - fraction <= 8192. * f64::EPSILON
        || !lower.is_normal()
    {
        return Err(unresolved());
    }
    Ok(lower)
}
