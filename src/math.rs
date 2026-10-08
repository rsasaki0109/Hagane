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
        self.dot(self).sqrt()
    }
    pub fn finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }
    pub fn normalized(self) -> Result<Self> {
        let n = self.norm();
        if !n.is_finite() || n == 0.0 {
            return Err(Error::InvalidInput("zero or nonfinite direction"));
        }
        Ok(self * (1.0 / n))
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
/// Rigid transform; reflections and scale are intentionally excluded.
#[derive(Clone, Copy, Debug)]
pub struct Transform {
    pub origin: Point3,
    pub axes: [Vec3; 3],
}
impl Transform {
    pub fn new(origin: Point3, axes: [Vec3; 3], tol: Tolerance) -> Result<Self> {
        if !origin.finite()
            || axes
                .iter()
                .any(|a| !a.finite() || (a.norm() - 1.0).abs() > tol.linear)
            || axes[0].dot(axes[1]).abs() > tol.linear
            || axes[0].cross(axes[1]).dot(axes[2]) < 1.0 - tol.linear
        {
            return Err(Error::InvalidInput(
                "transform requires a right-handed orthonormal basis",
            ));
        }
        Ok(Self { origin, axes })
    }
    pub fn point(self, p: Point3) -> Point3 {
        self.origin + self.axes[0] * p.x + self.axes[1] * p.y + self.axes[2] * p.z
    }
}
