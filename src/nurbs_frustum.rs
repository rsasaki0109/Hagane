//! Checked closed rational ruled frusta, retaining their exact spline boundaries.
use crate::*;

#[derive(Clone, Debug)]
pub struct NurbsFrustumSolid {
    solid: Solid,
    frame: Frame3,
    radii: [f64; 2],
    height: f64,
}
const DIRECTIONS: [[f64; 2]; 4] = [[1., 0.], [0., 1.], [-1., 0.], [0., -1.]];
fn knots() -> Vec<f64> {
    vec![0., 0., 0., 1., 1., 1.]
}
fn same_curve(a: &NurbsCurve, b: &NurbsCurve) -> bool {
    a.degree() == b.degree()
        && a.knots() == b.knots()
        && a.weights() == b.weights()
        && a.control_points() == b.control_points()
}
fn same_pcurve(a: &PCurve, b: &PCurve) -> bool {
    match (a, b) {
        (PCurve::Nurbs(a), PCurve::Nurbs(b)) => same_curve(a, b),
        (
            PCurve::Affine {
                origin: a,
                direction: b,
            },
            PCurve::Affine {
                origin: c,
                direction: d,
            },
        ) => a == c && b == d,
        _ => false,
    }
}
fn same_surface(a: &Surface, b: &Surface) -> bool {
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
        ) => a == d && b == e && c == f,
        (Surface::Nurbs(a), Surface::Nurbs(b)) => {
            a.degrees() == b.degrees()
                && a.control_counts() == b.control_counts()
                && a.control_points() == b.control_points()
                && a.weights() == b.weights()
                && [0, 1]
                    .into_iter()
                    .all(|i| a.knots(i).ok() == b.knots(i).ok())
        }
        _ => false,
    }
}
fn same_body(a: &Solid, b: &Solid) -> bool {
    a.vertices.len() == b.vertices.len()
        && a.edges.len() == b.edges.len()
        && a.shell.faces.len() == b.shell.faces.len()
        && a.vertices
            .iter()
            .zip(&b.vertices)
            .all(|(a, b)| a.point == b.point)
        && a.edges.iter().zip(&b.edges).all(|(a, b)| {
            a.vertices == b.vertices
                && match (&a.curve, &b.curve) {
                    (Curve::Nurbs(a), Curve::Nurbs(b)) => same_curve(a, b),
                    (Curve::Line { a, b }, Curve::Line { a: c, b: d }) => a == c && b == d,
                    _ => false,
                }
        })
        && a.shell.faces.iter().zip(&b.shell.faces).all(|(a, b)| {
            a.orientation == b.orientation
                && same_surface(&a.surface, &b.surface)
                && a.wires.len() == b.wires.len()
                && a.wires.iter().zip(&b.wires).all(|(a, b)| {
                    a.coedges.len() == b.coedges.len()
                        && a.coedges.iter().zip(&b.coedges).all(|(a, b)| {
                            a.edge == b.edge
                                && a.forward == b.forward
                                && same_pcurve(&a.pcurve, &b.pcurve)
                        })
                })
        })
}
fn parameters(
    frame: Frame3,
    radii: [f64; 2],
    height: f64,
    policy: GeometryTolerance,
) -> Result<f64> {
    let checked = Frame3::new_with_tolerance(frame.origin(), frame.axes(), policy)?;
    if radii
        .into_iter()
        .chain([height])
        .any(|x| !x.is_finite() || !x.is_normal() || x <= 0.)
    {
        return Err(Error::InvalidInput(
            "frustum needs positive finite resolved radii and height; apex is unsupported",
        ));
    }
    let scale = (2. * radii[0].max(radii[1])).hypot(height);
    let world = frame
        .origin()
        .x
        .abs()
        .max(frame.origin().y.abs())
        .max(frame.origin().z.abs())
        + 2. * radii[0].max(radii[1])
        + height;
    let arithmetic = 4096. * f64::EPSILON * world * 8.;
    let drift = frame
        .axes()
        .into_iter()
        .zip(checked.axes())
        .map(|(a, b)| (a - b).norm())
        .fold(0., f64::max)
        * scale;
    if !scale.is_finite()
        || !arithmetic.is_finite()
        || !drift.is_finite()
        || arithmetic + drift >= policy.linear() / 8.
    {
        return Err(Error::Unsupported(
            "frustum frame or world-coordinate precision is unresolved",
        ));
    }
    let band = policy.length_at_scale(scale)?;
    if radii
        .into_iter()
        .chain([height])
        .any(|x| x <= 10. * band + arithmetic)
    {
        return Err(Error::InvalidInput(
            "frustum radii and height must exceed ten geometric tolerances",
        ));
    }
    Ok(arithmetic + drift)
}
fn build(frame: Frame3, radii: [f64; 2], height: f64) -> Result<Solid> {
    let mut solid = Solid {
        vertices: Vec::new(),
        edges: Vec::new(),
        shell: Shell { faces: Vec::new() },
    };
    let weights = vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.];
    let mut uv = [Vec::new(), Vec::new()];
    for layer in 0..2 {
        let z = if layer == 0 { 0. } else { height };
        for d in DIRECTIONS {
            solid.vertices.push(Vertex {
                point: frame.point(Point3::new(radii[layer] * d[0], radii[layer] * d[1], z)),
            });
        }
        for q in 0..4 {
            let a = DIRECTIONS[q];
            let b = DIRECTIONS[(q + 1) % 4];
            let local = [a, [a[0] + b[0], a[1] + b[1]], b]
                .map(|d| Point3::new(radii[layer] * d[0], radii[layer] * d[1], 0.));
            uv[layer].push(NurbsCurve::new(
                2,
                knots(),
                local.to_vec(),
                weights.clone(),
            )?);
            let points = local.map(|p| frame.point(Point3::new(p.x, p.y, z)));
            solid.edges.push(Edge {
                vertices: [4 * layer + q, 4 * layer + (q + 1) % 4],
                curve: Curve::Nurbs(Box::new(NurbsCurve::new(
                    2,
                    knots(),
                    points.to_vec(),
                    weights.clone(),
                )?)),
            });
        }
    }
    for q in 0..4 {
        solid.edges.push(Edge {
            vertices: [q, 4 + q],
            curve: Curve::Line {
                a: solid.vertices[q].point,
                b: solid.vertices[4 + q].point,
            },
        });
    }
    for (layer, curves) in uv.into_iter().enumerate() {
        solid.shell.faces.push(Face {
            surface: Surface::Plane {
                origin: frame.point(Point3::new(0., 0., if layer == 0 { 0. } else { height })),
                u: frame.axes()[0],
                v: frame.axes()[1],
            },
            orientation: if layer == 0 { -1 } else { 1 },
            wires: vec![Wire {
                coedges: curves
                    .into_iter()
                    .enumerate()
                    .map(|(q, c)| Coedge {
                        edge: layer * 4 + q,
                        forward: true,
                        pcurve: PCurve::Nurbs(Box::new(c)),
                    })
                    .collect(),
            }],
        });
    }
    for q in 0..4 {
        let bottom = match &solid.edges[q].curve {
            Curve::Nurbs(c) => c,
            _ => unreachable!(),
        };
        let top = match &solid.edges[4 + q].curve {
            Curve::Nurbs(c) => c,
            _ => unreachable!(),
        };
        let mut points = Vec::new();
        let mut w = Vec::new();
        for (i, &weight) in weights.iter().enumerate() {
            points.extend([bottom.control_points()[i], top.control_points()[i]]);
            w.extend([weight, weight]);
        }
        let surface =
            NurbsSurface::new([2, 1], [knots(), vec![0., 0., 1., 1.]], [3, 2], points, w)?;
        let c = |edge, forward, origin, direction| Coedge {
            edge,
            forward,
            pcurve: PCurve::Affine { origin, direction },
        };
        solid.shell.faces.push(Face {
            surface: Surface::Nurbs(Box::new(surface)),
            orientation: 1,
            wires: vec![Wire {
                coedges: vec![
                    c(q, true, [0., 0.], [1., 0.]),
                    c(8 + (q + 1) % 4, true, [1., 0.], [0., 1.]),
                    c(4 + q, false, [0., 1.], [1., 0.]),
                    c(8 + q, false, [0., 0.], [0., 1.]),
                ],
            }],
        });
    }
    Ok(solid)
}
impl NurbsFrustumSolid {
    pub fn new(
        frame: Frame3,
        radii: [f64; 2],
        height: f64,
        policy: GeometryTolerance,
    ) -> Result<Self> {
        parameters(frame, radii, height, policy)?;
        Self::from_brep(build(frame, radii, height)?, frame, radii, height, policy)
    }
    /// Accept only the complete canonical actual geometry and shared topology.
    /// This does not fit, repair, normalize weights or replace the supplied body.
    pub fn from_brep(
        solid: Solid,
        frame: Frame3,
        radii: [f64; 2],
        height: f64,
        policy: GeometryTolerance,
    ) -> Result<Self> {
        let result = Self {
            solid,
            frame,
            radii,
            height,
        };
        result.validate(policy)?;
        Ok(result)
    }
    pub fn solid(&self) -> &Solid {
        &self.solid
    }
    pub fn frame(&self) -> Frame3 {
        self.frame
    }
    pub fn radii(&self) -> [f64; 2] {
        self.radii
    }
    pub fn height(&self) -> f64 {
        self.height
    }
    pub fn validate(&self, policy: GeometryTolerance) -> Result<()> {
        let arithmetic = parameters(self.frame, self.radii, self.height, policy)?;
        let expected = build(self.frame, self.radii, self.height)?;
        if !same_body(&self.solid, &expected) {
            return Err(Error::InvalidTopology(
                "frustum differs from its complete canonical rational B-rep certificate",
            ));
        }
        let mut uses = [(0usize, 0i32); 12];
        for f in &self.solid.shell.faces {
            for c in &f.wires[0].coedges {
                uses[c.edge].0 += 1;
                uses[c.edge].1 += i32::from(f.orientation) * if c.forward { 1 } else { -1 };
                for t in [0., 0.25, 0.5, 0.75, 1.] {
                    let uv = c.pcurve.try_evaluate(t)?;
                    let p = f.surface.try_evaluate(uv[0], uv[1])?;
                    let q = self.solid.edges[c.edge].curve.try_evaluate(t)?;
                    if !(p - q).finite() || (p - q).norm() + arithmetic >= policy.linear() / 4. {
                        return Err(Error::Unsupported(
                            "frustum same-parameter world precision is unresolved",
                        ));
                    }
                }
            }
        }
        if uses.iter().any(|&(n, s)| n != 2 || s != 0) {
            return Err(Error::InvalidTopology(
                "frustum boundary is not closed and oriented",
            ));
        }
        Ok(())
    }
    /// Exact AP214 model export; generic STEP import remains outside this scope.
    pub fn export_step_mm(&self, policy: GeometryTolerance) -> Result<String> {
        self.validate(policy)?;
        crate::nurbs_graph_step::write(&self.solid, policy.absolute())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_boundary_certificate_and_private_step_dispatch() {
        let policy = GeometryTolerance::default();
        let part = NurbsFrustumSolid::new(Frame3::IDENTITY, [2., 1.], 3., policy).unwrap();
        assert_eq!(
            (
                part.solid().vertices.len(),
                part.solid().edges.len(),
                part.solid().shell.faces.len()
            ),
            (8, 12, 6)
        );
        let step = part.export_step_mm(policy).unwrap();
        assert!(step.contains("RATIONAL_B_SPLINE_SURFACE"));
        assert!(step.contains("RATIONAL_B_SPLINE_CURVE"));
        assert_eq!(step.matches("=PLANE(").count(), 2);
        assert!(part.solid().validate(policy.absolute()).is_err());
        let mut damaged = part.solid().clone();
        damaged.vertices[0].point.x += 1e-12;
        assert!(NurbsFrustumSolid::from_brep(
            damaged,
            part.frame(),
            part.radii(),
            part.height(),
            policy
        )
        .is_err());
        let mut damaged = part.solid().clone();
        damaged.shell.faces[2].wires[0].coedges[1].forward = false;
        assert!(NurbsFrustumSolid::from_brep(
            damaged,
            part.frame(),
            part.radii(),
            part.height(),
            policy
        )
        .is_err());
    }
    #[test]
    fn unresolved_apex_far_world_and_conditioning_are_explicit_errors() {
        let policy = GeometryTolerance::default();
        for (r, h) in [
            ([0., 1.], 3.),
            ([1., f64::NAN], 3.),
            ([1., 2.], 0.),
            ([1e-12, 2.], 3.),
        ] {
            assert!(NurbsFrustumSolid::new(Frame3::IDENTITY, r, h, policy).is_err());
        }
        let frame = Frame3::translation(Point3::new(1e12, 0., 0.)).unwrap();
        assert!(NurbsFrustumSolid::new(frame, [2., 1.], 3., policy).is_err());
    }
}
