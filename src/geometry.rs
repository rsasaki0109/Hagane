use crate::{Error, Frame3, GeometryTolerance, Point3, Result, Tolerance, Transform, Vec3};
use std::f64::consts::TAU;
#[derive(Clone, Debug)]
pub enum Curve {
    Line {
        a: Point3,
        b: Point3,
    },
    Circle {
        center: Point3,
        radius: f64,
    },
    FramedCircle {
        frame: Frame3,
        radius: f64,
    },
    Arc {
        frame: Frame3,
        radius: f64,
        sweep: f64,
    },
}
impl Curve {
    pub fn evaluate(&self, t: f64) -> Point3 {
        match *self {
            Self::Line { a, b } => a + (b - a) * t,
            Self::FramedCircle { frame, radius } | Self::Arc { frame, radius, .. } => {
                frame.point(Vec3::new(radius * t.cos(), radius * t.sin(), 0.0))
            }
            Self::Circle { center, radius } => {
                center + Vec3::new(radius * t.cos(), radius * t.sin(), 0.0)
            }
        }
    }
    pub fn transformed(&self, transform: Transform) -> Result<Self> {
        Ok(match *self {
            Self::Line { a, b } => {
                let (a, b) = (transform.point(a), transform.point(b));
                if !a.finite() || !b.finite() {
                    return Err(Error::InvalidInput(
                        "transformed line exceeds finite coordinates",
                    ));
                }
                Self::Line { a, b }
            }
            Self::Circle { center, radius } => Self::FramedCircle {
                frame: transform.compose(Frame3::translation(center)?)?,
                radius,
            },
            Self::Arc {
                frame,
                radius,
                sweep,
            } => Self::Arc {
                frame: transform.compose(frame)?,
                radius,
                sweep,
            },
            Self::FramedCircle { frame, radius } => Self::FramedCircle {
                frame: transform.compose(frame)?,
                radius,
            },
        })
    }
    pub fn range(&self) -> [f64; 2] {
        match self {
            Self::Line { .. } => [0.0, 1.0],
            Self::Arc { sweep, .. } => [0.0, *sweep],
            Self::Circle { .. } | Self::FramedCircle { .. } => [0.0, TAU],
        }
    }
}
#[derive(Clone, Debug)]
pub enum Surface {
    Plane {
        origin: Point3,
        u: Vec3,
        v: Vec3,
    },
    FramedCylinder {
        frame: Frame3,
        radius: f64,
        height: f64,
    },
    Cylinder {
        center: Point3,
        radius: f64,
        height: f64,
    },
}
impl Surface {
    pub fn transformed(&self, transform: Transform) -> Result<Self> {
        Ok(match *self {
            Self::Plane { origin, u, v } => {
                let origin = transform.point(origin);
                if !origin.finite() {
                    return Err(Error::InvalidInput(
                        "transformed plane exceeds finite coordinates",
                    ));
                }
                Self::Plane {
                    origin,
                    u: transform.vector(u),
                    v: transform.vector(v),
                }
            }
            Self::Cylinder {
                center,
                radius,
                height,
            } => Self::FramedCylinder {
                frame: transform.compose(Frame3::translation(center)?)?,
                radius,
                height,
            },
            Self::FramedCylinder {
                frame,
                radius,
                height,
            } => Self::FramedCylinder {
                frame: transform.compose(frame)?,
                radius,
                height,
            },
        })
    }
    pub fn evaluate(&self, u: f64, v: f64) -> Point3 {
        match *self {
            Self::Plane {
                origin,
                u: du,
                v: dv,
            } => origin + du * u + dv * v,
            Self::FramedCylinder { frame, radius, .. } => {
                frame.point(Vec3::new(radius * u.cos(), radius * u.sin(), v))
            }
            Self::Cylinder { center, radius, .. } => {
                center + Vec3::new(radius * u.cos(), radius * u.sin(), v)
            }
        }
    }
    pub fn normal(&self, u: f64) -> Vec3 {
        match *self {
            Self::Plane { u, v, .. } => u.cross(v),
            Self::Cylinder { .. } => Vec3::new(u.cos(), u.sin(), 0.0),
            Self::FramedCylinder { frame, .. } => frame.vector(Vec3::new(u.cos(), u.sin(), 0.0)),
        }
    }
    pub fn parameters(&self, p: Point3) -> [f64; 2] {
        match *self {
            Self::Plane { origin, u, v } => [(p - origin).dot(u), (p - origin).dot(v)],
            Self::FramedCylinder { frame, .. } => {
                let p = frame.local_point(p);
                [p.y.atan2(p.x).rem_euclid(TAU), p.z]
            }
            Self::Cylinder { center, .. } => [
                (p.y - center.y).atan2(p.x - center.x).rem_euclid(TAU),
                p.z - center.z,
            ],
        }
    }
}
/// Line/plane classification according to explicit distance and angle budgets.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LinePlaneIntersection {
    Point {
        point: Point3,
        parameter: f64,
    },
    /// Direction lies within the angular budget of the plane; anchor lies outside
    /// its distance budget. This is not a claim of exact mathematical parallelism.
    Parallel,
    /// Direction and anchor are within the plane's angular/distance budgets.
    Coincident,
}
pub(crate) fn plane_basis_valid(u: Vec3, v: Vec3, tol: GeometryTolerance) -> bool {
    u.finite()
        && v.finite()
        && (u.norm() - 1.0).abs() <= tol.relative().clamp(64.0 * f64::EPSILON, 1e-10)
        && (v.norm() - 1.0).abs() <= tol.relative().clamp(64.0 * f64::EPSILON, 1e-10)
        && u.dot(v).abs() <= tol.angular().sin().min(1e-10)
}
/// Checked intersection/classification of an infinite line and plane. The
/// returned parameter satisfies `point = anchor + direction * parameter`.
/// Relative distance scale is the anchor-to-plane-origin separation.
pub fn intersect_line_plane(
    a: Point3,
    d: Vec3,
    plane: &Surface,
    tol: GeometryTolerance,
) -> Result<LinePlaneIntersection> {
    let Surface::Plane { origin, u, v } = *plane else {
        return Err(Error::Unsupported("line-plane requires a plane"));
    };
    if !a.finite() || !origin.finite() || !plane_basis_valid(u, v, tol) {
        return Err(Error::InvalidInput("invalid line or orthonormal plane"));
    }
    let unit = d.normalized()?;
    let n = u.cross(v).normalized()?;
    let offset = origin - a;
    let scale = offset.norm();
    if !scale.is_finite() {
        return Err(Error::InvalidInput(
            "line-plane separation exceeds finite range",
        ));
    }
    let distance = n.dot(offset);
    let den = n.dot(unit);
    if !distance.is_finite() || !den.is_finite() {
        return Err(Error::InvalidInput(
            "line-plane projection exceeds finite range",
        ));
    }
    let budget = tol.length_at_scale(scale)?;
    if den.abs() <= tol.angular().sin() {
        return Ok(if distance.abs() <= budget {
            LinePlaneIntersection::Coincident
        } else {
            LinePlaneIntersection::Parallel
        });
    }
    let travel = distance / den;
    let direction_scale = d.x.abs().max(d.y.abs()).max(d.z.abs());
    let scaled_direction = Vec3::new(
        d.x / direction_scale,
        d.y / direction_scale,
        d.z / direction_scale,
    );
    let parameter = (travel / direction_scale) / scaled_direction.norm();
    let point = a + unit * travel;
    if !point.finite()
        || !parameter.is_finite()
        || !travel.is_finite()
        || (travel != 0.0 && parameter == 0.0)
    {
        return Err(Error::InvalidInput(
            "intersection exceeds finite coordinate/parameter range",
        ));
    }
    let reconstructed = a + d * parameter;
    if !reconstructed.finite() || !tol.coincident(point, reconstructed, scale.max(travel.abs()))? {
        return Err(Error::InvalidInput(
            "intersection loses line agreement at this parameter magnitude",
        ));
    }
    let residual = (point - origin).dot(n).abs();
    if !residual.is_finite() || residual > tol.length_at_scale(scale.max(travel.abs()))? {
        return Err(Error::InvalidInput(
            "intersection loses plane agreement at this coordinate magnitude",
        ));
    }
    Ok(LinePlaneIntersection::Point { point, parameter })
}
/// Compatibility point-only API; near-parallel/coincident cases are explicit errors.
pub fn line_plane(a: Point3, d: Vec3, plane: &Surface) -> Result<Point3> {
    match intersect_line_plane(a, d, plane, GeometryTolerance::default())? {
        LinePlaneIntersection::Point { point, .. } => Ok(point),
        _ => Err(Error::Unsupported(
            "parallel or coincident line-plane within tolerance",
        )),
    }
}

/// Exact intersection circle of a bounded Z-cylinder and a horizontal plane.
/// Tilted planes and intersections outside the axial interval are unsupported.
pub fn cylinder_plane(cylinder: &Surface, plane: &Surface, tol: Tolerance) -> Result<Curve> {
    Tolerance::new(tol.linear)?;
    let Surface::Cylinder {
        center,
        radius,
        height,
    } = *cylinder
    else {
        return Err(Error::Unsupported("expected a cylinder surface"));
    };
    let Surface::Plane { origin, u, v } = *plane else {
        return Err(Error::Unsupported("expected a plane surface"));
    };
    if !center.finite()
        || !origin.finite()
        || !radius.is_finite()
        || radius <= tol.linear
        || !height.is_finite()
        || height <= tol.linear
        || !plane_basis_valid(u, v, GeometryTolerance::try_from(tol)?)
    {
        return Err(Error::InvalidInput("invalid intersection surfaces"));
    }
    if u.z != 0.0 || v.z != 0.0 {
        return Err(Error::Unsupported("tilted cylinder-plane intersection"));
    }
    let offset = origin.z - center.z;
    if offset < 0.0 || offset > height {
        return Err(Error::Unsupported("plane outside bounded cylinder"));
    }
    Ok(Curve::Circle {
        center: Point3::new(center.x, center.y, origin.z),
        radius,
    })
}
