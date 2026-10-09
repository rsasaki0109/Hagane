//! Closed scoped graph solid with one exact rectangular through opening.
use crate::*;
use std::collections::BTreeMap;
#[derive(Clone, Debug)]
pub struct NurbsGraphHoledSolid {
    pub solid: Solid,
    source: NurbsGraphSolid,
    hole: [[f64; 2]; 2],
    tolerance: Tolerance,
}
fn local_caps(
    source: &NurbsGraphSolid,
    hole: [[f64; 2]; 2],
    tol: Tolerance,
) -> Result<(NurbsHoledFace, NurbsHoledFace)> {
    let local = NurbsGraphSolid::new(source.dimensions(), source.bulge(), tol)?
        .trimmed_uv(source.source_domain(), tol)?;
    let Surface::Nurbs(roof) = &local.brep().shell.faces[1].surface else {
        unreachable!()
    };
    let top = NurbsHoledFace::new((**roof).clone(), source.source_domain(), vec![hole], 1, tol)?;
    let Surface::Nurbs(refined) = &top.face.surface else {
        unreachable!()
    };
    let base = NurbsSurface::new(
        refined.degrees(),
        [refined.knots(0)?.to_vec(), refined.knots(1)?.to_vec()],
        refined.control_counts(),
        refined
            .control_points()
            .iter()
            .map(|p| Point3::new(p.x, p.y, 0.))
            .collect(),
        refined.weights().to_vec(),
    )?;
    let bottom = NurbsHoledFace::new(base, source.source_domain(), vec![hole], -1, tol)?;
    Ok((bottom, top))
}
fn build(source: &NurbsGraphSolid, hole: [[f64; 2]; 2], tol: Tolerance) -> Result<Solid> {
    let (bottom, top) = local_caps(source, hole, tol)?;
    let mut solid = Solid {
        vertices: bottom.vertices.clone(),
        edges: bottom.edges.clone(),
        shell: Shell {
            faces: vec![bottom.face.clone()],
        },
    };
    solid.vertices.extend(top.vertices.clone());
    for edge in &top.edges {
        solid.edges.push(Edge {
            vertices: edge.vertices.map(|v| v + 8),
            curve: edge.curve.clone(),
        });
    }
    let mut top_face = top.face.clone();
    for c in top_face.wires.iter_mut().flat_map(|w| &mut w.coedges) {
        c.edge += 8;
    }
    solid.shell.faces.push(top_face);
    // All eight vertical edges are exact quadratic lines with shared endpoints.
    for i in 0..8 {
        let a = solid.vertices[i].point;
        let b = solid.vertices[i + 8].point;
        let curve = NurbsCurve::new(
            2,
            vec![0., 0., 0., 1., 1., 1.],
            vec![a, a + (b - a) * 0.5, b],
            vec![1.; 3],
        )?;
        solid.edges.push(Edge {
            vertices: [i, i + 8],
            curve: Curve::Nurbs(Box::new(curve)),
        });
    }
    for c in top.face.wires.iter().flat_map(|w| &w.coedges) {
        let edge = &top.edges[c.edge];
        let Curve::Nurbs(curve) = &edge.curve else {
            unreachable!()
        };
        let mut points = Vec::new();
        for p in curve.control_points() {
            for j in 0..3 {
                points.push(Point3::new(p.x, p.y, p.z * j as f64 / 2.));
            }
        }
        let surface = NurbsSurface::new(
            [curve.degree(), 2],
            [curve.knots().to_vec(), vec![0., 0., 0., 1., 1., 1.]],
            [curve.control_points().len(), 3],
            points,
            vec![1.; curve.control_points().len() * 3],
        )?;
        let local = NurbsFace::new(surface, if c.forward { 1 } else { -1 }, tol)?;
        let mut face = local.face;
        let [a, b] = edge.vertices;
        let ids = [c.edge, 16 + b, 8 + c.edge, 16 + a];
        for coedge in &mut face.wires[0].coedges {
            coedge.edge = ids[coedge.edge];
        }
        solid.shell.faces.push(face);
    }
    crate::nurbs_graph_solid::placed(solid, source.placement())
}
fn same_curve(a: &Curve, b: &Curve) -> bool {
    let (Curve::Nurbs(a), Curve::Nurbs(b)) = (a, b) else {
        return false;
    };
    a.degree() == b.degree()
        && a.knots() == b.knots()
        && a.weights() == b.weights()
        && a.control_points() == b.control_points()
}
fn same_surface(a: &Surface, b: &Surface) -> Result<bool> {
    let (Surface::Nurbs(a), Surface::Nurbs(b)) = (a, b) else {
        return Ok(false);
    };
    Ok(a.degrees() == b.degrees()
        && a.control_counts() == b.control_counts()
        && a.knots(0)? == b.knots(0)?
        && a.knots(1)? == b.knots(1)?
        && a.weights() == b.weights()
        && a.control_points() == b.control_points())
}
impl NurbsGraphHoledSolid {
    pub fn new(source: &NurbsGraphSolid, hole: [[f64; 2]; 2], tol: Tolerance) -> Result<Self> {
        source.validate(tol)?;
        let outer = source.source_domain();
        for axis in 0..2 {
            let [a, b] = hole[axis];
            let [lo, hi] = outer[axis];
            let guard = 128. * f64::EPSILON * lo.abs().max(hi.abs()).max(hi - lo);
            if !a.is_finite()
                || !b.is_finite()
                || a - lo <= guard
                || hi - b <= guard
                || b - a <= guard
                || (a - lo).min(hi - b).min(b - a) * source.dimensions()[axis] <= tol.linear * 2.
            {
                return Err(Error::InvalidInput(
                    "graph opening must be resolved and strictly inside all four walls",
                ));
            }
        }
        let result = Self {
            solid: build(source, hole, tol)?,
            source: source.clone(),
            hole,
            tolerance: tol,
        };
        result.validate(tol)?;
        result.volume()?;
        Ok(result)
    }
    pub fn source(&self) -> &NurbsGraphSolid {
        &self.source
    }
    pub fn hole(&self) -> [[f64; 2]; 2] {
        self.hole
    }
    pub fn brep(&self) -> &Solid {
        &self.solid
    }
    pub fn validate(&self, tol: Tolerance) -> Result<()> {
        self.source.validate(tol)?;
        Tolerance::new(tol.linear)?;
        let expected = build(&self.source, self.hole, tol)?;
        let bad =
            || Error::InvalidTopology("graph opening differs from its canonical closed B-rep");
        if self.solid.vertices.len() != 16
            || self.solid.edges.len() != 24
            || self.solid.shell.faces.len() != 10
        {
            return Err(bad());
        }
        if self
            .solid
            .vertices
            .iter()
            .zip(&expected.vertices)
            .any(|(a, b)| a.point != b.point)
        {
            return Err(bad());
        }
        for (a, b) in self.solid.edges.iter().zip(&expected.edges) {
            if a.vertices != b.vertices || !same_curve(&a.curve, &b.curve) {
                return Err(bad());
            }
            for k in 0..2 {
                if (a.curve.try_evaluate(a.curve.range()[k])?
                    - self.solid.vertices[a.vertices[k]].point)
                    .norm()
                    > tol.linear / 4.
                {
                    return Err(bad());
                }
            }
        }
        let mut incidence = [[0i32; 2]; 24];
        for (a, b) in self.solid.shell.faces.iter().zip(&expected.shell.faces) {
            if a.orientation != b.orientation
                || a.wires.len() != b.wires.len()
                || !same_surface(&a.surface, &b.surface)?
            {
                return Err(bad());
            }
            for (a, b) in a.wires.iter().zip(&b.wires) {
                if a.coedges.len() != b.coedges.len() {
                    return Err(bad());
                }
                for (a, b) in a.coedges.iter().zip(&b.coedges) {
                    let (
                        PCurve::Affine {
                            origin: o,
                            direction: d,
                        },
                        PCurve::Affine {
                            origin: p,
                            direction: e,
                        },
                    ) = (&a.pcurve, &b.pcurve)
                    else {
                        return Err(bad());
                    };
                    if a.edge != b.edge || a.forward != b.forward || o != p || d != e {
                        return Err(bad());
                    }
                }
            }
            for c in a.wires.iter().flat_map(|w| &w.coedges) {
                incidence[c.edge][0] += 1;
                incidence[c.edge][1] += a.orientation as i32 * if c.forward { 1 } else { -1 };
                let edge = &self.solid.edges[c.edge];
                let Surface::Nurbs(surface) = &a.surface else {
                    unreachable!()
                };
                for t in edge.curve.range() {
                    let uv = c.pcurve.evaluate(t);
                    let gap =
                        (edge.curve.try_evaluate(t)? - surface.evaluate(uv[0], uv[1])?).norm();
                    if !gap.is_finite() || gap > tol.linear / 4. {
                        return Err(bad());
                    }
                }
            }
        }
        if incidence.iter().any(|c| *c != [2, 0]) {
            return Err(bad());
        }
        Ok(())
    }
    pub fn volume(&self) -> Result<f64> {
        self.validate(self.tolerance)?;
        let [u, v] = self.source.source_domain();
        let [hu, hv] = self.hole;
        // Disjoint positive-volume strips avoid cancellation in source-minus-hole.
        let strips = [
            [[u[0], hu[0]], v],
            [[hu[1], u[1]], v],
            [hu, [v[0], hv[0]]],
            [hu, [hv[1], v[1]]],
        ];
        let mut total = 0.;
        let mut correction = 0.;
        for ranges in strips {
            let term = self.source.trimmed_uv(ranges, self.tolerance)?.volume()?;
            let next = total + term;
            correction += if total.abs() >= term.abs() {
                (total - next) + term
            } else {
                (term - next) + total
            };
            total = next;
        }
        let volume = total + correction;
        let full = self.source.volume()?;
        let removed = self
            .source
            .trimmed_uv(self.hole, self.tolerance)?
            .volume()?;
        if !volume.is_finite()
            || volume <= 0.
            || ((volume + removed) - full).abs() > 512. * f64::EPSILON * full.abs()
        {
            return Err(Error::InvalidInput(
                "graph opening volume conservation is unresolved",
            ));
        }
        Ok(volume)
    }
    /// The source enclosure is also conservative for this material subset.
    pub fn bounds(&self) -> Result<Bounds> {
        self.validate(self.tolerance)?;
        self.source.bounds()
    }
    pub fn tessellate_bounded(
        &self,
        error: f64,
        max_cells: usize,
        tol: Tolerance,
    ) -> Result<NurbsGraphMesh> {
        self.validate(tol)?;
        if !error.is_finite() || error <= 0. || !(32..=65536).contains(&max_cells) {
            return Err(Error::InvalidInput(
                "graph opening error or total cell budget is invalid",
            ));
        }
        let roundoff = self.source.arithmetic_budget()?;
        if roundoff >= error / 4. {
            return Err(Error::Tessellation(
                "graph opening world precision cannot resolve error",
            ));
        }
        // Source polynomial g=4*b*u*(1-u)*v*(1-v) has |g_uu|,|g_vv|
        // <=2|b| and |g_uv|<=4|b|. Each retained cap grid cell has both
        // source-UV widths <=1/N: bilinear interpolation plus triangle twist
        // is bounded by 1.5|b|/N². Ruled wall height has no vertical second
        // derivative and admits the smaller bound |b|/(2N²). Shared cached
        // positions and refinement arithmetic reserve the separate world guard.
        let mut n = 1usize;
        while 1.5 * self.source.bulge().abs() / (n * n) as f64 + roundoff > error {
            n *= 2;
            if 32 * n * n > max_cells {
                return Err(Error::Tessellation(
                    "graph opening total cell budget exceeded",
                ));
            }
        }
        let bound = 1.5 * self.source.bulge().abs() / (n * n) as f64 + roundoff;
        let domain = self.source.source_domain();
        let mut grid: [Vec<f64>; 2] = [Vec::new(), Vec::new()];
        for axis in 0..2 {
            let breaks = [
                domain[axis][0],
                self.hole[axis][0],
                self.hole[axis][1],
                domain[axis][1],
            ];
            for span in 0..3 {
                for j in 0..n {
                    let value = if j == 0 {
                        breaks[span]
                    } else {
                        breaks[span] + (breaks[span + 1] - breaks[span]) * j as f64 / n as f64
                    };
                    if grid[axis].last().is_some_and(|previous| *previous >= value) {
                        return Err(Error::Tessellation(
                            "graph opening UV subdivisions are unresolved",
                        ));
                    }
                    grid[axis].push(value);
                }
            }
            grid[axis].push(breaks[3]);
        }
        let mut output = NurbsGraphMesh {
            mesh: Mesh::default(),
            vertex_nodes: Vec::new(),
            vertex_uv: Vec::new(),
            vertex_faces: Vec::new(),
            error_bounds: Vec::new(),
            subdivisions: n,
        };
        let mut cache = Cache::default();
        for fi in 0..2 {
            let face = &self.solid.shell.faces[fi];
            for i in 0..3 * n {
                for j in 0..3 * n {
                    if (n..2 * n).contains(&i) && (n..2 * n).contains(&j) {
                        continue;
                    }
                    let z = if fi == 0 { 0 } else { n };
                    let keys = [[i, j, z], [i + 1, j, z], [i + 1, j + 1, z], [i, j + 1, z]];
                    let uv = keys.map(|key| [grid[0][key[0]], grid[1][key[1]]]);
                    emit(
                        &mut output,
                        &mut cache,
                        face,
                        fi,
                        keys,
                        uv,
                        if fi == 0 { roundoff } else { bound },
                        roundoff,
                    )?;
                }
            }
        }
        for (wall, c) in self.solid.shell.faces[1]
            .wires
            .iter()
            .flat_map(|w| &w.coedges)
            .enumerate()
        {
            let edge = &self.solid.edges[c.edge];
            let PCurve::Affine { origin, direction } = c.pcurve else {
                unreachable!()
            };
            let axis = usize::from(direction[1] != 0.);
            let fixed = 1 - axis;
            let fixed_index = grid[fixed].iter().position(|x| *x == origin[fixed]).ok_or(
                Error::InvalidTopology("opening wall is outside canonical UV grid"),
            )?;
            let range = edge.curve.range();
            let start = grid[axis]
                .iter()
                .position(|x| *x == range[0])
                .ok_or(Error::InvalidTopology("opening wall start misses UV grid"))?;
            let end = grid[axis]
                .iter()
                .position(|x| *x == range[1])
                .ok_or(Error::InvalidTopology("opening wall end misses UV grid"))?;
            let fi = wall + 2;
            let face = &self.solid.shell.faces[fi];
            for i in start..end {
                for j in 0..n {
                    let key = |k, z| {
                        if axis == 0 {
                            [k, fixed_index, z]
                        } else {
                            [fixed_index, k, z]
                        }
                    };
                    let keys = [key(i, j), key(i + 1, j), key(i + 1, j + 1), key(i, j + 1)];
                    let uv = [
                        [grid[axis][i], j as f64 / n as f64],
                        [grid[axis][i + 1], j as f64 / n as f64],
                        [grid[axis][i + 1], (j + 1) as f64 / n as f64],
                        [grid[axis][i], (j + 1) as f64 / n as f64],
                    ];
                    emit(&mut output, &mut cache, face, fi, keys, uv, bound, roundoff)?;
                }
            }
        }
        Ok(output)
    }
}
#[derive(Default)]
struct Cache {
    nodes: BTreeMap<[usize; 3], (usize, Point3)>,
    display: BTreeMap<(usize, [usize; 3]), usize>,
}
#[allow(clippy::too_many_arguments)]
fn emit(
    output: &mut NurbsGraphMesh,
    cache: &mut Cache,
    face: &Face,
    fi: usize,
    keys: [[usize; 3]; 4],
    uv: [[f64; 2]; 4],
    bound: f64,
    roundoff: f64,
) -> Result<()> {
    let Surface::Nurbs(surface) = &face.surface else {
        unreachable!()
    };
    let mut indices = [0; 4];
    for k in 0..4 {
        if let Some(&index) = cache.display.get(&(fi, keys[k])) {
            indices[k] = index;
            continue;
        }
        let evaluation =
            surface.evaluate_with_partials(uv[k][0], uv[k][1], [KnotSide::Right; 2])?;
        let next = cache.nodes.len();
        let (node, position) = *cache
            .nodes
            .entry(keys[k])
            .or_insert((next, evaluation.point));
        let gap = (position - evaluation.point).norm();
        if !gap.is_finite() || gap > roundoff {
            return Err(Error::Tessellation(
                "graph opening shared rim evaluations disagree",
            ));
        }
        let index = output.mesh.positions.len();
        cache.display.insert((fi, keys[k]), index);
        indices[k] = index;
        output.mesh.positions.push(position);
        output
            .mesh
            .normals
            .push(evaluation.normal()? * face.orientation as f64);
        output.vertex_nodes.push(node);
        output.vertex_uv.push(uv[k]);
        output.vertex_faces.push(fi);
    }
    for mut tri in [
        [indices[0], indices[1], indices[2]],
        [indices[0], indices[2], indices[3]],
    ] {
        if face.orientation < 0 {
            tri.swap(1, 2);
        }
        let normal = (output.mesh.positions[tri[1]] - output.mesh.positions[tri[0]])
            .cross(output.mesh.positions[tri[2]] - output.mesh.positions[tri[0]])
            .normalized()?;
        if tri
            .iter()
            .any(|&v| normal.dot(output.mesh.normals[v]) <= 0.)
        {
            return Err(Error::Tessellation(
                "graph opening triangle orientation is unresolved",
            ));
        }
        output.mesh.triangles.push(tri);
        output.mesh.face_ids.push(fi);
        output.error_bounds.push(bound);
    }
    Ok(())
}
