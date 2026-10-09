//! Checked graph stock with one through four exact convex polygon openings.
use crate::*;
#[derive(Clone, Debug)]
pub struct NurbsGraphPolygonMultiHoledSolid {
    pub solid: Solid,
    outer: NurbsGraphPolygonSolid,
    openings: Vec<Vec<[f64; 2]>>,
    construction_tolerance: Tolerance,
}
fn build(
    outer: &NurbsGraphPolygonSolid,
    openings: &[Vec<[f64; 2]>],
    tol: Tolerance,
) -> Result<Solid> {
    outer.validate(tol)?;
    if !(1..=4).contains(&openings.len())
        || openings.iter().any(|p| !(3..=16).contains(&p.len()))
        || outer.polygon().len() + openings.iter().map(Vec::len).sum::<usize>() > 64
    {
        return Err(Error::Unsupported(
            "graph stock supports 1 through 4 polygon openings, at most 64 total corners",
        ));
    }
    let source = outer.source();
    let [l, w, _] = source.dimensions();
    let margin = 4. * tol.linear + source.arithmetic_budget()?;
    let mut bodies = vec![outer.clone()];
    for opening in openings {
        let body = NurbsGraphPolygonSolid::new(source, opening.clone(), tol)?;
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
                        "each polygon opening must lie strictly inside resolved outer walls",
                    ));
                }
            }
        }
        bodies.push(body);
    }
    for i in 0..openings.len() {
        for j in i + 1..openings.len() {
            let mut separated = false;
            for (a, b) in [(&openings[i], &openings[j]), (&openings[j], &openings[i])] {
                for k in 0..a.len() {
                    let p = a[k];
                    let q = a[(k + 1) % a.len()];
                    let dx = l * (q[0] - p[0]);
                    let dy = w * (q[1] - p[1]);
                    let length = dx.hypot(dy);
                    let mut all = true;
                    for v in b {
                        let gap = (dx / length) * (w * (v[1] - p[1]))
                            - (dy / length) * (l * (v[0] - p[0]));
                        if !gap.is_finite() {
                            return Err(Error::InvalidInput(
                                "polygon opening separation arithmetic overflows",
                            ));
                        }
                        all &= gap < -margin;
                    }
                    separated |= all;
                }
            }
            if !separated {
                return Err(Error::Unsupported(
                    "polygon openings must be disjoint with a resolved separating axis",
                ));
            }
        }
    }
    crate::nurbs_graph_polygon_material::material_regions(
        outer.polygon(),
        openings,
        [l, w],
        margin,
    )?;
    let counts: Vec<_> = bodies.iter().map(|b| b.polygon().len()).collect();
    let n = counts.iter().sum::<usize>();
    let mut offsets = vec![];
    let mut offset = 0;
    for count in &counts {
        offsets.push(offset);
        offset += count;
    }
    let vertex_id = |i: usize, count: usize, offset: usize| {
        if i < count {
            offset + i
        } else {
            n + offset + i - count
        }
    };
    let edge_id = |i: usize, count: usize, offset: usize| i / count * n + offset + i % count;
    let mut solid = Solid {
        vertices: vec![],
        edges: vec![],
        shell: Shell { faces: vec![] },
    };
    for top in 0..2 {
        for (body, count) in bodies.iter().zip(&counts) {
            solid
                .vertices
                .extend_from_slice(&body.brep().vertices[top * count..(top + 1) * count]);
        }
    }
    for category in 0..3 {
        for ((body, count), offset) in bodies.iter().zip(&counts).zip(&offsets) {
            for edge in &body.brep().edges[category * count..(category + 1) * count] {
                let mut edge = edge.clone();
                edge.vertices = edge.vertices.map(|i| vertex_id(i, *count, *offset));
                solid.edges.push(edge);
            }
        }
    }
    for cap in 0..2 {
        let mut face = bodies[0].brep().shell.faces[cap].clone();
        for c in &mut face.wires[0].coedges {
            c.edge = edge_id(c.edge, counts[0], 0);
        }
        for k in 1..bodies.len() {
            let mut wire = bodies[k].brep().shell.faces[cap].wires[0].clone();
            wire.coedges.reverse();
            for c in &mut wire.coedges {
                c.edge = edge_id(c.edge, counts[k], offsets[k]);
                c.forward = !c.forward;
            }
            face.wires.push(wire);
        }
        solid.shell.faces.push(face);
    }
    for (k, body) in bodies.iter().enumerate() {
        for original in &body.brep().shell.faces[2..] {
            let mut face = original.clone();
            if k > 0 {
                face.orientation = -face.orientation;
            }
            for c in &mut face.wires[0].coedges {
                c.edge = edge_id(c.edge, counts[k], offsets[k]);
            }
            solid.shell.faces.push(face);
        }
    }
    Ok(solid)
}
impl NurbsGraphPolygonMultiHoledSolid {
    pub fn new(
        outer: &NurbsGraphPolygonSolid,
        openings: Vec<Vec<[f64; 2]>>,
        tol: Tolerance,
    ) -> Result<Self> {
        let solid = build(outer, &openings, tol)?;
        let body = Self {
            solid,
            outer: outer.clone(),
            openings,
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
    pub fn polygon(&self) -> &[[f64; 2]] {
        self.outer_polygon()
    }
    pub fn openings(&self) -> &[Vec<[f64; 2]>] {
        &self.openings
    }
    pub fn holes(&self) -> &[Vec<[f64; 2]>] {
        self.openings()
    }
    pub fn genus(&self) -> usize {
        self.openings.len()
    }
    pub fn brep(&self) -> &Solid {
        &self.solid
    }
    pub(crate) fn construction_tolerance(&self) -> Tolerance {
        self.construction_tolerance
    }
    pub(crate) fn material(
        &self,
    ) -> Result<crate::nurbs_graph_polygon_material::NurbsGraphMaterial> {
        crate::nurbs_graph_polygon_material::material_regions(
            self.outer_polygon(),
            self.openings(),
            [self.source().dimensions()[0], self.source().dimensions()[1]],
            4. * self.construction_tolerance.linear + self.source().arithmetic_budget()?,
        )
    }
    pub fn bounds(&self) -> Result<Bounds> {
        self.validate(self.construction_tolerance)?;
        self.source().bounds()
    }
    pub fn validate(&self, tol: Tolerance) -> Result<()> {
        let expected = build(&self.outer, &self.openings, tol)?;
        let n = self.outer_polygon().len() + self.openings.iter().map(Vec::len).sum::<usize>();
        let bad = || {
            Error::InvalidTopology(
                "polygon openings differ from canonical multiply-holed graph B-rep",
            )
        };
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
            || self.solid.vertices.len() as isize - self.solid.edges.len() as isize
                + self.solid.shell.faces.len() as isize
                - 2 * self.openings.len() as isize
                != 2 - 2 * self.openings.len() as isize
        {
            return Err(bad());
        }
        Ok(())
    }
}
impl NurbsGraphPolygonSolid {
    pub fn through_uv_polygons(
        &self,
        openings: Vec<Vec<[f64; 2]>>,
        tol: Tolerance,
    ) -> Result<NurbsGraphPolygonMultiHoledSolid> {
        NurbsGraphPolygonMultiHoledSolid::new(self, openings, tol)
    }
}
impl NurbsGraphSolid {
    pub fn through_uv_polygons(
        &self,
        openings: Vec<Vec<[f64; 2]>>,
        tol: Tolerance,
    ) -> Result<NurbsGraphPolygonMultiHoledSolid> {
        let [u, v] = self.source_domain();
        let outer = NurbsGraphPolygonSolid::new(
            self,
            vec![[u[0], v[0]], [u[1], v[0]], [u[1], v[1]], [u[0], v[1]]],
            tol,
        )?;
        outer.through_uv_polygons(openings, tol)
    }
}
