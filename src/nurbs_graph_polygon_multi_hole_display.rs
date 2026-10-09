//! Conforming display sampling of the actual convex polygon graph B-rep with several polygon openings.
use crate::*;
use std::collections::BTreeMap;
type Key = [usize; 5];
#[derive(Default)]
struct Cache {
    nodes: BTreeMap<Key, (usize, Point3)>,
    display: BTreeMap<(usize, Key), usize>,
}
impl NurbsGraphPolygonMultiHoledSolid {
    /// Uniform conforming material-triangle and wall grids. Bounds include an engineering
    /// floating-point allowance; they are not interval arithmetic certificates.
    pub fn tessellate_bounded(
        &self,
        error: f64,
        max_triangles: usize,
        tol: Tolerance,
    ) -> Result<NurbsGraphMesh> {
        self.validate(tol)?;
        let corners = self.polygon().len() + self.holes().iter().map(Vec::len).sum::<usize>();
        let holes = self.holes().len();
        let base_triangles = 4 * (corners + holes - 1);
        if !error.is_finite() || error <= 0. || !(base_triangles..=65536).contains(&max_triangles) {
            return Err(Error::InvalidInput(
                "graph polygon opening error or triangle budget is invalid",
            ));
        }
        let arithmetic = self.source().arithmetic_budget()?;
        if !arithmetic.is_finite() || arithmetic >= error / 4. {
            return Err(Error::Tessellation(
                "graph polygon opening precision cannot resolve error",
            ));
        }
        // Affine UV restriction can perturb unit weights through roundoff.
        // A rational convex combination differs from its unit-weight version
        // by at most 2*delta/(1-delta) times the control-hull diameter.
        // Reserve this contribution inside the source engineering allowance.
        for face in &self.brep().shell.faces {
            let Surface::Nurbs(surface) = &face.surface else {
                return Err(Error::InvalidTopology(
                    "graph polygon opening display requires NURBS",
                ));
            };
            let delta = surface
                .weights()
                .iter()
                .map(|w| (w - 1.).abs())
                .fold(0., f64::max);
            let first = surface.control_points()[0];
            let radius = surface
                .control_points()
                .iter()
                .map(|p| (*p - first).norm())
                .fold(0., f64::max);
            let perturbation = 4. * delta / (1. - delta) * radius;
            if delta > 256. * f64::EPSILON
                || !perturbation.is_finite()
                || perturbation > arithmetic / 4.
            {
                return Err(Error::Tessellation(
                    "graph polygon opening rational perturbation exceeds arithmetic allowance",
                ));
            }
        }
        // Source roof Hessian norm <=6|b|. Cap triangle UV diameter <=sqrt(2)/N.
        // On a ruled wall |h_tt|<=12|b| and |h_t|<=2|b|; its height
        // fraction contributes a mixed derivative. 14|b|/N² bounds both.
        let curvature = 14. * self.source().bulge().abs();
        let limit = ((max_triangles / base_triangles) as f64).sqrt().floor() as usize;
        let mut n = 1;
        while curvature / (n * n) as f64 + arithmetic > error {
            if n >= limit {
                return Err(Error::Tessellation(
                    "graph polygon opening triangle budget exceeded",
                ));
            }
            n += 1;
        }
        let bound = curvature / (n * n) as f64 + arithmetic;
        let mut out = NurbsGraphMesh {
            mesh: Mesh::default(),
            vertex_nodes: vec![],
            vertex_uv: vec![],
            vertex_faces: vec![],
            error_bounds: vec![],
            subdivisions: n,
        };
        let mut cache = Cache::default();
        let material = self.material()?;
        let points = material.points;
        let triangles = material.triangles;
        if points.len() != corners || triangles.len() != corners + 2 * holes - 2 {
            return Err(Error::InvalidTopology(
                "polygon multi-opening triangulation count differs",
            ));
        }
        for cap in 0..2 {
            for (ti, &triangle) in triangles.iter().enumerate() {
                let vertex = |a: usize, b: usize| {
                    let weights = [n - a - b, a, b];
                    let active: Vec<_> = (0..3).filter(|&k| weights[k] > 0).collect();
                    let key = match *active.as_slice() {
                        [i] => corner_key(triangle[i], cap * n),
                        [i, j] => edge_key(triangle[i], triangle[j], weights[j], n, cap * n),
                        _ => [2, ti, a, b, cap],
                    };
                    let uv = match *active.as_slice() {
                        [i] => points[triangle[i]],
                        [i, j] => edge_uv(&points, triangle[i], triangle[j], weights[j], n),
                        _ => std::array::from_fn(|axis| {
                            let rim = closed_lerp(
                                points[triangle[1]][axis],
                                points[triangle[2]][axis],
                                b as f64,
                                (a + b) as f64,
                            );
                            closed_lerp(points[triangle[0]][axis], rim, (a + b) as f64, n as f64)
                        }),
                    };
                    (key, uv)
                };
                for a in 0..n {
                    for b in 0..n - a {
                        emit(
                            &mut out,
                            &mut cache,
                            &self.brep().shell.faces[cap],
                            cap,
                            [vertex(a, b), vertex(a + 1, b), vertex(a, b + 1)],
                            if cap == 0 { arithmetic } else { bound },
                            arithmetic,
                        )?;
                        if a + b + 1 < n {
                            emit(
                                &mut out,
                                &mut cache,
                                &self.brep().shell.faces[cap],
                                cap,
                                [vertex(a + 1, b), vertex(a + 1, b + 1), vertex(a, b + 1)],
                                if cap == 0 { arithmetic } else { bound },
                                arithmetic,
                            )?;
                        }
                    }
                }
            }
        }
        let mut offset = 0;
        for polygon in std::iter::once(self.polygon()).chain(self.holes().iter().map(Vec::as_slice))
        {
            for edge in 0..polygon.len() {
                let start = offset + edge;
                let end = offset + (edge + 1) % polygon.len();
                let fi = 2 + offset + edge;
                let vertex = |i, j| {
                    (
                        edge_key(start, end, i, n, j),
                        [i as f64 / n as f64, j as f64 / n as f64],
                    )
                };
                for i in 0..n {
                    for j in 0..n {
                        let v = [
                            vertex(i, j),
                            vertex(i + 1, j),
                            vertex(i + 1, j + 1),
                            vertex(i, j + 1),
                        ];
                        emit(
                            &mut out,
                            &mut cache,
                            &self.brep().shell.faces[fi],
                            fi,
                            [v[0], v[1], v[2]],
                            bound,
                            arithmetic,
                        )?;
                        emit(
                            &mut out,
                            &mut cache,
                            &self.brep().shell.faces[fi],
                            fi,
                            [v[0], v[2], v[3]],
                            bound,
                            arithmetic,
                        )?;
                    }
                }
            }
            offset += polygon.len();
        }
        Ok(out)
    }
}
// The exact interpolation belongs to [min(a,b),max(a,b)]. Enforcing
// that closed interval only corrects floating-point overshoot; endpoints keep
// their original bits, including across fan and polygon edge seams.
fn closed_lerp(a: f64, b: f64, numerator: f64, denominator: f64) -> f64 {
    if numerator == 0. {
        a
    } else if numerator == denominator {
        b
    } else {
        (a + (b - a) * (numerator / denominator)).clamp(a.min(b), a.max(b))
    }
}
fn corner_key(corner: usize, height: usize) -> Key {
    [0, corner, height, 0, 0]
}
fn edge_key(start: usize, end: usize, k: usize, n: usize, height: usize) -> Key {
    if k == 0 {
        corner_key(start, height)
    } else if k == n {
        corner_key(end, height)
    } else if start < end {
        [1, start, end, k, height]
    } else {
        [1, end, start, n - k, height]
    }
}
fn edge_uv(points: &[[f64; 2]], start: usize, end: usize, k: usize, n: usize) -> [f64; 2] {
    let (a, b, step) = if start < end {
        (start, end, k)
    } else {
        (end, start, n - k)
    };
    std::array::from_fn(|axis| closed_lerp(points[a][axis], points[b][axis], step as f64, n as f64))
}
#[allow(clippy::too_many_arguments)]
fn emit(
    out: &mut NurbsGraphMesh,
    cache: &mut Cache,
    face: &Face,
    fi: usize,
    vertices: [(Key, [f64; 2]); 3],
    bound: f64,
    arithmetic: f64,
) -> Result<()> {
    let Surface::Nurbs(surface) = &face.surface else {
        return Err(Error::InvalidTopology(
            "graph polygon opening display requires NURBS",
        ));
    };
    let mut triangle = [0; 3];
    for (k, (key, uv)) in vertices.into_iter().enumerate() {
        if let Some(&v) = cache.display.get(&(fi, key)) {
            triangle[k] = v;
            continue;
        }
        let evaluation = surface.evaluate_with_partials(uv[0], uv[1], [KnotSide::Right; 2])?;
        let next = cache.nodes.len();
        let (node, position) = *cache.nodes.entry(key).or_insert((next, evaluation.point));
        let gap = (position - evaluation.point).norm();
        if !gap.is_finite() || gap > arithmetic {
            return Err(Error::Tessellation(
                "graph polygon opening shared evaluations disagree",
            ));
        }
        let index = out.mesh.positions.len();
        cache.display.insert((fi, key), index);
        triangle[k] = index;
        out.mesh.positions.push(position);
        out.mesh
            .normals
            .push(evaluation.normal()? * face.orientation as f64);
        out.vertex_nodes.push(node);
        out.vertex_uv.push(uv);
        out.vertex_faces.push(fi);
    }
    if face.orientation < 0 {
        triangle.swap(1, 2);
    }
    let normal = (out.mesh.positions[triangle[1]] - out.mesh.positions[triangle[0]])
        .cross(out.mesh.positions[triangle[2]] - out.mesh.positions[triangle[0]])
        .normalized()
        .map_err(|_| Error::Tessellation("graph polygon opening triangle area is unresolved"))?;
    if triangle
        .iter()
        .any(|&v| normal.dot(out.mesh.normals[v]) <= 0.)
    {
        return Err(Error::Tessellation(
            "graph polygon opening triangle orientation is unresolved",
        ));
    }
    out.mesh.triangles.push(triangle);
    out.mesh.face_ids.push(fi);
    out.error_bounds.push(bound);
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn inside(p: [f64; 2], polygon: &[[f64; 2]]) -> bool {
        (0..polygon.len()).all(|i| {
            let a = polygon[i];
            let b = polygon[(i + 1) % polygon.len()];
            (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]) > 1e-12
        })
    }
    // Clip a segment against the strict interior halfplanes of the convex
    // opening; sampling alone would miss a narrow crossing between samples.
    fn enters_opening(a: [f64; 2], b: [f64; 2], polygon: &[[f64; 2]]) -> bool {
        let mut lo: f64 = 0.;
        let mut hi: f64 = 1.;
        for i in 0..polygon.len() {
            let p = polygon[i];
            let q = polygon[(i + 1) % polygon.len()];
            let cross =
                |x: [f64; 2]| (q[0] - p[0]) * (x[1] - p[1]) - (q[1] - p[1]) * (x[0] - p[0]) - 1e-12;
            let start = cross(a);
            let slope = cross(b) - start;
            if slope == 0. {
                if start <= 0. {
                    return false;
                }
            } else if slope > 0. {
                lo = lo.max(-start / slope);
            } else {
                hi = hi.min(-start / slope);
            }
        }
        lo < hi && hi > 0. && lo < 1.
    }
    fn check(body: &NurbsGraphPolygonMultiHoledSolid, error: f64) -> Result<NurbsGraphMesh> {
        let display = body.tessellate_bounded(error, 65536, Tolerance::default())?;
        let mut edges = BTreeMap::<(usize, usize), (usize, i32)>::new();
        let mut positions = BTreeMap::new();
        for (i, &node) in display.vertex_nodes.iter().enumerate() {
            if let Some(previous) = positions.insert(node, display.mesh.positions[i]) {
                assert_eq!(previous, display.mesh.positions[i]);
            }
            let fi = display.vertex_faces[i];
            let Surface::Nurbs(surface) = &body.brep().shell.faces[fi].surface else {
                unreachable!()
            };
            if fi >= 2 + body.polygon().len() {
                let mut edge = fi - 2 - body.polygon().len();
                let polygon = body
                    .holes()
                    .iter()
                    .find(|p| {
                        if edge < p.len() {
                            true
                        } else {
                            edge -= p.len();
                            false
                        }
                    })
                    .unwrap();
                let a = polygon[edge];
                let b = polygon[(edge + 1) % polygon.len()];
                let dimensions = body.source().dimensions();
                let inward = body.source().placement().vector(Vec3::new(
                    -dimensions[1] * (b[1] - a[1]),
                    dimensions[0] * (b[0] - a[0]),
                    0.,
                ));
                assert!(display.mesh.normals[i].dot(inward) > 0.);
            }
            let uv = display.vertex_uv[i];
            let jet = surface.evaluate_with_partials(uv[0], uv[1], [KnotSide::Right; 2])?;
            assert!(
                (jet.point - display.mesh.positions[i]).norm()
                    <= body.source().arithmetic_budget()?
            );
            assert!(
                jet.normal()?.dot(display.mesh.normals[i])
                    * body.brep().shell.faces[fi].orientation as f64
                    > 0.999999
            );
        }
        for (ti, triangle) in display.mesh.triangles.iter().enumerate() {
            for k in 0..3 {
                let a = display.vertex_nodes[triangle[k]];
                let b = display.vertex_nodes[triangle[(k + 1) % 3]];
                let e = edges.entry((a.min(b), a.max(b))).or_default();
                e.0 += 1;
                e.1 += if a < b { 1 } else { -1 };
            }
            let fi = display.mesh.face_ids[ti];
            let Surface::Nurbs(surface) = &body.brep().shell.faces[fi].surface else {
                unreachable!()
            };
            if fi < 2 {
                let uv = triangle.map(|v| display.vertex_uv[v]);
                let center = std::array::from_fn(|axis| uv.iter().map(|p| p[axis] / 3.).sum());
                for hole in body.holes() {
                    assert!(!inside(center, hole));
                    // An entire opening inside one triangle would evade edge
                    // crossing and centroid tests. Check its corners as well.
                    for &point in hole {
                        let signs = std::array::from_fn::<_, 3, _>(|k| {
                            let a = uv[k];
                            let b = uv[(k + 1) % 3];
                            (b[0] - a[0]) * (point[1] - a[1]) - (b[1] - a[1]) * (point[0] - a[0])
                        });
                        assert!(
                            !(signs.iter().all(|v| *v > 1e-12)
                                || signs.iter().all(|v| *v < -1e-12))
                        );
                    }
                }
                for k in 0..3 {
                    for hole in body.holes() {
                        assert!(!enters_opening(uv[k], uv[(k + 1) % 3], hole));
                    }
                }
            }
            for a in 0..=4 {
                for b in 0..=4 - a {
                    let w = [a as f64 / 4., b as f64 / 4., (4 - a - b) as f64 / 4.];
                    let uv: [f64; 2] = std::array::from_fn(|axis| {
                        (0..3)
                            .map(|k| w[k] * display.vertex_uv[triangle[k]][axis])
                            .sum()
                    });
                    if fi < 2 {
                        for hole in body.holes() {
                            assert!(!inside(uv, hole), "cap sample enters opening");
                        }
                    }
                    let p = display.mesh.positions[triangle[0]];
                    let approximate = p
                        + (display.mesh.positions[triangle[1]] - p) * w[1]
                        + (display.mesh.positions[triangle[2]] - p) * w[2];
                    assert!(
                        (surface.evaluate(uv[0], uv[1])? - approximate).norm()
                            <= display.error_bounds[ti]
                    );
                }
            }
        }
        assert!(edges.values().all(|v| *v == (2, 0)));
        assert_eq!(
            positions.len() as isize - edges.len() as isize + display.mesh.triangles.len() as isize,
            2 - 2 * body.holes().len() as isize
        );
        assert!(display.mesh.signed_volume() > 0.);
        Ok(display)
    }
    fn rectangle(u: [f64; 2], v: [f64; 2]) -> Vec<[f64; 2]> {
        vec![[u[0], v[0]], [u[1], v[0]], [u[1], v[1]], [u[0], v[1]]]
    }
    #[test]
    fn multiple_openings_closed_genus_signed_curvature_trim_and_rigid() -> Result<()> {
        let tol = Tolerance::default();
        for count in 1..=4 {
            let holes = [
                rectangle([0.2, 0.35], [0.2, 0.35]),
                rectangle([0.55, 0.7], [0.2, 0.35]),
                rectangle([0.2, 0.35], [0.55, 0.7]),
                rectangle([0.55, 0.7], [0.55, 0.7]),
            ];
            for bulge in [-1., 0., 1.] {
                let transform = Transform::translation(Vec3::new(40., -30., 10.))?
                    .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.6)?)?;
                let source = NurbsGraphSolid::new([3., 2., 4.], bulge, tol)?
                    .trimmed_uv([[0.05, 0.95], [0.05, 0.95]], tol)?
                    .transformed(transform, tol)?;
                let outer =
                    NurbsGraphPolygonSolid::new(&source, rectangle([0.1, 0.9], [0.1, 0.9]), tol)?;
                let body =
                    NurbsGraphPolygonMultiHoledSolid::new(&outer, holes[..count].to_vec(), tol)?;
                let display = check(&body, 0.05)?;
                let primitive = |x: f64| x * x / 2. - x * x * x / 3.;
                let volume = |u: [f64; 2], v: [f64; 2]| {
                    6. * (4. * (u[1] - u[0]) * (v[1] - v[0])
                        + 4. * bulge
                            * (primitive(u[1]) - primitive(u[0]))
                            * (primitive(v[1]) - primitive(v[0])))
                };
                let exact = volume([0.1, 0.9], [0.1, 0.9])
                    - holes[..count]
                        .iter()
                        .map(|p| volume([p[0][0], p[1][0]], [p[0][1], p[2][1]]))
                        .sum::<f64>();
                assert!((display.mesh.signed_volume() - exact).abs() < 0.005);
                let corners = 4 + 4 * count;
                assert_eq!(
                    display.mesh.triangles.len(),
                    4 * (corners + count - 1) * display.subdivisions.pow(2)
                );
                assert!(body
                    .tessellate_bounded(1., 4 * (corners + count - 1) - 1, tol)
                    .is_err());
                if bulge != 0. {
                    assert!(body
                        .tessellate_bounded(0.00001, 4 * (corners + count - 1), tol)
                        .is_err());
                }
                assert!(body
                    .tessellate_bounded(source.arithmetic_budget()?, 65536, tol)
                    .is_err());
            }
        }
        Ok(())
    }
}
