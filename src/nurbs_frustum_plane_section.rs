//! Exact rational closed sections strictly between the retained frustum caps.
use crate::*;
#[derive(Clone, Debug)]
pub struct FrustumPlaneSectionUse {
    pub face_id: usize,
    pub pcurve: PCurve,
}
#[derive(Clone, Debug)]
pub struct NurbsFrustumPlaneSection {
    source: NurbsFrustumSolid,
    plane: Surface,
    vertices: [Vertex; 4],
    edges: [Edge; 4],
    uses: [FrustumPlaneSectionUse; 4],
}
fn unresolved() -> Error {
    Error::Unsupported(
        "frustum plane section denominator, cap clearance or coefficient precision is unresolved",
    )
}
fn norm(v: Vec3) -> f64 {
    v.x.hypot(v.y).hypot(v.z)
}
impl NurbsFrustumPlaneSection {
    pub fn vertices(&self) -> &[Vertex; 4] {
        &self.vertices
    }
    pub fn edges(&self) -> &[Edge; 4] {
        &self.edges
    }
    pub fn uses(&self) -> &[FrustumPlaneSectionUse; 4] {
        &self.uses
    }
    pub fn plane(&self) -> &Surface {
        &self.plane
    }
    /// Validate the complete deterministic coefficient certificate and actual
    /// source attachments. Traversal follows increasing source quarter U;
    /// it is independent of the caller's plane-normal orientation.
    pub fn validate(&self, policy: GeometryTolerance) -> Result<()> {
        let expected = construct(&self.source, &self.plane, policy)?;
        for i in 0..4 {
            if self.vertices[i].point != expected.vertices[i].point
                || self.edges[i].vertices != expected.edges[i].vertices
                || !crate::nurbs_graph_step_import::same_curve(
                    &self.edges[i].curve,
                    &expected.edges[i].curve,
                )
                || self.uses[i].face_id != expected.uses[i].face_id
            {
                return Err(Error::InvalidTopology(
                    "plane section canonical coefficient or topology certificate changed",
                ));
            }
            let (PCurve::Nurbs(a), PCurve::Nurbs(b)) =
                (&self.uses[i].pcurve, &expected.uses[i].pcurve)
            else {
                return Err(unresolved());
            };
            if a.degree() != b.degree()
                || a.knots() != b.knots()
                || a.weights() != b.weights()
                || a.control_points() != b.control_points()
            {
                return Err(Error::InvalidTopology(
                    "plane section source pcurve coefficient certificate changed",
                ));
            }
        }
        Ok(())
    }
}
impl NurbsFrustumSolid {
    /// Closed four-quarter exact rational section, strictly between both caps.
    /// This returns a spatial boundary with source-face UV provenance, not a
    /// solid partition. Increasing source U fixes traversal for either plane normal.
    pub fn section_by_plane(
        &self,
        plane: &Surface,
        policy: GeometryTolerance,
    ) -> Result<NurbsFrustumPlaneSection> {
        construct(self, plane, policy)
    }
}
fn construct(
    source: &NurbsFrustumSolid,
    plane: &Surface,
    policy: GeometryTolerance,
) -> Result<NurbsFrustumPlaneSection> {
    source.validate(policy)?;
    let Surface::Plane { origin, u, v } = *plane else {
        return Err(Error::Unsupported("frustum section requires a plane"));
    };
    if !origin.finite() || !crate::geometry::plane_basis_valid(u, v, policy) {
        return Err(Error::InvalidInput(
            "section plane requires a finite origin and resolved orthonormal basis",
        ));
    }
    let world_normal = u.cross(v).normalized()?;
    let frame = source.frame();
    let n = frame.local_vector(world_normal);
    let delta = origin - frame.origin();
    let distance = norm(delta);
    let [r0, r1] = source.radii();
    let height = source.height();
    let scale = r0.max(r1).max(height);
    let radius0 = r0 / scale;
    let dr = (r1 - r0) / scale;
    let h = height / scale;
    let d = world_normal.dot(delta) / scale;
    let world = scale
        .max(origin.x.abs())
        .max(origin.y.abs())
        .max(origin.z.abs())
        .max(frame.origin().x.abs())
        .max(frame.origin().y.abs())
        .max(frame.origin().z.abs());
    if !delta.finite() || !distance.is_finite() || !d.is_finite() {
        return Err(unresolved());
    }
    let band = policy.length_at_scale((2. * r0.max(r1)).hypot(height))?;
    let dirs = [[1., 0.], [0., 1.], [-1., 0.], [0., -1.]];
    let weights = [1., 0.5f64.sqrt(), 1.];
    let k = radius0 * n.z * h + dr * d;
    if !k.is_normal() {
        return Err(unresolved());
    }
    let mut curves = vec![];
    let mut pcurves = vec![];
    for q in 0..4 {
        let next = dirs[(q + 1) % 4];
        let points = [dirs[q], [dirs[q][0] + next[0], dirs[q][1] + next[1]], next];
        let a: [f64; 3] = std::array::from_fn(|i| points[i][0] * weights[i]);
        let b: [f64; 3] = std::array::from_fn(|i| points[i][1] * weights[i]);
        let g: [f64; 3] = std::array::from_fn(|i| n.x * a[i] + n.y * b[i]);
        let raw_den: [f64; 3] = std::array::from_fn(|i| n.z * h * weights[i] + dr * g[i]);
        let raw_num: [f64; 3] = std::array::from_fn(|i| d * weights[i] - radius0 * g[i]);
        let sign = if raw_den.iter().all(|x| *x > 0.) {
            1.
        } else if raw_den.iter().all(|x| *x < 0.) {
            -1.
        } else {
            return Err(unresolved());
        };
        let den = raw_den.map(|x| x * sign);
        let num = raw_num.map(|x| x * sign);
        let min = den.iter().copied().fold(f64::INFINITY, f64::min);
        let max = den.iter().copied().fold(0f64, f64::max);
        let condition = max / min;
        let arithmetic =
            32768. * f64::EPSILON * world * (1. + condition + 1. / min) * (1. + distance / scale);
        if !min.is_normal() || !arithmetic.is_finite() || arithmetic >= policy.linear() / 8. {
            return Err(unresolved());
        }
        let mut controls = vec![];
        for i in 0..3 {
            let identity = radius0 * raw_den[i] + dr * raw_num[i] - k * weights[i];
            if !identity.is_finite() || identity.abs() * scale / min > policy.linear() / 16. {
                return Err(unresolved());
            }
            let local = Point3::new(
                scale * (k * a[i] / raw_den[i]),
                scale * (k * b[i] / raw_den[i]),
                height * (num[i] / den[i]),
            );
            let world_point = frame.point(local);
            // A plane equation is affine: checking every rational control
            // coefficient proves the entire positive-weight spatial curve lies
            // in its caller plane within this engineering arithmetic reserve.
            if world_normal.dot(world_point - origin).abs() + arithmetic > policy.linear() / 8. {
                return Err(unresolved());
            }
            if !world_point.finite() || !local.finite() {
                return Err(unresolved());
            }
            controls.push(world_point);
        }
        let curve = NurbsCurve::new(2, vec![0., 0., 0., 1., 1., 1.], controls, den.to_vec())?;
        let mut uv = vec![];
        let mut uv_weights = vec![];
        for i in 0..4 {
            let f = i as f64 / 3.;
            let lower = if i > 0 { f * den[i - 1] } else { 0. };
            let upper = if i < 3 { (1. - f) * den[i] } else { 0. };
            let w = lower + upper;
            let vn = (if i > 0 { f * num[i - 1] } else { 0. })
                + (if i < 3 { (1. - f) * num[i] } else { 0. });
            let vv = vn / w;
            if !w.is_normal()
                || !vv.is_finite()
                || height * vv <= 10. * band + arithmetic
                || height * (1. - vv) <= 10. * band + arithmetic
            {
                return Err(unresolved());
            }
            uv.push(Point3::new(lower / w, vv, 0.));
            uv_weights.push(w);
        }
        let pcurve = PCurve::nurbs(NurbsCurve::new(
            3,
            vec![0., 0., 0., 0., 1., 1., 1., 1.],
            uv,
            uv_weights,
        )?)?;
        for t in [0., 0.125, 0.5, 0.875, 1.] {
            let point = curve.evaluate(t)?;
            let params = pcurve.try_evaluate(t)?;
            let actual = source.solid().shell.faces[2 + q]
                .surface
                .try_evaluate(params[0], params[1])?;
            if norm(actual - point) + arithmetic > policy.linear() / 4.
                || world_normal.dot(point - origin).abs() + arithmetic > policy.linear() / 4.
            {
                return Err(unresolved());
            }
        }
        curves.push(curve);
        pcurves.push(pcurve);
    }
    let vertices: [Vertex; 4] = std::array::from_fn(|q| Vertex {
        point: curves[q].control_points()[0],
    });
    for q in 0..4 {
        if curves[q].control_points()[2] != vertices[(q + 1) % 4].point {
            return Err(Error::InvalidTopology(
                "section quarter endpoints do not share exact vertices",
            ));
        }
    }
    let edges = std::array::from_fn(|q| Edge {
        vertices: [q, (q + 1) % 4],
        curve: Curve::Nurbs(Box::new(curves[q].clone())),
    });
    let uses = std::array::from_fn(|q| FrustumPlaneSectionUse {
        face_id: 2 + q,
        pcurve: pcurves[q].clone(),
    });
    Ok(NurbsFrustumPlaneSection {
        source: source.clone(),
        plane: plane.clone(),
        vertices,
        edges,
        uses,
    })
}
