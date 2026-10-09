//! One exact convex polygon through opening in a canonical graph stock.
use crate::*;
#[derive(Clone, Debug)]
pub struct NurbsGraphPolygonHoledSolid {
    pub solid: Solid,
    outer: NurbsGraphPolygonSolid,
    opening: Vec<[f64; 2]>,
    construction_tolerance: Tolerance,
}
fn build(outer: &NurbsGraphPolygonSolid, opening: &[[f64; 2]], tol: Tolerance) -> Result<Solid> {
    outer.validate(tol)?;
    let source = outer.source();
    let inner = NurbsGraphPolygonSolid::new(source, opening.to_vec(), tol)?;
    let [l, w, _] = source.dimensions();
    let margin = 4. * tol.linear + source.arithmetic_budget()?;
    for i in 0..outer.polygon().len() {
        let a = outer.polygon()[i];
        let b = outer.polygon()[(i + 1) % outer.polygon().len()];
        let dx = l * (b[0] - a[0]);
        let dy = w * (b[1] - a[1]);
        let length = dx.hypot(dy);
        for p in opening {
            let distance =
                (dx / length) * (w * (p[1] - a[1])) - (dy / length) * (l * (p[0] - a[0]));
            if !distance.is_finite() || distance <= margin {
                return Err(Error::Unsupported(
                    "polygon opening must be strictly inside resolved outer walls",
                ));
            }
        }
    }
    crate::nurbs_graph_polygon_material::material_triangles(
        outer.polygon(),
        opening,
        [l, w],
        margin,
    )?;
    let no = outer.polygon().len();
    let ni = opening.len();
    let n = no + ni;
    let remap_vertex = |i: usize, count: usize, offset: usize| {
        if i < count {
            offset + i
        } else {
            n + offset + i - count
        }
    };
    let remap_edge = |i: usize, count: usize, offset: usize| i / count * n + offset + i % count;
    let mut solid = Solid {
        vertices: Vec::new(),
        edges: vec![],
        shell: Shell { faces: vec![] },
    };
    for top in 0..2 {
        solid
            .vertices
            .extend_from_slice(&outer.brep().vertices[top * no..(top + 1) * no]);
        solid
            .vertices
            .extend_from_slice(&inner.brep().vertices[top * ni..(top + 1) * ni]);
    }
    for category in 0..3 {
        for (body, count, offset) in [(outer, no, 0), (&inner, ni, no)] {
            for e in &body.brep().edges[category * count..(category + 1) * count] {
                let mut e = e.clone();
                e.vertices = e.vertices.map(|i| remap_vertex(i, count, offset));
                solid.edges.push(e);
            }
        }
    }
    for cap in 0..2 {
        let mut face = outer.brep().shell.faces[cap].clone();
        for c in &mut face.wires[0].coedges {
            c.edge = remap_edge(c.edge, no, 0);
        }
        let mut hole = inner.brep().shell.faces[cap].wires[0].clone();
        hole.coedges.reverse();
        for c in &mut hole.coedges {
            c.edge = remap_edge(c.edge, ni, no);
            c.forward = !c.forward;
        }
        face.wires.push(hole);
        solid.shell.faces.push(face);
    }
    for (body, count, offset, sign) in [(outer, no, 0, 1), (&inner, ni, no, -1)] {
        for original in &body.brep().shell.faces[2..] {
            let mut face = original.clone();
            face.orientation *= sign;
            for c in &mut face.wires[0].coedges {
                c.edge = remap_edge(c.edge, count, offset);
            }
            solid.shell.faces.push(face);
        }
    }
    Ok(solid)
}
impl NurbsGraphPolygonHoledSolid {
    pub fn new(
        outer: &NurbsGraphPolygonSolid,
        opening: Vec<[f64; 2]>,
        tol: Tolerance,
    ) -> Result<Self> {
        let solid = build(outer, &opening, tol)?;
        let body = Self {
            solid,
            outer: outer.clone(),
            opening,
            construction_tolerance: tol,
        };
        body.validate(tol)?;
        Ok(body)
    }
    pub fn source(&self) -> &NurbsGraphSolid {
        self.outer.source()
    }
    pub fn outer_polygon(&self) -> &[[f64; 2]] {
        self.outer.polygon()
    }
    pub fn opening(&self) -> &[[f64; 2]] {
        &self.opening
    }
    pub fn polygon(&self) -> &[[f64; 2]] {
        self.outer_polygon()
    }
    pub fn hole(&self) -> &[[f64; 2]] {
        self.opening()
    }
    pub fn brep(&self) -> &Solid {
        &self.solid
    }
    pub(crate) fn construction_tolerance(&self) -> Tolerance {
        self.construction_tolerance
    }
    pub(crate) fn material_triangles(&self) -> Result<Vec<[usize; 3]>> {
        Ok(crate::nurbs_graph_polygon_material::material_triangles(
            self.outer_polygon(),
            self.opening(),
            [self.source().dimensions()[0], self.source().dimensions()[1]],
            4. * self.construction_tolerance.linear + self.source().arithmetic_budget()?,
        )?
        .triangles)
    }
    pub fn bounds(&self) -> Result<Bounds> {
        self.validate(self.construction_tolerance)?;
        self.source().bounds()
    }
    pub fn validate(&self, tol: Tolerance) -> Result<()> {
        let expected = build(&self.outer, &self.opening, tol)?;
        let n = self.outer_polygon().len() + self.opening.len();
        let bad =
            || Error::InvalidTopology("polygon opening differs from canonical genus-one B-rep");
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
            let (Curve::Nurbs(ca), Curve::Nurbs(cb)) = (&a.curve, &b.curve) else {
                return Err(bad());
            };
            if a.vertices != b.vertices
                || ca.degree() != cb.degree()
                || ca.knots() != cb.knots()
                || ca.weights() != cb.weights()
                || ca.control_points() != cb.control_points()
            {
                return Err(bad());
            }
        }
        let mut incidence = vec![[0i32; 2]; 3 * n];
        for (a, b) in self.solid.shell.faces.iter().zip(&expected.shell.faces) {
            let (Surface::Nurbs(sa), Surface::Nurbs(sb)) = (&a.surface, &b.surface) else {
                return Err(bad());
            };
            if a.orientation != b.orientation
                || a.wires.len() != b.wires.len()
                || sa.degrees() != sb.degrees()
                || sa.control_counts() != sb.control_counts()
                || sa.control_points() != sb.control_points()
                || sa.weights() != sb.weights()
                || sa.knots(0)? != sb.knots(0)?
                || sa.knots(1)? != sb.knots(1)?
            {
                return Err(bad());
            }
            for (wa, wb) in a.wires.iter().zip(&b.wires) {
                if wa.coedges.len() != wb.coedges.len() {
                    return Err(bad());
                }
                for (c, d) in wa.coedges.iter().zip(&wb.coedges) {
                    let same = matches!((&c.pcurve,&d.pcurve),(PCurve::Affine{origin:a,direction:da},PCurve::Affine{origin:b,direction:db})if a==b&&da==db);
                    if c.edge != d.edge || c.forward != d.forward || !same {
                        return Err(bad());
                    }
                    incidence[c.edge][0] += 1;
                    incidence[c.edge][1] += a.orientation as i32 * if c.forward { 1 } else { -1 };
                    let edge = &self.solid.edges[c.edge];
                    for k in 0..=8 {
                        let t = k as f64 / 8.;
                        let uv = c.pcurve.evaluate(t);
                        let gap = (edge.curve.try_evaluate(t)? - sa.evaluate(uv[0], uv[1])?).norm();
                        if !gap.is_finite() || gap > tol.linear / 4. {
                            return Err(bad());
                        }
                    }
                    for (k, t) in edge.curve.range().into_iter().enumerate() {
                        let gap = (edge.curve.try_evaluate(t)?
                            - self.solid.vertices[edge.vertices[k]].point)
                            .norm();
                        if !gap.is_finite() || gap > tol.linear / 4. {
                            return Err(bad());
                        }
                    }
                }
            }
        }
        if incidence.iter().any(|v| *v != [2, 0])
            || self.solid.vertices.len() + self.solid.shell.faces.len() - 2
                != self.solid.edges.len()
        {
            return Err(bad());
        }
        Ok(())
    }
}
impl NurbsGraphPolygonSolid {
    pub fn through_uv_polygon(
        &self,
        opening: Vec<[f64; 2]>,
        tol: Tolerance,
    ) -> Result<NurbsGraphPolygonHoledSolid> {
        NurbsGraphPolygonHoledSolid::new(self, opening, tol)
    }
}
impl NurbsGraphSolid {
    pub fn through_uv_polygon(
        &self,
        opening: Vec<[f64; 2]>,
        tol: Tolerance,
    ) -> Result<NurbsGraphPolygonHoledSolid> {
        let [u, v] = self.source_domain();
        let outer = NurbsGraphPolygonSolid::new(
            self,
            vec![[u[0], v[0]], [u[1], v[0]], [u[1], v[1]], [u[0], v[1]]],
            tol,
        )?;
        outer.through_uv_polygon(opening, tol)
    }
}
