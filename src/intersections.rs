//! Typed analytic intersections. These operate on surfaces, not trimmed faces.
use crate::{geometry::plane_basis_valid, *};
use std::f64::consts::TAU;

/// Infinite, unit-speed line with affine UV curves on both input planes.
/// Parameter `t` is signed length; it is not an edge's normalized parameter.
#[derive(Clone, Debug)]
pub struct PlaneIntersectionLine {
    pub origin: Point3,
    pub direction: Vec3,
    pub first: PCurve,
    pub second: PCurve,
}
impl PlaneIntersectionLine {
    pub fn evaluate(&self, parameter: f64) -> Result<Point3> {
        let point = self.origin + self.direction * parameter;
        if !parameter.is_finite() || !point.finite() {
            return Err(Error::InvalidInput(
                "intersection line parameter exceeds finite range",
            ));
        }
        Ok(point)
    }
}
#[derive(Clone, Debug)]
pub enum PlanePlaneIntersection {
    Line(PlaneIntersectionLine),
    /// Within the angular policy, with origins separated beyond the length budget.
    Parallel,
    /// Within both angular and distance budgets; not exact identity of planes.
    Coincident,
}
fn plane(surface: &Surface, tol: GeometryTolerance) -> Result<(Point3, Vec3, Vec3, Vec3)> {
    let Surface::Plane { origin, u, v } = *surface else {
        return Err(Error::Unsupported(
            "plane intersection requires planar surfaces",
        ));
    };
    if !origin.finite() || !plane_basis_valid(u, v, tol) {
        return Err(Error::InvalidInput(
            "invalid orthonormal intersection plane",
        ));
    }
    Ok((origin, u, v, u.cross(v).normalized()?))
}
/// Intersects infinite orthonormal planes. Direction follows n_first × n_second.
/// Anchor is the closest intersection point to the first plane's origin.
/// Relative distance scale uses origin separation; no face trims are applied.
pub fn intersect_plane_plane(
    first: &Surface,
    second: &Surface,
    tol: GeometryTolerance,
) -> Result<PlanePlaneIntersection> {
    let (a, u, v, n) = plane(first, tol)?;
    let (b, x, y, m) = plane(second, tol)?;
    let offset = b - a;
    let scale = offset.norm();
    let budget = tol.length_at_scale(scale)?;
    let cross = n.cross(m);
    let sine = cross.norm();
    if sine <= tol.angular().sin() {
        let distances = [offset.dot(n).abs(), offset.dot(m).abs()];
        if distances.iter().any(|d| !d.is_finite()) {
            return Err(Error::InvalidInput(
                "plane separation projection exceeds finite range",
            ));
        }
        return Ok(if distances.into_iter().all(|d| d <= budget) {
            PlanePlaneIntersection::Coincident
        } else {
            PlanePlaneIntersection::Parallel
        });
    }
    let direction = cross.normalized()?;
    let across = direction.cross(n).normalized()?;
    let travel = offset.dot(m) / across.dot(m);
    let origin = a + across * travel;
    let uv = |base: Point3, du: Vec3, dv: Vec3| PCurve::Affine {
        origin: [(origin - base).dot(du), (origin - base).dot(dv)],
        direction: [direction.dot(du), direction.dot(dv)],
    };
    let result = PlaneIntersectionLine {
        origin,
        direction,
        first: uv(a, u, v),
        second: uv(b, x, y),
    };
    // Validate an anchor and a local unit-length displacement on both surfaces.
    let validation_scale = scale.max(travel.abs()).max(tol.linear());
    for t in [0.0, validation_scale] {
        let point = result.evaluate(t)?;
        for (surface, pc) in [(first, &result.first), (second, &result.second)] {
            let uv = pc.evaluate(t);
            if uv.iter().any(|c| !c.is_finite())
                || !tol.coincident(point, surface.evaluate(uv[0], uv[1]), validation_scale)?
            {
                return Err(Error::InvalidInput(
                    "plane intersection loses surface/UV agreement",
                ));
            }
        }
    }
    Ok(PlanePlaneIntersection::Line(result))
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum IntersectionContact {
    Crossing,
    Tangent,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CylinderIntersectionPoint {
    pub point: Point3,
    /// Parameter on the caller's original (possibly non-unit) line direction.
    pub parameter: f64,
    /// Circular angle in [0,2pi), normal-span length in [0,height].
    pub uv: [f64; 2],
    pub contact: IntersectionContact,
}
#[derive(Clone, Debug, PartialEq)]
pub enum LineCylinderIntersection {
    Empty,
    /// Sorted by increasing line parameter. Only the lateral surface is intersected.
    Points(Vec<CylinderIntersectionPoint>),
    /// An exactly resolved axial/extrusion generator overlaps the lateral surface.
    Coincident {
        parameter_range: [f64; 2],
        angle: f64,
    },
}
fn cylinder_angle(x: f64, y: f64) -> f64 {
    let angle = y.atan2(x).rem_euclid(TAU);
    // Tiny negative angles can round to TAU during remainder addition.
    if angle >= TAU {
        0.0
    } else {
        angle
    }
}
pub(crate) fn line_parameter(travel: f64, direction: Vec3) -> Result<f64> {
    let scale = direction
        .x
        .abs()
        .max(direction.y.abs())
        .max(direction.z.abs());
    let norm = Vec3::new(
        direction.x / scale,
        direction.y / scale,
        direction.z / scale,
    )
    .norm();
    let t = (travel / scale) / norm;
    if !t.is_finite() || (travel != 0.0 && t == 0.0) {
        return Err(Error::InvalidInput(
            "line intersection parameter is unrepresentable",
        ));
    }
    Ok(t)
}
/// Intersects an infinite line with a bounded full cylinder's lateral surface.
/// Supports Z and framed cylinders, including oblique lines. No end disks or
/// angular face trims are included. Near tangencies and near-axial lines whose
/// topology cannot be resolved under the policy return errors instead of hits.
pub fn intersect_line_cylinder(
    anchor: Point3,
    direction: Vec3,
    cylinder: &Surface,
    tol: GeometryTolerance,
) -> Result<LineCylinderIntersection> {
    let (frame, radius, height) = match *cylinder {
        Surface::Cylinder {
            center,
            radius,
            height,
        } => (Frame3::translation(center)?, radius, height),
        Surface::FramedCylinder {
            frame,
            radius,
            height,
        } => (frame, radius, height),
        _ => {
            return Err(Error::Unsupported(
                "line-cylinder requires a cylinder surface",
            ))
        }
    };
    if !anchor.finite()
        || !radius.is_finite()
        || !height.is_finite()
        || radius <= 10.0 * tol.linear()
        || height <= 10.0 * tol.linear()
    {
        return Err(Error::InvalidInput("invalid line or cylinder dimensions"));
    }
    let unit = direction.normalized()?;
    let a = frame.local_point(anchor);
    let d = frame.local_vector(unit);
    if !a.finite() || !d.finite() {
        return Err(Error::InvalidInput(
            "cylinder local coordinates exceed finite range",
        ));
    }
    let radial_speed = d.x.hypot(d.y);
    let budget = tol.length_at_scale(radius)?;
    if radial_speed == 0.0 {
        let gap = a.x.hypot(a.y) - radius;
        if gap == 0.0 {
            let mut range = [
                line_parameter(-a.z / d.z, direction)?,
                line_parameter((height - a.z) / d.z, direction)?,
            ];
            range.sort_by(f64::total_cmp);
            if range[0] == range[1] {
                return Err(Error::InvalidInput("generator interval is unrepresentable"));
            }
            for (i, t) in range.into_iter().enumerate() {
                let p = anchor + direction * t;
                let q = frame.local_point(p);
                let expected_z = if (i == 0) == (d.z > 0.0) { 0.0 } else { height };
                if !p.finite()
                    || !q.finite()
                    || (q.x.hypot(q.y) - radius).abs() > budget
                    || (q.z - expected_z).abs() > tol.length_at_scale(height)?
                {
                    return Err(Error::InvalidInput(
                        "generator endpoints lose cylinder agreement",
                    ));
                }
            }
            return Ok(LineCylinderIntersection::Coincident {
                parameter_range: range,
                angle: cylinder_angle(a.x, a.y),
            });
        }
        if !gap.is_finite() || gap.abs() <= budget {
            return Err(Error::InvalidInput(
                "unresolved near-coincident cylinder generator",
            ));
        }
        return Ok(LineCylinderIntersection::Empty);
    }
    if radial_speed <= tol.angular().sin() {
        return Err(Error::Unsupported(
            "near-axial line-cylinder intersection is unresolved under angular policy",
        ));
    }
    // Closest approach in the radial plane avoids cancellation in B²-4AC.
    let ux = d.x / radial_speed;
    let uy = d.y / radial_speed;
    let along = a.x * ux + a.y * uy;
    let perpendicular = (a.x * (-uy) + a.y * ux).abs();
    if !along.is_finite() || !perpendicular.is_finite() {
        return Err(Error::InvalidInput(
            "line-cylinder radial projection exceeds finite range",
        ));
    }
    let gap = perpendicular - radius;
    let contact = if gap == 0.0 {
        IntersectionContact::Tangent
    } else if gap.abs() <= budget {
        return Err(Error::InvalidInput(
            "unresolved near-tangent line-cylinder intersection",
        ));
    } else if gap > 0.0 {
        return Ok(LineCylinderIntersection::Empty);
    } else {
        IntersectionContact::Crossing
    };
    let half = radius * (1.0 - (perpendicular / radius).powi(2)).max(0.0).sqrt();
    let travels = if contact == IntersectionContact::Tangent {
        vec![-along / radial_speed]
    } else {
        vec![
            (-along - half) / radial_speed,
            (-along + half) / radial_speed,
        ]
    };
    let mut hits = Vec::new();
    for travel in travels {
        let local = a + d * travel;
        if !local.finite() {
            return Err(Error::InvalidInput(
                "cylinder intersection exceeds finite coordinate range",
            ));
        }
        let parameter = line_parameter(travel, direction)?;
        let point = anchor + unit * travel;
        let reconstructed_local = frame.local_point(point);
        if !reconstructed_local.finite()
            || (reconstructed_local.x.hypot(reconstructed_local.y) - radius).abs() > budget
            || (reconstructed_local.z - local.z).abs() > tol.length_at_scale(height)?
        {
            return Err(Error::InvalidInput(
                "intersection loses local cylinder dimensions at world coordinate magnitude",
            ));
        }
        let uv = [cylinder_angle(local.x, local.y), local.z];
        let scale = radius.max(height).max(travel.abs());
        if !tol.coincident(point, anchor + direction * parameter, scale)?
            || !tol.coincident(point, cylinder.evaluate(uv[0], uv[1]), radius.max(height))?
            || (local.x.hypot(local.y) - radius).abs() > budget
        {
            return Err(Error::InvalidInput(
                "line-cylinder hit loses line/surface agreement",
            ));
        }
        // Do not silently select axial membership at an unresolved end level.
        let end_guard = 64.0 * f64::EPSILON * height.max(a.z.abs()).max((d.z * travel).abs());
        if local.z != 0.0
            && local.z != height
            && local.z.abs().min((local.z - height).abs()) <= end_guard
        {
            return Err(Error::InvalidInput(
                "cylinder hit is unresolved at an axial end level",
            ));
        }
        if local.z < 0.0 || local.z > height {
            continue;
        }
        hits.push(CylinderIntersectionPoint {
            point,
            parameter,
            uv,
            contact,
        });
    }
    if hits.len() == 2
        && (hits[1].parameter <= hits[0].parameter
            || (hits[1].point - hits[0].point).norm() <= budget)
    {
        return Err(Error::InvalidInput("line-cylinder roots are unresolved"));
    }
    Ok(if hits.is_empty() {
        LineCylinderIntersection::Empty
    } else {
        LineCylinderIntersection::Points(hits)
    })
}

/// Native/WASM fixture: mode 0 secant planes/transverse line, 1 parallel planes,
/// 2 axial generator. `offset` is the radial Y coordinate (modes 0/1) or X
/// coordinate (mode 2); placement rotates all input geometry together.
pub fn intersections_demo_json(mode: u32, offset: f64, placement: f64) -> Result<String> {
    if mode > 2 || !offset.is_finite() || !placement.is_finite() {
        return Err(Error::InvalidInput(
            "intersection demo requires mode 0..2 and finite values",
        ));
    }
    let tol = GeometryTolerance::default();
    let tr = Transform::rotation(Vec3::new(1.0, 2.0, 3.0), placement)?;
    let first = Surface::Plane {
        origin: Point3::new(0.0, 0.0, 0.0),
        u: Vec3::new(1.0, 0.0, 0.0),
        v: Vec3::new(0.0, 1.0, 0.0),
    }
    .transformed(tr)?;
    let second = if mode == 1 {
        Surface::Plane {
            origin: Point3::new(0.0, 0.0, offset),
            u: Vec3::new(1.0, 0.0, 0.0),
            v: Vec3::new(0.0, 1.0, 0.0),
        }
    } else {
        Surface::Plane {
            origin: Point3::new(3.0, 0.0, 0.0),
            u: Vec3::new(0.0, 1.0, 0.0),
            v: Vec3::new(0.0, 0.0, 1.0),
        }
    }
    .transformed(tr)?;
    let cylinder = Surface::Cylinder {
        center: Point3::new(0.0, 0.0, 0.0),
        radius: 2.0,
        height: 4.0,
    }
    .transformed(tr)?;
    let (a, d) = if mode == 2 {
        (Point3::new(offset, 0.0, 1.0), Vec3::new(0.0, 0.0, 2.0))
    } else {
        (Point3::new(-5.0, offset, 2.0), Vec3::new(2.0, 0.0, 0.0))
    };
    let xyz = |p: Vec3| format!("[{},{},{}]", p.x, p.y, p.z);
    let uv_line = |p: &PCurve| -> Result<String> {
        let PCurve::Affine { origin, direction } = p else {
            return Err(Error::InvalidInput("expected affine intersection pcurve"));
        };
        Ok(format!(
            "{{\"origin\":[{},{}],\"direction\":[{},{}]}}",
            origin[0], origin[1], direction[0], direction[1]
        ))
    };
    let planes = match intersect_plane_plane(&first, &second, tol)? {
        PlanePlaneIntersection::Parallel => "{\"kind\":\"parallel\"}".to_string(),
        PlanePlaneIntersection::Coincident => "{\"kind\":\"coincident\"}".to_string(),
        PlanePlaneIntersection::Line(l) => format!(
            "{{\"kind\":\"line\",\"origin\":{},\"direction\":{},\"first_uv\":{},\"second_uv\":{}}}",
            xyz(l.origin),
            xyz(l.direction),
            uv_line(&l.first)?,
            uv_line(&l.second)?
        ),
    };
    let hits = match intersect_line_cylinder(tr.point(a), tr.vector(d), &cylinder, tol)? {
        LineCylinderIntersection::Empty => "{\"kind\":\"empty\"}".to_string(),
        LineCylinderIntersection::Coincident {
            parameter_range,
            angle,
        } => format!(
            "{{\"kind\":\"coincident\",\"range\":[{},{}],\"angle\":{}}}",
            parameter_range[0], parameter_range[1], angle
        ),
        LineCylinderIntersection::Points(points) => {
            let values: Vec<_> = points
                .into_iter()
                .map(|p| {
                    format!(
                        "{{\"point\":{},\"parameter\":{},\"uv\":[{},{}],\"contact\":\"{}\"}}",
                        xyz(p.point),
                        p.parameter,
                        p.uv[0],
                        p.uv[1],
                        if p.contact == IntersectionContact::Tangent {
                            "tangent"
                        } else {
                            "crossing"
                        }
                    )
                })
                .collect();
            format!("{{\"kind\":\"points\",\"points\":[{}]}}", values.join(","))
        }
    };
    Ok(format!("{{\"planes\":{planes},\"cylinder\":{hits}}}"))
}
