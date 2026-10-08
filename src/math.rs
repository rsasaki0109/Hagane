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
        let scale = self.x.abs().max(self.y.abs()).max(self.z.abs());
        if !self.finite() || scale == 0.0 {
            return Err(Error::InvalidInput("zero or nonfinite direction"));
        }
        let scaled = Self::new(self.x / scale, self.y / scale, self.z / scale);
        let n = scaled.norm();
        Ok(Self::new(scaled.x / n, scaled.y / n, scaled.z / n))
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
/// Distinct absolute length, angular (radians), and dimensionless relative
/// tolerances. Scale must be a local geometric length, not a world offset.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeometryTolerance {
    linear: f64,
    angular: f64,
    relative: f64,
}
impl Default for GeometryTolerance {
    fn default() -> Self {
        Self {
            linear: 1e-8,
            angular: 1e-10,
            relative: 1e-10,
        }
    }
}
impl GeometryTolerance {
    pub fn new(linear: f64, angular: f64, relative: f64) -> Result<Self> {
        Tolerance::new(linear)?;
        if !angular.is_finite()
            || angular <= 0.0
            || angular >= std::f64::consts::FRAC_PI_2
            || !relative.is_finite()
            || !(0.0..1.0).contains(&relative)
        {
            return Err(Error::InvalidInput(
                "angular tolerance must be in (0, pi/2), relative tolerance in [0, 1)",
            ));
        }
        Ok(Self {
            linear,
            angular,
            relative,
        })
    }
    pub fn linear(self) -> f64 {
        self.linear
    }
    pub fn angular(self) -> f64 {
        self.angular
    }
    pub fn relative(self) -> f64 {
        self.relative
    }
    pub fn absolute(self) -> Tolerance {
        Tolerance {
            linear: self.linear,
        }
    }
    pub fn length_at_scale(self, scale: f64) -> Result<f64> {
        if !scale.is_finite() || scale < 0.0 {
            return Err(Error::InvalidInput(
                "tolerance scale must be a finite nonnegative geometric length",
            ));
        }
        Ok(self.linear.max(self.relative * scale))
    }
    pub fn coincident(self, a: Point3, b: Point3, scale: f64) -> Result<bool> {
        if !a.finite() || !b.finite() {
            return Err(Error::InvalidInput("coincidence requires finite points"));
        }
        let length = (a - b).norm();
        if !length.is_finite() {
            return Err(Error::InvalidInput("point separation exceeds finite range"));
        }
        Ok(length <= self.length_at_scale(scale)?)
    }
    /// Parallel or antiparallel, within the angular budget.
    pub fn parallel(self, a: Vec3, b: Vec3) -> Result<bool> {
        Ok(a.normalized()?.cross(b.normalized()?).norm() <= self.angular.sin())
    }
    pub fn perpendicular(self, a: Vec3, b: Vec3) -> Result<bool> {
        Ok(a.normalized()?.dot(b.normalized()?).abs() <= self.angular.sin())
    }
}
impl TryFrom<Tolerance> for GeometryTolerance {
    type Error = Error;
    fn try_from(tol: Tolerance) -> Result<Self> {
        Self::new(
            tol.linear,
            Self::default().angular,
            Self::default().relative,
        )
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
    /// Compatibility constructor: the linear tolerance is validated; axis
    /// checks use default angular/relative budgets, independently of length units.
    /// Use `new_with_tolerance` to choose those budgets explicitly.
    pub fn new(origin: Point3, axes: [Vec3; 3], tol: Tolerance) -> Result<Self> {
        Self::new_with_tolerance(origin, axes, GeometryTolerance::try_from(tol)?)
    }
    /// Angular and relative budgets are dimensionless and capped at 1e-10
    /// for rigid-frame orthonormalization; the length budget does not affect axes.
    pub fn new_with_tolerance(
        origin: Point3,
        axes: [Vec3; 3],
        tol: GeometryTolerance,
    ) -> Result<Self> {
        let epsilon = tol.angular.sin().min(1e-10);
        let relative = tol.relative.clamp(64.0 * f64::EPSILON, 1e-10);
        if !origin.finite()
            || axes
                .iter()
                .any(|a| !a.finite() || (a.norm() - 1.0).abs() > relative)
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
