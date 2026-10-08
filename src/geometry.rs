use crate::{Error, Frame3, Point3, Result, Tolerance, Transform, Vec3};
use std::f64::consts::TAU;
#[derive(Clone, Debug)]
pub enum Curve {
    Line { a: Point3, b: Point3 },
    Circle { center: Point3, radius: f64 },
    FramedCircle { frame: Frame3, radius: f64 },
}
impl Curve {
    pub fn evaluate(&self, t: f64) -> Point3 {
        match *self {
            Self::Line { a, b } => a + (b - a) * t,
            Self::FramedCircle { frame, radius } => {
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
            Self::FramedCircle { frame, radius } => Self::FramedCircle {
                frame: transform.compose(frame)?,
                radius,
            },
        })
    }
    pub fn range(&self) -> [f64; 2] {
        match self {
            Self::Line { .. } => [0.0, 1.0],
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
/// Exact intersection of a line with an infinite plane; parallel/coincident is explicit.
pub fn line_plane(a: Point3, d: Vec3, plane: &Surface) -> Result<Point3> {
    let Surface::Plane { origin, u, v } = *plane else {
        return Err(Error::Unsupported("line-plane requires a plane"));
    };
    if !a.finite()
        || !d.finite()
        || !origin.finite()
        || !u.finite()
        || !v.finite()
        || d.norm() == 0.0
    {
        return Err(Error::InvalidInput("invalid line or plane"));
    }
    let n = u.cross(v);
    if (u.norm() - 1.0).abs() > 1e-10 || (v.norm() - 1.0).abs() > 1e-10 || u.dot(v).abs() > 1e-10 {
        return Err(Error::InvalidInput("plane requires orthonormal axes"));
    }
    let den = n.dot(d);
    if den.abs() <= f64::EPSILON * d.norm() {
        return Err(Error::Unsupported("parallel or coincident line-plane"));
    }
    let point = a + d * (n.dot(origin - a) / den);
    if !point.finite() {
        return Err(Error::InvalidInput(
            "intersection exceeds finite coordinate range",
        ));
    }
    Ok(point)
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
        || !u.finite()
        || !v.finite()
        || (u.norm() - 1.0).abs() > 1e-10
        || (v.norm() - 1.0).abs() > 1e-10
        || u.dot(v).abs() > 1e-10
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
