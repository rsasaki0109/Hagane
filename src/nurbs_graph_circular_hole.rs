//! Canonical graph solid with one exact rational circular through bore.
use crate::nurbs_graph_step_import::{same_curve, same_surface};
use crate::*;
#[derive(Clone, Debug)]
pub struct NurbsGraphCircularHoledSolid {
    pub solid: Solid,
    source: NurbsGraphSolid,
    center: [f64; 2],
    radius: f64,
    tolerance: Tolerance,
}
fn regions(source: &NurbsGraphSolid, center: [f64; 2], radius: f64, tol: Tolerance) -> Result<()> {
    source.validate(tol)?;
    Tolerance::new(tol.linear)?;
    if center.iter().any(|x| !x.is_finite()) || !radius.is_finite() || radius <= 0. {
        return Err(Error::InvalidInput(
            "circle bore center and positive radius must be finite",
        ));
    }
    let arithmetic = source.arithmetic_budget()?;
    if arithmetic >= tol.linear / 4. {
        return Err(Error::Unsupported(
            "circle bore source arithmetic exceeds precision reserve",
        ));
    }
    let margin = 4. * tol.linear + arithmetic;
    let domain = source.source_domain();
    let dimensions = source.dimensions();
    if radius <= margin {
        return Err(Error::Unsupported("circle bore radius is unresolved"));
    }
    for a in 0..2 {
        let lo = dimensions[a] * domain[a][0];
        let hi = dimensions[a] * domain[a][1];
        let left = center[a] - radius;
        let right = center[a] + radius;
        if !left.is_finite()
            || !right.is_finite()
            || left >= center[a]
            || right <= center[a]
            || left - lo <= margin
            || hi - right <= margin
        {
            return Err(Error::InvalidInput(
                "circle bore must be resolved and strictly inside all source walls",
            ));
        }
    }
    Ok(())
}
fn uv_quarters(source: &NurbsGraphSolid, c: [f64; 2], r: f64) -> Result<Vec<NurbsCurve>> {
    let [l, w, _] = source.dimensions();
    let x = [(c[0] + r) / l, c[0] / l, (c[0] - r) / l, c[0] / l];
    let y = [c[1] / w, (c[1] + r) / w, c[1] / w, (c[1] - r) / w];
    let controls = [
        [[x[0], y[0]], [x[0], y[1]], [x[1], y[1]]],
        [[x[1], y[1]], [x[2], y[1]], [x[2], y[2]]],
        [[x[2], y[2]], [x[2], y[3]], [x[3], y[3]]],
        [[x[3], y[3]], [x[0], y[3]], [x[0], y[0]]],
    ];
    controls
        .into_iter()
        .map(|p| {
            NurbsCurve::new(
                2,
                vec![0., 0., 0., 1., 1., 1.],
                p.into_iter().map(|p| Point3::new(p[0], p[1], 0.)).collect(),
                vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
            )
        })
        .collect()
}
fn build(source: &NurbsGraphSolid, c: [f64; 2], r: f64, tol: Tolerance) -> Result<Solid> {
    regions(source, c, r, tol)?;
    let local = NurbsGraphSolid::new(source.dimensions(), source.bulge(), tol)?
        .trimmed_uv(source.source_domain(), tol)?;
    let mut solid = local.brep().clone();
    let Surface::Nurbs(roof) = &solid.shell.faces[1].surface else {
        unreachable!()
    };
    let uv = uv_quarters(source, c, r)?;
    let top = uv
        .iter()
        .map(|curve| roof.parameter_curve_nurbs(curve, Tolerance::new(0.75 * tol.linear)?))
        .collect::<Result<Vec<_>>>()?;
    let bottom = top
        .iter()
        .map(|curve| {
            NurbsCurve::new(
                curve.degree(),
                curve.knots().to_vec(),
                curve
                    .control_points()
                    .iter()
                    .map(|p| Point3::new(p.x, p.y, 0.))
                    .collect(),
                curve.weights().to_vec(),
            )
        })
        .collect::<Result<Vec<_>>>()?;
    for curve in &bottom {
        solid.vertices.push(Vertex {
            point: curve.evaluate(0.)?,
        });
    }
    for curve in &top {
        solid.vertices.push(Vertex {
            point: curve.evaluate(0.)?,
        });
    }
    for (i, curve) in bottom.iter().enumerate() {
        solid.edges.push(Edge {
            vertices: [8 + i, 8 + (i + 1) % 4],
            curve: Curve::Nurbs(Box::new(curve.clone())),
        });
    }
    for (i, curve) in top.iter().enumerate() {
        solid.edges.push(Edge {
            vertices: [12 + i, 12 + (i + 1) % 4],
            curve: Curve::Nurbs(Box::new(curve.clone())),
        });
    }
    for i in 0..4 {
        solid.edges.push(Edge {
            vertices: [8 + i, 12 + i],
            curve: Curve::Nurbs(Box::new(NurbsCurve::new(
                1,
                vec![0., 0., 1., 1.],
                vec![solid.vertices[8 + i].point, solid.vertices[12 + i].point],
                vec![1.; 2],
            )?)),
        });
    }
    for (fi, offset) in [(0, 12), (1, 16)] {
        solid.shell.faces[fi].wires.push(Wire {
            coedges: (0..4)
                .rev()
                .map(|i| {
                    Ok(Coedge {
                        edge: offset + i,
                        forward: false,
                        pcurve: PCurve::nurbs(uv[i].clone())?,
                    })
                })
                .collect::<Result<Vec<_>>>()?,
        });
    }
    for (i, curve) in top.iter().enumerate() {
        let mut points = Vec::new();
        let mut weights = Vec::new();
        for (p, &weight) in curve.control_points().iter().zip(curve.weights()) {
            points.extend([Point3::new(p.x, p.y, 0.), *p]);
            weights.extend([weight, weight]);
        }
        let surface = NurbsSurface::new(
            [8, 1],
            [curve.knots().to_vec(), vec![0., 0., 1., 1.]],
            [9, 2],
            points,
            weights,
        )?;
        let edges = [12 + i, 20 + (i + 1) % 4, 16 + i, 20 + i];
        let affine = [
            ([0., 0.], [1., 0.]),
            ([1., 0.], [0., 1.]),
            ([0., 1.], [1., 0.]),
            ([0., 0.], [0., 1.]),
        ];
        solid.shell.faces.push(Face {
            surface: Surface::Nurbs(Box::new(surface)),
            orientation: -1,
            wires: vec![Wire {
                coedges: (0..4)
                    .map(|j| Coedge {
                        edge: edges[j],
                        forward: j < 2,
                        pcurve: PCurve::Affine {
                            origin: affine[j].0,
                            direction: affine[j].1,
                        },
                    })
                    .collect(),
            }],
        });
    }
    crate::nurbs_graph_solid::placed(solid, source.placement())
}
fn same_pcurve(a: &PCurve, b: &PCurve) -> bool {
    match (a, b) {
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
        (PCurve::Nurbs(a), PCurve::Nurbs(b)) => {
            a.degree() == b.degree()
                && a.knots() == b.knots()
                && a.control_points() == b.control_points()
                && a.weights() == b.weights()
        }
        _ => false,
    }
}
impl NurbsGraphSolid {
    pub fn through_xy_circle(
        &self,
        center: [f64; 2],
        radius: f64,
        tol: Tolerance,
    ) -> Result<NurbsGraphCircularHoledSolid> {
        NurbsGraphCircularHoledSolid::new(self, center, radius, tol)
    }
}
impl NurbsGraphCircularHoledSolid {
    pub fn new(
        source: &NurbsGraphSolid,
        center: [f64; 2],
        radius: f64,
        tol: Tolerance,
    ) -> Result<Self> {
        let result = Self {
            solid: build(source, center, radius, tol)?,
            source: source.clone(),
            center,
            radius,
            tolerance: tol,
        };
        result.validate(tol)?;
        Ok(result)
    }
    pub fn source(&self) -> &NurbsGraphSolid {
        &self.source
    }
    pub fn center(&self) -> [f64; 2] {
        self.center
    }
    pub fn radius(&self) -> f64 {
        self.radius
    }
    pub fn brep(&self) -> &Solid {
        &self.solid
    }
    pub(crate) fn construction_tolerance(&self) -> Tolerance {
        self.tolerance
    }
    pub fn bounds(&self) -> Result<Bounds> {
        self.validate(self.tolerance)?;
        self.source.bounds()
    }
    pub fn validate(&self, tol: Tolerance) -> Result<()> {
        let expected = build(&self.source, self.center, self.radius, tol)?;
        let bad =
            || Error::InvalidTopology("circle bore differs from its canonical rational B-rep");
        let actual = &self.solid;
        if actual.vertices.len() != 16 || actual.edges.len() != 24 || actual.shell.faces.len() != 10
        {
            return Err(bad());
        }
        for (a, b) in actual.vertices.iter().zip(&expected.vertices) {
            if a.point != b.point {
                return Err(bad());
            }
        }
        for (a, b) in actual.edges.iter().zip(&expected.edges) {
            if a.vertices != b.vertices || !same_curve(&a.curve, &b.curve) {
                return Err(bad());
            }
        }
        let mut uses = vec![[0usize; 2]; 24];
        for (a, b) in actual.shell.faces.iter().zip(&expected.shell.faces) {
            if a.orientation != b.orientation
                || !same_surface(&a.surface, &b.surface)?
                || a.wires.len() != b.wires.len()
            {
                return Err(bad());
            }
            for (aw, bw) in a.wires.iter().zip(&b.wires) {
                if aw.coedges.len() != bw.coedges.len() {
                    return Err(bad());
                }
                for (c, d) in aw.coedges.iter().zip(&bw.coedges) {
                    if c.edge != d.edge
                        || c.forward != d.forward
                        || !same_pcurve(&c.pcurve, &d.pcurve)
                    {
                        return Err(bad());
                    }
                    uses[c.edge][usize::from(c.forward == (a.orientation == 1))] += 1;
                    let Curve::Nurbs(curve) = &actual.edges[c.edge].curve else {
                        return Err(bad());
                    };
                    let domain = curve.domain();
                    for k in 0..=8 {
                        let t = domain[0] + (domain[1] - domain[0]) * k as f64 / 8.;
                        let uv = c.pcurve.try_evaluate(t)?;
                        let p = a.surface.try_evaluate(uv[0], uv[1])?;
                        let q = curve.evaluate(t)?;
                        let delta = p - q;
                        if !delta.finite()
                            || delta.x.hypot(delta.y).hypot(delta.z) > tol.linear / 4.
                        {
                            return Err(Error::Unsupported(
                                "circle bore edge/surface identity exceeds precision budget",
                            ));
                        }
                    }
                }
            }
        }
        if uses.iter().any(|u| *u != [1, 1]) {
            return Err(bad());
        }
        for edge in &actual.edges {
            let Curve::Nurbs(curve) = &edge.curve else {
                return Err(bad());
            };
            for (id, t) in edge.vertices.into_iter().zip(curve.domain()) {
                let delta = actual.vertices[id].point - curve.evaluate(t)?;
                if !delta.finite() || delta.x.hypot(delta.y).hypot(delta.z) > tol.linear / 4. {
                    return Err(Error::Unsupported(
                        "circle bore endpoint identity exceeds precision budget",
                    ));
                }
            }
        }
        Ok(())
    }
}
