//! Conforming display sampling of the actual convex polygon graph B-rep.
use crate::*;
use std::collections::BTreeMap;
type Key = [usize; 5];
#[derive(Default)]
struct Cache {
    nodes: BTreeMap<Key, (usize, Point3)>,
    display: BTreeMap<(usize, Key), usize>,
}
impl NurbsGraphPolygonSolid {
    /// Uniform conforming fan and wall grids. Bounds include an engineering
    /// floating-point allowance; they are not interval arithmetic certificates.
    pub fn tessellate_bounded(
        &self,
        error: f64,
        max_triangles: usize,
        tol: Tolerance,
    ) -> Result<NurbsGraphMesh> {
        self.validate(tol)?;
        let corners = self.polygon().len();
        if !error.is_finite() || error <= 0. || !(4 * corners..=65536).contains(&max_triangles) {
            return Err(Error::InvalidInput(
                "graph polygon error or triangle budget is invalid",
            ));
        }
        let arithmetic = self.source().arithmetic_budget()?;
        if !arithmetic.is_finite() || arithmetic >= error / 4. {
            return Err(Error::Tessellation(
                "graph polygon precision cannot resolve error",
            ));
        }
        // Affine UV restriction can perturb unit weights through roundoff.
        // A rational convex combination differs from its unit-weight version
        // by at most 2*delta/(1-delta) times the control-hull diameter.
        // Reserve this contribution inside the source engineering allowance.
        for face in &self.brep().shell.faces {
            let Surface::Nurbs(surface) = &face.surface else {
                return Err(Error::InvalidTopology(
                    "graph polygon display requires NURBS",
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
                    "graph polygon rational perturbation exceeds arithmetic allowance",
                ));
            }
        }
        // Source roof Hessian norm <=6|b|. Fan triangle UV diameter <=sqrt(2)/N.
        // On a ruled wall |h_tt|<=12|b| and |h_t|<=2|b|; its height
        // fraction contributes a mixed derivative. 14|b|/N² bounds both.
        let curvature = 14. * self.source().bulge().abs();
        let limit = ((max_triangles / (4 * corners)) as f64).sqrt().floor() as usize;
        let mut n = 1;
        while curvature / (n * n) as f64 + arithmetic > error {
            if n >= limit {
                return Err(Error::Tessellation(
                    "graph polygon triangle budget exceeded",
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
        let polygon = self.polygon();
        let center = std::array::from_fn::<_, 2, _>(|axis| {
            polygon
                .iter()
                .enumerate()
                .skip(1)
                .fold(polygon[0][axis], |mean, (i, p)| {
                    closed_lerp(mean, p[axis], 1., (i + 1) as f64)
                })
        });
        for cap in 0..2 {
            for edge in 0..corners {
                let next = (edge + 1) % corners;
                let vertex = |a: usize, b: usize| {
                    let key = if a + b == n {
                        boundary_key(edge, b, cap * n, n, corners)
                    } else if a == 0 && b == 0 {
                        [1, 0, 0, cap, 0]
                    } else if b == 0 {
                        [1, edge, a, cap, 0]
                    } else if a == 0 {
                        [1, next, b, cap, 0]
                    } else {
                        [2, edge, a, b, cap]
                    };
                    // Evaluate endpoints exactly and the fan as nested convex
                    // combinations. Multiplying a difference by the integer
                    // numerator before division can overshoot a domain endpoint.
                    let uv = std::array::from_fn(|axis| {
                        let radial = a + b;
                        if radial == 0 {
                            center[axis]
                        } else {
                            let rim = closed_lerp(
                                polygon[edge][axis],
                                polygon[next][axis],
                                b as f64,
                                radial as f64,
                            );
                            closed_lerp(center[axis], rim, radial as f64, n as f64)
                        }
                    });
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
        for edge in 0..corners {
            let fi = edge + 2;
            let vertex = |i, j| {
                (
                    boundary_key(edge, i, j, n, corners),
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
fn boundary_key(edge: usize, k: usize, height: usize, n: usize, corners: usize) -> Key {
    if k == n {
        [0, (edge + 1) % corners, 0, height, 0]
    } else {
        [0, edge, k, height, 0]
    }
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
            "graph polygon display requires NURBS",
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
                "graph polygon shared evaluations disagree",
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
        .normalized()?;
    if triangle
        .iter()
        .any(|&v| normal.dot(out.mesh.normals[v]) <= 0.)
    {
        return Err(Error::Tessellation(
            "graph polygon triangle orientation is unresolved",
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
    #[test]
    fn conforming_actual_faces_and_signed_volume() -> Result<()> {
        let tol = Tolerance::default();
        for polygon in [
            vec![[0.1, 0.1], [0.9, 0.2], [0.3, 0.9]],
            vec![[0.1, 0.1], [0.9, 0.1], [0.9, 0.9], [0.1, 0.9]],
            vec![[0.2, 0.1], [0.8, 0.1], [0.9, 0.6], [0.5, 0.9], [0.1, 0.6]],
        ] {
            for bulge in [-1., 0., 1.] {
                let source = NurbsGraphSolid::new([3., 2., 4.], bulge, tol)?;
                let body = NurbsGraphPolygonSolid::new(&source, polygon.clone(), tol)?;
                let display = body.tessellate_bounded(0.01, 65536, tol)?;
                assert_eq!(
                    display.mesh.triangles.len(),
                    4 * polygon.len() * display.subdivisions.pow(2)
                );
                let mut edges = BTreeMap::<(usize, usize), (usize, i32)>::new();
                let mut nodes = BTreeMap::new();
                for (v, &node) in display.vertex_nodes.iter().enumerate() {
                    let point = display.mesh.positions[v];
                    if let Some(previous) = nodes.insert(node, point) {
                        assert_eq!(previous, point);
                    }
                    let fi = display.vertex_faces[v];
                    let Surface::Nurbs(surface) = &body.brep().shell.faces[fi].surface else {
                        unreachable!()
                    };
                    let uv = display.vertex_uv[v];
                    assert!(
                        (surface.evaluate(uv[0], uv[1])? - point).norm()
                            <= source.arithmetic_budget()?
                    );
                }
                for (ti, triangle) in display.mesh.triangles.iter().enumerate() {
                    for k in 0..3 {
                        let a = display.vertex_nodes[triangle[k]];
                        let b = display.vertex_nodes[triangle[(k + 1) % 3]];
                        let entry = edges.entry((a.min(b), a.max(b))).or_default();
                        entry.0 += 1;
                        entry.1 += if a < b { 1 } else { -1 };
                    }
                    // Corresponding UV interpolation samples check actual surface error.
                    let fi = display.mesh.face_ids[ti];
                    let Surface::Nurbs(surface) = &body.brep().shell.faces[fi].surface else {
                        unreachable!()
                    };
                    for a in 0..=6 {
                        for b in 0..=6 - a {
                            let weights = [a as f64 / 6., b as f64 / 6., (6 - a - b) as f64 / 6.];
                            let uv: [f64; 2] = std::array::from_fn(|axis| {
                                (0..3)
                                    .map(|k| weights[k] * display.vertex_uv[triangle[k]][axis])
                                    .sum::<f64>()
                            });
                            let p = display.mesh.positions[triangle[0]];
                            let approximate = p
                                + (display.mesh.positions[triangle[1]] - p) * weights[1]
                                + (display.mesh.positions[triangle[2]] - p) * weights[2];
                            assert!(
                                (surface.evaluate(uv[0], uv[1])? - approximate).norm()
                                    <= display.error_bounds[ti]
                            );
                        }
                    }
                }
                assert!(edges.values().all(|v| *v == (2, 0)));
                assert!(display.mesh.signed_volume() > 0.);
                if bulge == 0. {
                    let area = (0..polygon.len())
                        .map(|i| {
                            let a = polygon[i];
                            let b = polygon[(i + 1) % polygon.len()];
                            a[0] * b[1] - a[1] * b[0]
                        })
                        .sum::<f64>()
                        / 2.;
                    assert!((display.mesh.signed_volume() - 24. * area).abs() < 1e-10);
                }
            }
        }
        Ok(())
    }
    #[test]
    fn trimmed_rigid_curved_volume_matches_independent_rectangle() -> Result<()> {
        let tol = Tolerance::default();
        let transform = Transform::translation(Vec3::new(40., -30., 10.))?
            .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.6)?)?;
        let source = NurbsGraphSolid::new([3., 2., 4.], -1., tol)?
            .trimmed_uv([[0.1, 0.9], [0.1, 0.9]], tol)?
            .transformed(transform, tol)?;
        let body = NurbsGraphPolygonSolid::new(
            &source,
            vec![[0.2, 0.2], [0.8, 0.2], [0.8, 0.8], [0.2, 0.8]],
            tol,
        )?;
        let primitive = |x: f64| x * x / 2. - x * x * x / 3.;
        let integral = primitive(0.8) - primitive(0.2);
        let exact = 6. * (4. * 0.6 * 0.6 - 4. * integral * integral);
        let coarse = body.tessellate_bounded(0.1, 65536, tol)?;
        let fine = body.tessellate_bounded(0.01, 65536, tol)?;
        assert!(
            (fine.mesh.signed_volume() - exact).abs() < (coarse.mesh.signed_volume() - exact).abs()
        );
        assert!((fine.mesh.signed_volume() - exact).abs() < 0.002);
        Ok(())
    }
    #[test]
    fn oblique_split_domain_endpoints_remain_closed() -> Result<()> {
        let tol = Tolerance::default();
        let source = NurbsGraphSolid::new([20., 12., 3.], 20., tol)?;
        let split = source.split_uv_line([0., 0.8], [0.8, 0.], tol)?;
        for body in [&split.negative, &split.positive] {
            let display = body.tessellate_bounded(0.5, 65536, tol)?;
            assert!(display.mesh.signed_volume() > 0.);
            for (i, uv) in display.vertex_uv.iter().enumerate() {
                let Surface::Nurbs(surface) =
                    &body.brep().shell.faces[display.vertex_faces[i]].surface
                else {
                    unreachable!()
                };
                let domain = surface.domain();
                for axis in 0..2 {
                    assert!(domain[axis][0] <= uv[axis] && uv[axis] <= domain[axis][1]);
                }
                assert!(
                    (surface.evaluate(uv[0], uv[1])? - display.mesh.positions[i]).norm()
                        <= source.arithmetic_budget()?
                );
            }
        }
        Ok(())
    }
    #[test]
    fn budgets_and_unresolved_errors_are_explicit() -> Result<()> {
        let tol = Tolerance::default();
        let source = NurbsGraphSolid::new([3., 2., 4.], 1., tol)?;
        let body =
            NurbsGraphPolygonSolid::new(&source, vec![[0.1, 0.1], [0.9, 0.2], [0.3, 0.9]], tol)?;
        assert!(body.tessellate_bounded(0.01, 12, tol).is_err());
        assert!(body.tessellate_bounded(1., 11, tol).is_err());
        assert!(body.tessellate_bounded(f64::NAN, 65536, tol).is_err());
        assert!(body
            .tessellate_bounded(source.arithmetic_budget()?, 65536, tol)
            .is_err());
        Ok(())
    }
}
