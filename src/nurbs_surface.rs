//! Tensor-product rational B-spline surfaces with exact analytic first partials.
use crate::nurbs::{
    de_boor, homogeneous_jet, is_c0, knot_span, project, rational_partial, validate_axis,
    weighted_controls,
};
use crate::{Error, KnotSide, Point3, Result, Vec3};

/// Untrimmed, clamped, nonperiodic, positive-weight NURBS surface.
/// Controls use U-major order: `control_points[u * count_v + v]`.
/// This standalone geometry is not yet a B-rep face or a solid constructor.
///
/// ```
/// use hagane::{NurbsSurface, Point3, Vec3};
/// let knots = vec![0.0, 0.0, 1.0, 1.0];
/// let plane = NurbsSurface::new([1, 1], [knots.clone(), knots], [2, 2],
///     vec![Point3::new(0.0, 0.0, 0.0), Point3::new(0.0, 1.0, 0.0),
///          Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 1.0, 0.0)],
///     vec![1.0; 4])?;
/// assert_eq!(plane.evaluate(0.5, 0.5)?, Point3::new(0.5, 0.5, 0.0));
/// assert_eq!(plane.normal(0.5, 0.5)?, Vec3::new(0.0, 0.0, 1.0));
/// # Ok::<(), hagane::Error>(())
/// ```
#[derive(Clone, Debug)]
pub struct NurbsSurface {
    degrees: [usize; 2],
    knots: [Vec<f64>; 2],
    counts: [usize; 2],
    points: Vec<Point3>,
    weights: Vec<f64>,
    homogeneous: Vec<[f64; 4]>,
}
#[derive(Clone, Copy, Debug)]
pub struct SurfaceEvaluation {
    pub point: Point3,
    pub du: Vec3,
    pub dv: Vec3,
}
impl SurfaceEvaluation {
    /// Oriented unit normal `du × dv`. Degenerate/nearly parallel tangents
    /// are errors; neither an arbitrary vector nor a zero normal is substituted.
    pub fn normal(&self) -> Result<Vec3> {
        let scale = |v: Vec3| -> Result<Vec3> {
            let m = v.x.abs().max(v.y.abs()).max(v.z.abs());
            if !v.finite() || m == 0.0 {
                return Err(Error::InvalidInput("degenerate NURBS tangent plane"));
            }
            Ok(Vec3::new(v.x / m, v.y / m, v.z / m))
        };
        let a = scale(self.du)?;
        let b = scale(self.dv)?;
        let c = a.cross(b);
        let norm = c.x.hypot(c.y).hypot(c.z);
        if norm <= 64.0 * f64::EPSILON * a.norm() * b.norm() {
            return Err(Error::InvalidInput(
                "degenerate or numerically singular NURBS tangent plane",
            ));
        }
        Ok(Vec3::new(c.x / norm, c.y / norm, c.z / norm))
    }
}
impl NurbsSurface {
    pub fn new(
        degrees: [usize; 2],
        knots: [Vec<f64>; 2],
        counts: [usize; 2],
        points: Vec<Point3>,
        weights: Vec<f64>,
    ) -> Result<Self> {
        let count = counts[0]
            .checked_mul(counts[1])
            .ok_or(Error::InvalidInput("NURBS surface control count overflows"))?;
        if count > 65536 || count != points.len() {
            return Err(Error::InvalidInput(
                "NURBS surface requires a rectangular control grid of at most 65536 points",
            ));
        }
        for axis in 0..2 {
            validate_axis(degrees[axis], &knots[axis], counts[axis])?;
        }
        let homogeneous = weighted_controls(&points, &weights)?;
        Ok(Self {
            degrees,
            knots,
            counts,
            points,
            weights,
            homogeneous,
        })
    }
    pub fn degrees(&self) -> [usize; 2] {
        self.degrees
    }
    pub fn knots(&self, axis: usize) -> Result<&[f64]> {
        self.knots
            .get(axis)
            .map(Vec::as_slice)
            .ok_or(Error::InvalidInput("surface axis must be 0 or 1"))
    }
    pub fn control_counts(&self) -> [usize; 2] {
        self.counts
    }
    pub fn control_points(&self) -> &[Point3] {
        &self.points
    }
    pub fn weights(&self) -> &[f64] {
        &self.weights
    }
    pub fn domain(&self) -> [[f64; 2]; 2] {
        std::array::from_fn(|i| {
            [
                self.knots[i][self.degrees[i]],
                self.knots[i][self.counts[i]],
            ]
        })
    }
    fn spans(&self, uv: [f64; 2], sides: [KnotSide; 2]) -> Result<[usize; 2]> {
        Ok([
            knot_span(
                self.degrees[0],
                &self.knots[0],
                self.counts[0],
                uv[0],
                sides[0],
            )?,
            knot_span(
                self.degrees[1],
                &self.knots[1],
                self.counts[1],
                uv[1],
                sides[1],
            )?,
        ])
    }
    pub fn evaluate(&self, u: f64, v: f64) -> Result<Point3> {
        let spans = self.spans([u, v], [KnotSide::Right; 2])?;
        let [p, q] = self.degrees;
        let [a, b] = spans;
        let mut rows = Vec::with_capacity(p + 1);
        for i in a - p..=a {
            let offset = i * self.counts[1];
            rows.push(de_boor(
                q,
                &self.knots[1],
                b,
                v,
                self.homogeneous[offset + b - q..=offset + b].to_vec(),
            )?);
        }
        project(de_boor(p, &self.knots[0], a, u, rows)?)
    }
    /// Checked ordinary first partials: potentially C0 knots on either axis
    /// require explicit side selection via `evaluate_with_partials`.
    pub fn partials(&self, u: f64, v: f64) -> Result<SurfaceEvaluation> {
        self.spans([u, v], [KnotSide::Right; 2])?;
        if is_c0(self.degrees[0], &self.knots[0], self.counts[0], u)
            || is_c0(self.degrees[1], &self.knots[1], self.counts[1], v)
        {
            return Err(Error::Unsupported(
                "surface partials at a C0 knot require explicit sides",
            ));
        }
        self.evaluate_with_partials(u, v, [KnotSide::Right; 2])
    }
    pub fn normal(&self, u: f64, v: f64) -> Result<Vec3> {
        self.partials(u, v)?.normal()
    }
    pub fn evaluate_with_partials(
        &self,
        u: f64,
        v: f64,
        sides: [KnotSide; 2],
    ) -> Result<SurfaceEvaluation> {
        let [a, b] = self.spans([u, v], sides)?;
        let [p, q] = self.degrees;
        let mut rows = Vec::with_capacity(p + 1);
        let mut v_derivatives = Vec::with_capacity(p + 1);
        for i in a - p..=a {
            let offset = i * self.counts[1];
            let (h, dv) = homogeneous_jet(
                q,
                &self.knots[1],
                b,
                v,
                self.homogeneous[offset + b - q..=offset + b].to_vec(),
            )?;
            rows.push(h);
            v_derivatives.push(dv);
        }
        let (h, du) = homogeneous_jet(p, &self.knots[0], a, u, rows)?;
        let dv = de_boor(p, &self.knots[0], a, u, v_derivatives)?;
        let point = project(h)?;
        Ok(SurfaceEvaluation {
            point,
            du: rational_partial(point, h, du)?,
            dv: rational_partial(point, h, dv)?,
        })
    }
}

impl NurbsSurface {
    /// Uniform-parameter display sampling of an untrimmed regular patch.
    /// This is not certified chord-tolerance tessellation. C0 interior knot
    /// lines require future patch splitting and are explicitly unsupported.
    pub fn sample_grid(&self, cells: [usize; 2]) -> Result<crate::Mesh> {
        if cells.iter().any(|n| *n == 0 || *n > 256) {
            return Err(Error::InvalidInput(
                "surface display grid requires 1..256 cells per axis",
            ));
        }
        for axis in 0..2 {
            let p = self.degrees[axis];
            let count = self.counts[axis];
            if self.knots[axis][p + 1..count]
                .iter()
                .any(|u| is_c0(p, &self.knots[axis], count, *u))
            {
                return Err(Error::Unsupported(
                    "C0 surface grid sampling requires knot-line splitting",
                ));
            }
        }
        let domain = self.domain();
        let [nu, nv] = cells;
        let mut evaluations = Vec::with_capacity((nu + 1) * (nv + 1));
        for i in 0..=nu {
            for j in 0..=nv {
                let u = if i == nu {
                    domain[0][1]
                } else {
                    domain[0][0] + (domain[0][1] - domain[0][0]) * i as f64 / nu as f64
                };
                let v = if j == nv {
                    domain[1][1]
                } else {
                    domain[1][0] + (domain[1][1] - domain[1][0]) * j as f64 / nv as f64
                };
                let e = self.partials(u, v)?;
                evaluations.push((e.point, e.normal()?));
            }
        }
        let mut mesh = crate::Mesh::default();
        for i in 0..nu {
            for j in 0..nv {
                let a = i * (nv + 1) + j;
                let b = (i + 1) * (nv + 1) + j;
                let c = b + 1;
                let d = a + 1;
                for tri in [[a, b, c], [a, c, d]] {
                    let start = mesh.positions.len();
                    let positions = tri.map(|k| evaluations[k].0);
                    let cross = (positions[1] - positions[0]).cross(positions[2] - positions[0]);
                    if !cross.finite() || cross.x.hypot(cross.y).hypot(cross.z) == 0.0 {
                        return Err(Error::Tessellation("degenerate sampled surface triangle"));
                    }
                    if tri.iter().any(|k| {
                        let alignment = cross.dot(evaluations[*k].1);
                        !alignment.is_finite() || alignment <= 0.0
                    }) {
                        return Err(Error::Tessellation(
                            "surface grid is too coarse or has inconsistent sampled orientation",
                        ));
                    }
                    mesh.positions.extend(positions);
                    mesh.normals.extend(tri.map(|k| evaluations[k].1));
                    mesh.triangles.push([start, start + 1, start + 2]);
                    mesh.face_ids.push(0);
                }
            }
        }
        Ok(mesh)
    }
}
/// Open rational quadratic patch, with a variable center control height/weight.
/// Output is a display grid plus exact selected point, partials and normal.
pub fn nurbs_surface_demo_json(height: f64, weight: f64, u: f64, v: f64) -> Result<String> {
    let surface = nurbs_surface_demo_geometry(height, weight)?;
    let e = surface.partials(u, v)?;
    let normal = e.normal()?;
    let brep = crate::NurbsFace::new(surface.clone(), 1, crate::Tolerance::default())?;
    let mesh = brep.sample_grid([24, 24], crate::Tolerance::default())?;
    let mut output=format!("{{\"height\":{height},\"weight\":{weight},\"u\":{u},\"v\":{v},\"point\":[{},{},{}],\"du\":[{},{},{}],\"dv\":[{},{},{}],\"normal\":[{},{},{}],\"positions\":[",e.point.x,e.point.y,e.point.z,e.du.x,e.du.y,e.du.z,e.dv.x,e.dv.y,e.dv.z,normal.x,normal.y,normal.z);
    for (i, p) in mesh.positions.iter().enumerate() {
        if i > 0 {
            output.push(',');
        }
        output.push_str(&format!("{},{},{}", p.x, p.y, p.z));
    }
    output.push_str("],\"normals\":[");
    for (i, n) in mesh.normals.iter().enumerate() {
        if i > 0 {
            output.push(',');
        }
        output.push_str(&format!("{},{},{}", n.x, n.y, n.z));
    }
    output.push_str("]}");
    let refined = surface.insert_knot(0, 0.35, 1)?.insert_knot(1, 0.7, 1)?;
    let boundaries = refined.boundary_edges()?;
    let mut sections = Vec::new();
    for (axis, parameter, boundary, forward) in [
        (1, 0.0, true, true),
        (0, 1.0, true, true),
        (1, 1.0, true, false),
        (0, 0.0, true, false),
        (0, u, false, true),
        (1, v, false, true),
    ] {
        let boundary_index = sections.len();
        let curve = if boundary {
            boundaries[boundary_index].curve.clone()
        } else {
            refined.isocurve(axis, parameter)?
        };
        let polyline = curve.tessellate_bounded(0.05, 16384)?;
        let samples: Vec<_> = polyline
            .points
            .iter()
            .flat_map(|p| [p.x, p.y, p.z])
            .collect();
        sections.push(serde_json::json!({
            "pcurve_origin": if axis == 0 { [parameter,0.] } else { [0.,parameter] },
            "pcurve_direction": if axis == 0 { [0.,1.] } else { [1.,0.] },
            "fixed_axis": axis, "fixed_parameter": parameter, "boundary": boundary,
            "forward": forward, "samples": samples, "parameters": polyline.parameters,
            "error_bounds": polyline.error_bounds, "degree": curve.degree(), "knots": curve.knots(),
            "control_points": curve.control_points().iter().map(|p| [p.x,p.y,p.z]).collect::<Vec<_>>(),
            "weights": curve.weights(), "parameter_range": curve.domain()
        }));
    }
    let mut json: serde_json::Value = serde_json::from_str(&output)
        .map_err(|_| Error::InvalidInput("surface demo serialization failed"))?;
    json["sections"] = serde_json::json!(sections);
    json["brep"] = serde_json::json!({
        "vertices": brep.vertices.iter().map(|v| [v.point.x,v.point.y,v.point.z]).collect::<Vec<_>>(),
        "edge_vertices": brep.edges.iter().map(|e| e.vertices).collect::<Vec<_>>(),
        "edge_ranges": brep.edges.iter().map(|e| e.curve.range()).collect::<Vec<_>>(),
        "coedge_edges": brep.face.wires[0].coedges.iter().map(|c| c.edge).collect::<Vec<_>>(),
        "coedge_forward": brep.face.wires[0].coedges.iter().map(|c| c.forward).collect::<Vec<_>>(),
        "orientation": brep.face.orientation, "faces": 1, "closed": false,
        "validation": "canonical rectangular boundary; global surface regularity is not certified"
    });
    json["refined_control_counts"] = serde_json::json!(refined.control_counts());
    json["section_chord_error"] = serde_json::json!(0.05);
    Ok(json.to_string())
}

pub(crate) fn nurbs_surface_demo_geometry(height: f64, weight: f64) -> Result<NurbsSurface> {
    if !height.is_finite() || height.abs() > 100.0 {
        return Err(Error::InvalidInput(
            "surface demo height must be finite and within [-100,100]",
        ));
    }
    let mut points = Vec::new();
    let mut weights = Vec::new();
    for i in 0..3 {
        for j in 0..3 {
            let center = i == 1 && j == 1;
            points.push(Point3::new(
                -40.0 + 40.0 * i as f64,
                -30.0 + 30.0 * j as f64,
                if center { height } else { 0.0 },
            ));
            weights.push(if center { weight } else { 1.0 });
        }
    }
    let knots = vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0];
    NurbsSurface::new([2, 2], [knots.clone(), knots], [3, 3], points, weights)
}
