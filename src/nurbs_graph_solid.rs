//! A deliberately restricted closed polynomial graph B-rep.
use crate::*;
use std::collections::BTreeMap;
#[derive(Clone, Debug)]
pub struct NurbsGraphSolid {
    pub solid: Solid,
    dimensions: [f64; 3],
    bulge: f64,
    construction_tolerance: Tolerance,
    placement: Transform,
    source_domain: [[f64; 2]; 2],
}
#[derive(Clone, Debug)]
pub struct NurbsGraphMesh {
    pub mesh: Mesh,
    pub vertex_nodes: Vec<usize>,
    pub vertex_uv: Vec<[f64; 2]>,
    pub vertex_faces: Vec<usize>,
    pub error_bounds: Vec<f64>,
    pub subdivisions: usize,
}
const CORNERS: [[usize; 4]; 6] = [
    [0, 1, 3, 2],
    [4, 5, 7, 6],
    [0, 2, 6, 4],
    [1, 3, 7, 5],
    [0, 1, 5, 4],
    [2, 3, 7, 6],
];
const FULL: [[f64; 2]; 2] = [[0., 1.], [0., 1.]];
// Shared exact construction order for retained graph roofs and strict import
// candidate rejection. Shape admission remains with the full solid constructor.
pub(crate) fn graph_roof(d: [f64; 3], b: f64, ranges: [[f64; 2]; 2]) -> Result<NurbsSurface> {
    let [l, w, h] = d;
    let mut points = Vec::new();
    for i in 0..3 {
        for j in 0..3 {
            points.push(Point3::new(
                l * i as f64 / 2.,
                w * j as f64 / 2.,
                h + if i == 1 && j == 1 { b } else { 0. },
            ));
        }
    }
    let knots = vec![0., 0., 0., 1., 1., 1.];
    let original = NurbsSurface::new([2, 2], [knots.clone(), knots], [3, 3], points, vec![1.; 9])?;
    let roof = if ranges == FULL {
        original
    } else {
        original.restricted(ranges)?
    };
    Ok(roof)
}
fn build(d: [f64; 3], b: f64, ranges: [[f64; 2]; 2], tol: Tolerance) -> Result<Solid> {
    let roof = graph_roof(d, b, ranges)?;
    let roof_edges = roof.boundary_edges()?;
    let base_points = roof
        .control_points()
        .iter()
        .map(|p| Point3::new(p.x, p.y, 0.))
        .collect();
    let base = NurbsSurface::new(
        [2, 2],
        [roof.knots(0)?.to_vec(), roof.knots(1)?.to_vec()],
        [3, 3],
        base_points,
        roof.weights().to_vec(),
    )?;
    let mut vertices = vec![
        Vertex {
            point: Point3::new(0., 0., 0.)
        };
        8
    ];
    for (k, [u, v]) in [
        [ranges[0][0], ranges[1][0]],
        [ranges[0][1], ranges[1][0]],
        [ranges[0][0], ranges[1][1]],
        [ranges[0][1], ranges[1][1]],
    ]
    .into_iter()
    .enumerate()
    {
        let top = roof.evaluate(u, v)?;
        vertices[k] = Vertex {
            point: Point3::new(top.x, top.y, 0.),
        };
        vertices[k + 4] = Vertex { point: top };
    }
    let mut surfaces = vec![base, roof];
    for boundary in [3, 1, 0, 2] {
        let curve = &roof_edges[boundary].curve;
        let mut points = Vec::new();
        for p in curve.control_points() {
            for j in 0..3 {
                points.push(Point3::new(p.x, p.y, p.z * (j as f64 / 2.)));
            }
        }
        surfaces.push(NurbsSurface::new(
            [2, 2],
            [curve.knots().to_vec(), vec![0., 0., 0., 1., 1., 1.]],
            [3, 3],
            points,
            vec![1.; 9],
        )?);
    }
    let mut solid = Solid {
        vertices,
        edges: Vec::new(),
        shell: Shell { faces: Vec::new() },
    };
    let mut edges = BTreeMap::new();
    for (f, surface) in surfaces.into_iter().enumerate() {
        let local = NurbsFace::new(surface, [-1, 1, -1, 1, 1, -1][f], tol)?;
        let mut face = local.face;
        for coedge in &mut face.wires[0].coedges {
            let edge = &local.edges[coedge.edge];
            let pair = edge.vertices.map(|i| CORNERS[f][i]);
            let id = if let Some(&id) = edges.get(&pair) {
                id
            } else {
                let id = solid.edges.len();
                solid.edges.push(Edge {
                    vertices: pair,
                    curve: edge.curve.clone(),
                });
                edges.insert(pair, id);
                id
            };
            coedge.edge = id;
        }
        solid.shell.faces.push(face);
    }
    Ok(solid)
}
pub(crate) fn placed(mut solid: Solid, transform: Transform) -> Result<Solid> {
    if transform == Transform::IDENTITY {
        return Ok(solid);
    }
    for vertex in &mut solid.vertices {
        vertex.point = transform.point(vertex.point);
        if !vertex.point.finite() {
            return Err(Error::InvalidInput("graph placement overflows"));
        }
    }
    for edge in &mut solid.edges {
        edge.curve = edge.curve.transformed(transform)?;
    }
    for face in &mut solid.shell.faces {
        face.surface = face.surface.transformed(transform)?;
    }
    Ok(solid)
}
impl NurbsGraphSolid {
    pub fn new(dimensions: [f64; 3], bulge: f64, tol: Tolerance) -> Result<Self> {
        Tolerance::new(tol.linear)?;
        let scale = dimensions.into_iter().fold(bulge.abs(), f64::max);
        let guard = 128. * f64::EPSILON * scale;
        if dimensions
            .iter()
            .any(|x| !x.is_finite() || *x <= guard.max(tol.linear * 2.))
            || !bulge.is_finite()
            || !(dimensions[2] + bulge).is_finite()
            || dimensions[2] + bulge <= guard.max(tol.linear * 2.)
            || (bulge != 0. && dimensions[2] + bulge == dimensions[2])
        {
            return Err(Error::InvalidInput(
                "graph dimensions or roof height are unresolved",
            ));
        }
        let result = Self {
            solid: build(dimensions, bulge, FULL, tol)?,
            dimensions,
            bulge,
            construction_tolerance: tol,
            placement: Transform::IDENTITY,
            source_domain: FULL,
        };
        result.volume()?;
        result.bounds()?;
        result.validate(tol)?;
        Ok(result)
    }
    pub fn source_domain(&self) -> [[f64; 2]; 2] {
        self.source_domain
    }
    /// Restrict in original source UV coordinates, preserving rigid placement.
    pub fn trimmed_uv(&self, ranges: [[f64; 2]; 2], tol: Tolerance) -> Result<Self> {
        self.validate(tol)?;
        for (axis, &[a, b]) in ranges.iter().enumerate() {
            let current = self.source_domain[axis];
            let guard = 128.
                * f64::EPSILON
                * current[0]
                    .abs()
                    .max(current[1].abs())
                    .max(current[1] - current[0]);
            if !a.is_finite()
                || !b.is_finite()
                || a < current[0]
                || b > current[1]
                || b - a <= guard
            {
                return Err(Error::InvalidInput(
                    "graph trim is outside current domain or unresolved",
                ));
            }
            if (b - a) * self.dimensions[axis] <= tol.linear * 2. {
                return Err(Error::InvalidInput(
                    "graph trim physical width is unresolved",
                ));
            }
        }
        let result = Self {
            solid: placed(
                build(self.dimensions, self.bulge, ranges, tol)?,
                self.placement,
            )?,
            dimensions: self.dimensions,
            bulge: self.bulge,
            construction_tolerance: tol,
            placement: self.placement,
            source_domain: ranges,
        };
        result.validate(tol)?;
        result.volume()?;
        Ok(result)
    }
    pub fn placement(&self) -> Transform {
        self.placement
    }
    /// Regenerate from the local canonical source with a composed rigid placement.
    pub fn transformed(&self, transform: Transform, tol: Tolerance) -> Result<Self> {
        self.validate(tol)?;
        let placement = transform.compose(self.placement)?;
        let result = Self {
            solid: placed(
                build(self.dimensions, self.bulge, self.source_domain, tol)?,
                placement,
            )?,
            dimensions: self.dimensions,
            bulge: self.bulge,
            construction_tolerance: tol,
            placement,
            source_domain: self.source_domain,
        };
        result.validate(tol)?;
        Ok(result)
    }
    pub(crate) fn arithmetic_budget(&self) -> Result<f64> {
        let axes = self.placement.axes();
        let origin = self.placement.origin();
        let extents = [
            self.dimensions[0],
            self.dimensions[1],
            self.dimensions[2].max((self.dimensions[2] + self.bulge).abs()),
        ];
        let mut scale: f64 = 0.;
        for k in 0..3 {
            let component = |p: Point3| [p.x, p.y, p.z][k];
            let envelope = component(origin).abs()
                + (0..3)
                    .map(|i| component(axes[i]).abs() * extents[i])
                    .sum::<f64>();
            if !envelope.is_finite() {
                return Err(Error::InvalidInput(
                    "graph placement coefficient envelope overflows",
                ));
            }
            scale = scale.max(envelope);
        }
        let budget = 4096. * f64::EPSILON * scale * 6.;
        if !budget.is_finite() {
            return Err(Error::InvalidInput(
                "graph placement arithmetic budget overflows",
            ));
        }
        Ok(budget)
    }
    pub fn brep(&self) -> &Solid {
        &self.solid
    }
    pub fn dimensions(&self) -> [f64; 3] {
        self.dimensions
    }
    pub fn bulge(&self) -> f64 {
        self.bulge
    }
    pub fn volume(&self) -> Result<f64> {
        self.validate(self.construction_tolerance)?;
        let [l, w, h] = self.dimensions;
        let v = if self.source_domain == FULL {
            l * w * (h + self.bulge / 9.)
        } else {
            let integral = |[a, b]: [f64; 2]| {
                let d = b - a;
                let m = a + d / 2.;
                d * (m * (1. - m) - d * d / 12.)
            };
            let [u, v] = self.source_domain;
            l * w
                * (h * (u[1] - u[0]) * (v[1] - v[0]) + 4. * self.bulge * integral(u) * integral(v))
        };
        if !v.is_finite() || v <= 0. {
            return Err(Error::InvalidInput("graph volume overflows or underflows"));
        }
        Ok(v)
    }
    /// Exact bounds for the original identity solid; conservative control-hull bounds after trim or placement.
    pub fn bounds(&self) -> Result<Bounds> {
        self.validate(self.construction_tolerance)?;
        if self.placement != Transform::IDENTITY || self.source_domain != FULL {
            let mut min = Point3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
            let mut max = Point3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
            for face in &self.solid.shell.faces {
                let Surface::Nurbs(surface) = &face.surface else {
                    unreachable!()
                };
                for p in surface.control_points() {
                    min.x = min.x.min(p.x);
                    min.y = min.y.min(p.y);
                    min.z = min.z.min(p.z);
                    max.x = max.x.max(p.x);
                    max.y = max.y.max(p.y);
                    max.z = max.z.max(p.z);
                }
            }
            return Ok(Bounds { min, max });
        }
        let [l, w, h] = self.dimensions;
        let z = h + self.bulge.max(0.) / 4.;
        if !z.is_finite() {
            return Err(Error::InvalidInput("graph bounds overflow"));
        }
        Ok(Bounds {
            min: Point3::new(0., 0., 0.),
            max: Point3::new(l, w, z),
        })
    }
    /// Validate the retained, canonical six-face representation, not a generic NURBS solid.
    pub fn validate(&self, tol: Tolerance) -> Result<()> {
        Tolerance::new(tol.linear)?;
        if self.placement != Transform::IDENTITY && self.arithmetic_budget()? >= tol.linear / 4. {
            return Err(Error::InvalidInput(
                "graph placed geometry cannot resolve validation tolerance",
            ));
        }
        let expected = placed(
            build(self.dimensions, self.bulge, self.source_domain, tol)?,
            self.placement,
        )?;
        let bad = || Error::InvalidTopology("graph solid differs from its canonical B-rep");
        if self.solid.vertices.len() != 8
            || self.solid.edges.len() != 12
            || self.solid.shell.faces.len() != 6
        {
            return Err(bad());
        }
        for (a, b) in self.solid.vertices.iter().zip(&expected.vertices) {
            if a.point != b.point {
                return Err(bad());
            }
        }
        for (a, b) in self.solid.edges.iter().zip(&expected.edges) {
            if a.vertices != b.vertices {
                return Err(bad());
            }
            let (Curve::Nurbs(a), Curve::Nurbs(b)) = (&a.curve, &b.curve) else {
                return Err(bad());
            };
            if a.degree() != b.degree()
                || a.knots() != b.knots()
                || a.weights() != b.weights()
                || a.control_points().len() != b.control_points().len()
                || a.control_points()
                    .iter()
                    .zip(b.control_points())
                    .any(|(a, b)| *a != *b)
            {
                return Err(bad());
            }
        }
        let mut incidence = [[0i32; 2]; 12];
        for (a, b) in self.solid.shell.faces.iter().zip(&expected.shell.faces) {
            if a.orientation != b.orientation || a.wires.len() != 1 || a.wires[0].coedges.len() != 4
            {
                return Err(bad());
            }
            let (Surface::Nurbs(sa), Surface::Nurbs(sb)) = (&a.surface, &b.surface) else {
                return Err(bad());
            };
            if sa.degrees() != sb.degrees()
                || sa.weights() != sb.weights()
                || sa.control_points().len() != sb.control_points().len()
                || sa
                    .control_points()
                    .iter()
                    .zip(sb.control_points())
                    .any(|(a, b)| *a != *b)
                || sa.knots(0)? != sb.knots(0)?
                || sa.knots(1)? != sb.knots(1)?
            {
                return Err(bad());
            }
            for (c, d) in a.wires[0].coedges.iter().zip(&b.wires[0].coedges) {
                if c.edge != d.edge || c.forward != d.forward {
                    return Err(bad());
                }
                let (
                    PCurve::Affine {
                        origin: o,
                        direction: r,
                    },
                    PCurve::Affine {
                        origin: p,
                        direction: s,
                    },
                ) = (&c.pcurve, &d.pcurve)
                else {
                    return Err(bad());
                };
                if o != p || r != s {
                    return Err(bad());
                }
                incidence[c.edge][0] += 1;
                incidence[c.edge][1] += a.orientation as i32 * if c.forward { 1 } else { -1 };
                let edge = &self.solid.edges[c.edge];
                for k in 0..2 {
                    let t = edge.curve.range()[k];
                    let point = edge.curve.try_evaluate(t)?;
                    let uv = c.pcurve.evaluate(t);
                    if (point - self.solid.vertices[edge.vertices[k]].point).norm() > tol.linear
                        || (point - sa.evaluate(uv[0], uv[1])?).norm() > tol.linear
                    {
                        return Err(bad());
                    }
                }
            }
        }
        if incidence.iter().any(|a| *a != [2, 0]) {
            return Err(bad());
        }
        Ok(())
    }
    /// Uniform conforming face grids, with topological node IDs and separate crease normals.
    pub fn tessellate_bounded(
        &self,
        error: f64,
        max_cells: usize,
        tol: Tolerance,
    ) -> Result<NurbsGraphMesh> {
        self.validate(tol)?;
        if !error.is_finite() || error <= 0. || !(6..=65536).contains(&max_cells) {
            return Err(Error::InvalidInput(
                "graph error and total cell budget are invalid",
            ));
        }
        let roundoff = self.arithmetic_budget()?;
        if !roundoff.is_finite() || roundoff >= error / 4. {
            return Err(Error::Tessellation(
                "graph coordinate precision cannot resolve error",
            ));
        }
        // g = 4*b*u*(1-u)*v*(1-v): |g_uu|,|g_vv| <= 2|b|,
        // |g_uv| <= 4|b|. Tensor linear interpolation contributes
        // |b|/(2*N²); the bilinear-to-triangle twist adds |b|/N².
        // Corresponding UV points give both directed geometric bounds.
        // The separate arithmetic allowance is an engineering guard.
        let du = self.source_domain[0][1] - self.source_domain[0][0];
        let dv = self.source_domain[1][1] - self.source_domain[1][0];
        let curvature = self.bulge.abs()
            * ((du * du + dv * dv) / 4. + du * dv)
                .max((du * du + du) / 4.)
                .max((dv * dv + dv) / 4.);
        let mut n = 1usize;
        while curvature / (n * n) as f64 + roundoff > error {
            n *= 2;
            if 6 * n * n > max_cells {
                return Err(Error::Tessellation("graph total cell budget exceeded"));
            }
        }
        let bound = curvature / (n * n) as f64 + roundoff;
        let mut out = NurbsGraphMesh {
            mesh: Mesh::default(),
            vertex_nodes: Vec::new(),
            vertex_uv: Vec::new(),
            vertex_faces: Vec::new(),
            error_bounds: Vec::new(),
            subdivisions: n,
        };
        let mut nodes: BTreeMap<[usize; 3], (usize, Point3)> = BTreeMap::new();
        for (fi, face) in self.solid.shell.faces.iter().enumerate() {
            let Surface::Nurbs(surface) = &face.surface else {
                unreachable!()
            };
            let base = out.mesh.positions.len();
            for i in 0..=n {
                for j in 0..=n {
                    let domain = surface.domain();
                    let uv = std::array::from_fn(|axis| {
                        let step = if axis == 0 { i } else { j };
                        if step == n {
                            domain[axis][1]
                        } else {
                            domain[axis][0]
                                + (domain[axis][1] - domain[axis][0]) * step as f64 / n as f64
                        }
                    });
                    let key = match fi {
                        0 => [i, j, 0],
                        1 => [i, j, n],
                        2 => [0, i, j],
                        3 => [n, i, j],
                        4 => [i, 0, j],
                        _ => [i, n, j],
                    };
                    let candidate = surface.evaluate(uv[0], uv[1])?;
                    let next = nodes.len();
                    let (node, position) = *nodes.entry(key).or_insert((next, candidate));
                    if (candidate - position).norm() > roundoff {
                        return Err(Error::Tessellation(
                            "graph shared edge coordinates disagree",
                        ));
                    }
                    out.mesh.positions.push(position);
                    out.mesh
                        .normals
                        .push(surface.normal(uv[0], uv[1])? * face.orientation as f64);
                    out.vertex_nodes.push(node);
                    out.vertex_uv.push(uv);
                    out.vertex_faces.push(fi);
                }
            }
            for i in 0..n {
                for j in 0..n {
                    let a = base + i * (n + 1) + j;
                    let b = a + n + 1;
                    let c = b + 1;
                    let d = a + 1;
                    for mut t in [[a, b, c], [a, c, d]] {
                        if face.orientation < 0 {
                            t.swap(1, 2);
                        }
                        let normal = (out.mesh.positions[t[1]] - out.mesh.positions[t[0]])
                            .cross(out.mesh.positions[t[2]] - out.mesh.positions[t[0]])
                            .normalized()?;
                        if t.iter().any(|&v| normal.dot(out.mesh.normals[v]) <= 0.) {
                            return Err(Error::Tessellation(
                                "graph triangle orientation is unresolved",
                            ));
                        }
                        out.mesh.triangles.push(t);
                        out.mesh.face_ids.push(fi);
                        out.error_bounds.push(
                            if fi == 0 || (fi != 1 && self.source_domain == FULL) {
                                roundoff
                            } else {
                                bound
                            },
                        );
                    }
                }
            }
        }
        Ok(out)
    }
}
