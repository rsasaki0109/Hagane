//! Cartesian geometry. Lengths use one caller-selected, consistent unit.
use crate::{Error, Result};
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}
pub type Point3 = Vec3;
impl Vec3 {
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
    pub fn dot(self, b: Self) -> f64 {
        self.x * b.x + self.y * b.y + self.z * b.z
    }
    pub fn cross(self, b: Self) -> Self {
        Self::new(
            self.y * b.z - self.z * b.y,
            self.z * b.x - self.x * b.z,
            self.x * b.y - self.y * b.x,
        )
    }
    pub fn norm(self) -> f64 {
        self.x.hypot(self.y).hypot(self.z)
    }
    pub fn finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }
    pub fn normalized(self) -> Result<Self> {
        let n = self.norm();
        if !n.is_finite() || n == 0.0 {
            return Err(Error::InvalidInput("zero or nonfinite direction"));
        }
        Ok(Self::new(self.x / n, self.y / n, self.z / n))
    }
}
impl std::ops::Add for Vec3 {
    type Output = Self;
    fn add(self, b: Self) -> Self {
        Self::new(self.x + b.x, self.y + b.y, self.z + b.z)
    }
}
impl std::ops::Sub for Vec3 {
    type Output = Self;
    fn sub(self, b: Self) -> Self {
        Self::new(self.x - b.x, self.y - b.y, self.z - b.z)
    }
}
impl std::ops::Mul<f64> for Vec3 {
    type Output = Self;
    fn mul(self, b: f64) -> Self {
        Self::new(self.x * b, self.y * b, self.z * b)
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Tolerance {
    pub linear: f64,
}
impl Default for Tolerance {
    fn default() -> Self {
        Self { linear: 1e-8 }
    }
}
impl Tolerance {
    pub fn new(linear: f64) -> Result<Self> {
        if !linear.is_finite() || linear <= 0.0 {
            Err(Error::InvalidInput("tolerance must be positive and finite"))
        } else {
            Ok(Self { linear })
        }
    }
    pub fn coincident(self, a: Point3, b: Point3) -> bool {
        (a - b).norm() <= self.linear
    }
}
/// Right-handed orthonormal coordinate frame, also used as a rigid transform.
/// Scale, shear and reflections are excluded. Components are immutable.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    origin: Point3,
    axes: [Vec3; 3],
}
/// Local-to-world frame. Its transform acts on points and directions separately.
pub type Frame3 = Transform;
impl Transform {
    pub const IDENTITY: Self = Self {
        origin: Point3::new(0.0, 0.0, 0.0),
        axes: [
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
        ],
    };
    /// Checks all pairwise orthogonality constraints. The acceptance threshold
    /// is capped at 1e-10 independently of the caller's length unit. Roundoff
    /// within that threshold is orthonormalized; measurable scale is rejected.
    pub fn new(origin: Point3, axes: [Vec3; 3], tol: Tolerance) -> Result<Self> {
        Tolerance::new(tol.linear)?;
        let epsilon = tol.linear.min(1e-10);
        if !origin.finite()
            || axes
                .iter()
                .any(|a| !a.finite() || (a.norm() - 1.0).abs() > epsilon)
            || axes[0].dot(axes[1]).abs() > epsilon
            || axes[0].dot(axes[2]).abs() > epsilon
            || axes[1].dot(axes[2]).abs() > epsilon
            || (axes[0].cross(axes[1]).dot(axes[2]) - 1.0).abs() > epsilon
        {
            return Err(Error::InvalidInput(
                "transform requires a right-handed orthonormal basis",
            ));
        }
        let u = axes[0].normalized()?;
        let v = (axes[1] - u * u.dot(axes[1])).normalized()?;
        let w = u.cross(v).normalized()?;
        Ok(Self {
            origin,
            axes: [u, w.cross(u), w],
        })
    }
    pub fn origin(self) -> Point3 {
        self.origin
    }
    pub fn axes(self) -> [Vec3; 3] {
        self.axes
    }
    pub fn translation(offset: Vec3) -> Result<Self> {
        Self::new(offset, Self::IDENTITY.axes, Tolerance::default())
    }
    /// Rodrigues rotation about an axis through the world origin, in radians.
    pub fn rotation(axis: Vec3, angle: f64) -> Result<Self> {
        if !angle.is_finite() {
            return Err(Error::InvalidInput("rotation angle must be finite"));
        }
        let axis = axis.normalized()?;
        let (s, c) = angle.sin_cos();
        let rotate = |v: Vec3| v * c + axis.cross(v) * s + axis * (axis.dot(v) * (1.0 - c));
        Self::new(
            Self::IDENTITY.origin,
            Self::IDENTITY.axes.map(rotate),
            Tolerance::default(),
        )
    }
    pub fn vector(self, v: Vec3) -> Vec3 {
        self.axes[0] * v.x + self.axes[1] * v.y + self.axes[2] * v.z
    }
    pub fn point(self, p: Point3) -> Point3 {
        self.origin + self.vector(p)
    }
    pub fn local_vector(self, v: Vec3) -> Vec3 {
        Vec3::new(
            v.dot(self.axes[0]),
            v.dot(self.axes[1]),
            v.dot(self.axes[2]),
        )
    }
    pub fn local_point(self, p: Point3) -> Point3 {
        self.local_vector(p - self.origin)
    }
    /// Composition: apply `inner` first, then `self`. Overflow is an error.
    pub fn compose(self, inner: Self) -> Result<Self> {
        Self::new(
            self.point(inner.origin),
            inner.axes.map(|v| self.vector(v)),
            Tolerance::default(),
        )
    }
    pub fn inverse(self) -> Result<Self> {
        let axes = [
            Vec3::new(self.axes[0].x, self.axes[1].x, self.axes[2].x),
            Vec3::new(self.axes[0].y, self.axes[1].y, self.axes[2].y),
            Vec3::new(self.axes[0].z, self.axes[1].z, self.axes[2].z),
        ];
        Self::new(
            self.local_vector(self.origin) * -1.0,
            axes,
            Tolerance::default(),
        )
    }
}
