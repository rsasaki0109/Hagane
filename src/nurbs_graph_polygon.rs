//! Canonical closed graph bodies restricted by a strictly convex UV polygon.
use crate::*;
#[derive(Clone, Debug)]
pub struct NurbsGraphPolygonSolid {
    pub solid: Solid,
    source: NurbsGraphSolid,
    polygon: Vec<[f64; 2]>,
    construction_tolerance: Tolerance,
}
fn build(source: &NurbsGraphSolid, polygon: &[[f64; 2]], tol: Tolerance) -> Result<Solid> {
    source.validate(tol)?;
    if !(3..=16).contains(&polygon.len()) {
        return Err(Error::InvalidInput(
            "graph polygon requires 3 through 16 corners",
        ));
    }
    let [l, w, _] = source.dimensions();
    let margin = 4. * tol.linear + source.arithmetic_budget()?;
    for i in 0..polygon.len() {
        let a = polygon[i];
        let b = polygon[(i + 1) % polygon.len()];
        let delta = [l * (b[0] - a[0]), w * (b[1] - a[1])];
        let length = delta[0].hypot(delta[1]);
        if !length.is_finite() || length <= margin {
            return Err(Error::Unsupported(
                "graph polygon physical edges are unresolved",
            ));
        }
        for (j, p) in polygon.iter().enumerate() {
            if j != i && j != (i + 1) % polygon.len() {
                let x = l * (p[0] - a[0]);
                let y = w * (p[1] - a[1]);
                let distance = (x * (delta[1] / length) - y * (delta[0] / length)).abs();
                if !distance.is_finite() || distance <= margin {
                    return Err(Error::Unsupported(
                        "graph polygon physical corner clearance is unresolved",
                    ));
                }
            }
        }
    }
    let local = NurbsGraphSolid::new(source.dimensions(), source.bulge(), tol)?
        .trimmed_uv(source.source_domain(), tol)?;
    let mut caps = Vec::new();
    for (i, orientation) in [-1, 1].into_iter().enumerate() {
        let Surface::Nurbs(surface) = &local.brep().shell.faces[i].surface else {
            return Err(Error::InvalidTopology("graph cap requires NURBS"));
        };
        caps.push(NurbsPolygonFace::new(
            (**surface).clone(),
            polygon.to_vec(),
            orientation,
            tol,
        )?);
    }
    let n = polygon.len();
    let mut vertices = caps[0].boundary.vertices.clone();
    vertices.extend(caps[1].boundary.vertices.clone());
    let mut edges = caps[0].boundary.edges.clone();
    for e in &caps[1].boundary.edges {
        let mut e = e.clone();
        e.vertices = e.vertices.map(|i| n + i);
        edges.push(e);
    }
    for i in 0..n {
        let a = vertices[i].point;
        let b = vertices[n + i].point;
        let curve = NurbsCurve::new(
            2,
            vec![0., 0., 0., 1., 1., 1.],
            vec![a, a + (b - a) * 0.5, b],
            vec![1.; 3],
        )?;
        edges.push(Edge {
            vertices: [i, n + i],
            curve: Curve::Nurbs(Box::new(curve)),
        });
    }
    let mut faces = Vec::new();
    for (i, cap) in caps.into_iter().enumerate() {
        let mut face = cap.face;
        for c in &mut face.wires[0].coedges {
            c.edge += i * n;
        }
        faces.push(face);
    }
    for i in 0..n {
        let Curve::Nurbs(curve) = &edges[n + i].curve else {
            return Err(Error::InvalidTopology(
                "graph polygon roof boundary requires NURBS",
            ));
        };
        let mut controls = Vec::new();
        for p in curve.control_points() {
            for j in 0..3 {
                controls.push(Point3::new(p.x, p.y, p.z * (j as f64 / 2.)));
            }
        }
        let surface = NurbsSurface::new(
            [curve.degree(), 2],
            [curve.knots().to_vec(), vec![0., 0., 0., 1., 1., 1.]],
            [curve.control_points().len(), 3],
            controls,
            curve
                .weights()
                .iter()
                .flat_map(|weight| [*weight; 3])
                .collect(),
        )?;
        let coedge = |edge, forward, origin, direction| Coedge {
            edge,
            forward,
            pcurve: PCurve::Affine { origin, direction },
        };
        faces.push(Face {
            surface: Surface::Nurbs(Box::new(surface)),
            orientation: 1,
            wires: vec![Wire {
                coedges: vec![
                    coedge(i, true, [0., 0.], [1., 0.]),
                    coedge(2 * n + (i + 1) % n, true, [1., 0.], [0., 1.]),
                    coedge(n + i, false, [0., 1.], [1., 0.]),
                    coedge(2 * n + i, false, [0., 0.], [0., 1.]),
                ],
            }],
        });
    }
    crate::nurbs_graph_solid::placed(
        Solid {
            vertices,
            edges,
            shell: Shell { faces },
        },
        source.placement(),
    )
}
fn same_curve(a: &Curve, b: &Curve) -> bool {
    match (a, b) {
        (Curve::Nurbs(a), Curve::Nurbs(b)) => {
            a.degree() == b.degree()
                && a.knots() == b.knots()
                && a.control_points() == b.control_points()
                && a.weights() == b.weights()
        }
        _ => false,
    }
}
fn same_surface(a: &Surface, b: &Surface) -> Result<bool> {
    match (a, b) {
        (Surface::Nurbs(a), Surface::Nurbs(b)) => Ok(a.degrees() == b.degrees()
            && a.control_points() == b.control_points()
            && a.weights() == b.weights()
            && a.knots(0)? == b.knots(0)?
            && a.knots(1)? == b.knots(1)?),
        _ => Ok(false),
    }
}
impl NurbsGraphPolygonSolid {
    pub fn new(source: &NurbsGraphSolid, polygon: Vec<[f64; 2]>, tol: Tolerance) -> Result<Self> {
        let solid = build(source, &polygon, tol)?;
        let result = Self {
            solid,
            source: source.clone(),
            polygon,
            construction_tolerance: tol,
        };
        result.validate(tol)?;
        Ok(result)
    }
    pub(crate) fn construction_tolerance(&self) -> Tolerance {
        self.construction_tolerance
    }
    pub fn source(&self) -> &NurbsGraphSolid {
        &self.source
    }
    pub fn polygon(&self) -> &[[f64; 2]] {
        &self.polygon
    }
    pub fn brep(&self) -> &Solid {
        &self.solid
    }
    /// Conservative source enclosure; validates the actual retained body first.
    pub fn bounds(&self) -> Result<Bounds> {
        self.validate(self.construction_tolerance)?;
        self.source.bounds()
    }
    pub fn validate(&self, tol: Tolerance) -> Result<()> {
        Tolerance::new(tol.linear)?;
        let expected = build(&self.source, &self.polygon, tol)?;
        let n = self.polygon.len();
        let bad =
            || Error::InvalidTopology("graph polygon differs from its canonical closed B-rep");
        if self.solid.vertices.len() != 2 * n
            || self.solid.edges.len() != 3 * n
            || self.solid.shell.faces.len() != n + 2
        {
            return Err(bad());
        }
        for (a, b) in self.solid.vertices.iter().zip(&expected.vertices) {
            if a.point != b.point {
                return Err(bad());
            }
        }
        for (a, b) in self.solid.edges.iter().zip(&expected.edges) {
            if a.vertices != b.vertices || !same_curve(&a.curve, &b.curve) {
                return Err(bad());
            }
        }
        let mut incidence = vec![[0i32; 2]; 3 * n];
        for (a, b) in self.solid.shell.faces.iter().zip(&expected.shell.faces) {
            if a.orientation != b.orientation
                || a.wires.len() != 1
                || a.wires[0].coedges.len() != b.wires[0].coedges.len()
                || !same_surface(&a.surface, &b.surface)?
            {
                return Err(bad());
            }
            let Surface::Nurbs(surface) = &a.surface else {
                return Err(bad());
            };
            for (c, d) in a.wires[0].coedges.iter().zip(&b.wires[0].coedges) {
                let same = matches!((&c.pcurve,&d.pcurve),(PCurve::Affine{origin:a,direction:da},PCurve::Affine{origin:b,direction:db}) if a==b&&da==db);
                if c.edge != d.edge || c.forward != d.forward || !same {
                    return Err(bad());
                }
                incidence[c.edge][0] += 1;
                incidence[c.edge][1] += a.orientation as i32 * if c.forward { 1 } else { -1 };
                let edge = &self.solid.edges[c.edge];
                let range = edge.curve.range();
                for k in 0..=8 {
                    let t = range[0] + (range[1] - range[0]) * k as f64 / 8.;
                    let point = edge.curve.try_evaluate(t)?;
                    let uv = c.pcurve.evaluate(t);
                    let gap = (point - surface.evaluate(uv[0], uv[1])?).norm();
                    if !gap.is_finite() || gap > tol.linear / 4. {
                        return Err(bad());
                    }
                }
                for (k, t) in range.into_iter().enumerate() {
                    let gap = (edge.curve.try_evaluate(t)?
                        - self.solid.vertices[edge.vertices[k]].point)
                        .norm();
                    if !gap.is_finite() || gap > tol.linear / 4. {
                        return Err(bad());
                    }
                }
            }
        }
        if incidence.iter().any(|v| *v != [2, 0]) || 2 * n + n + 2 != 3 * n + 2 {
            return Err(bad());
        }
        Ok(())
    }
}
