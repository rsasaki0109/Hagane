//! Exact signs for finite binary64 inputs. Fast floating-point filter followed
//! by independent signed dyadic-integer arithmetic; no geometry dependency.
use crate::{Error, Result};
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Orientation {
    Clockwise,
    Collinear,
    CounterClockwise,
}
impl Orientation {
    pub fn sign(self) -> i32 {
        match self {
            Self::Clockwise => -1,
            Self::Collinear => 0,
            Self::CounterClockwise => 1,
        }
    }
}

/// Exact sign of `(b-a) × (c-a)` for the supplied finite f64 values.
/// This does not snap nearly collinear points or certify distance calculations.
pub fn orient2d(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> Result<Orientation> {
    if a.into_iter().chain(b).chain(c).any(|x| !x.is_finite()) {
        return Err(Error::InvalidInput(
            "orientation requires finite coordinates",
        ));
    }
    let x = b[0] - a[0];
    let y = b[1] - a[1];
    let u = c[0] - a[0];
    let v = c[1] - a[1];
    let left = x * v;
    let right = y * u;
    let det = left - right;
    // Conservative bound on subtraction/product roundoff. Underflow or
    // overflow bypasses the filter, including products rounded to zero.
    let normal_product = |p: f64, s: f64, t: f64| {
        p.is_finite() && (p.abs() >= f64::MIN_POSITIVE || s == 0.0 || t == 0.0)
    };
    let bound = 8.0 * f64::EPSILON * (left.abs() + right.abs());
    if normal_product(left, x, v)
        && normal_product(right, y, u)
        && det.is_finite()
        && bound.is_finite()
        && det.abs() >= f64::MIN_POSITIVE
        && det.abs() > bound
    {
        return Ok(if det > 0.0 {
            Orientation::CounterClockwise
        } else {
            Orientation::Clockwise
        });
    }
    let x = Integer::from_f64(b[0]).sub(&Integer::from_f64(a[0]));
    let y = Integer::from_f64(b[1]).sub(&Integer::from_f64(a[1]));
    let u = Integer::from_f64(c[0]).sub(&Integer::from_f64(a[0]));
    let v = Integer::from_f64(c[1]).sub(&Integer::from_f64(a[1]));
    let det = x.mul(&v).sub(&y.mul(&u));
    Ok(if det.words.is_empty() {
        Orientation::Collinear
    } else if det.negative {
        Orientation::Clockwise
    } else {
        Orientation::CounterClockwise
    })
}

/// Exact sign of `(b-a) × (c-a) · (d-a)` for finite binary64 inputs.
/// No metric snapping is applied; integer arithmetic also handles overflow and
/// underflow of the corresponding floating determinant.
pub fn orient3d(a: [f64; 3], b: [f64; 3], c: [f64; 3], d: [f64; 3]) -> Result<i32> {
    if a.into_iter()
        .chain(b)
        .chain(c)
        .chain(d)
        .any(|v| !v.is_finite())
    {
        return Err(Error::InvalidInput(
            "3D orientation requires finite coordinates",
        ));
    }
    let difference = |p: [f64; 3]| {
        std::array::from_fn(|i| Integer::from_f64(p[i]).sub(&Integer::from_f64(a[i])))
    };
    Ok(triple(&difference(b), &difference(c), &difference(d)))
}
fn triple(a: &[Integer; 3], b: &[Integer; 3], c: &[Integer; 3]) -> i32 {
    let x = a[1].mul(&b[2]).sub(&a[2].mul(&b[1])).mul(&c[0]);
    let y = a[2].mul(&b[0]).sub(&a[0].mul(&b[2])).mul(&c[1]);
    let z = a[0].mul(&b[1]).sub(&a[1].mul(&b[0])).mul(&c[2]);
    let zero = Integer::from_f64(0.);
    let det = x.sub(&zero.sub(&y)).sub(&zero.sub(&z));
    if det.words.is_empty() {
        0
    } else if det.negative {
        -1
    } else {
        1
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum ExactLineRelation {
    Skew,
    Coplanar,
    Parallel,
    Collinear,
}
/// Exact relation of an anchor/direction line and two stored edge endpoints.
/// Differences and products are dyadic integers, so no rounded `anchor+direction`
/// point or overflowed endpoint subtraction is used to certify incidence.
pub(crate) fn exact_line_relation(
    anchor: [f64; 3],
    direction: [f64; 3],
    a: [f64; 3],
    b: [f64; 3],
) -> Result<ExactLineRelation> {
    if anchor
        .into_iter()
        .chain(direction)
        .chain(a)
        .chain(b)
        .any(|x| !x.is_finite())
    {
        return Err(Error::InvalidInput("line incidence requires finite values"));
    }
    let d = direction.map(Integer::from_f64);
    let edge = std::array::from_fn(|i| Integer::from_f64(b[i]).sub(&Integer::from_f64(a[i])));
    let offset =
        std::array::from_fn(|i| Integer::from_f64(a[i]).sub(&Integer::from_f64(anchor[i])));
    let parallel = |x: &[Integer; 3], y: &[Integer; 3]| {
        [(0, 1), (0, 2), (1, 2)]
            .iter()
            .all(|&(i, j)| x[i].mul(&y[j]).sub(&x[j].mul(&y[i])).words.is_empty())
    };
    Ok(if parallel(&d, &edge) {
        if parallel(&d, &offset) {
            ExactLineRelation::Collinear
        } else {
            ExactLineRelation::Parallel
        }
    } else if triple(&d, &edge, &offset) == 0 {
        ExactLineRelation::Coplanar
    } else {
        ExactLineRelation::Skew
    })
}
/// Certify a line lies in the plane spanned by stored ellipse axes. Dyadic
/// differences avoid rounded center-plus-axis points and overflowing subtraction.
pub(crate) fn exact_line_in_ellipse_plane(
    anchor: [f64; 3],
    direction: [f64; 3],
    center: [f64; 3],
    cosine: [f64; 3],
    sine: [f64; 3],
) -> Result<bool> {
    if anchor
        .into_iter()
        .chain(direction)
        .chain(center)
        .chain(cosine)
        .chain(sine)
        .any(|x| !x.is_finite())
    {
        return Err(Error::InvalidInput(
            "ellipse incidence requires finite values",
        ));
    }
    let a = cosine.map(Integer::from_f64);
    let b = sine.map(Integer::from_f64);
    let d = direction.map(Integer::from_f64);
    let offset =
        std::array::from_fn(|i| Integer::from_f64(anchor[i]).sub(&Integer::from_f64(center[i])));
    Ok(triple(&a, &b, &d) == 0 && triple(&a, &b, &offset) == 0)
}
pub(crate) fn exact_plane_support(
    a: [f64; 3],
    u: [f64; 3],
    v: [f64; 3],
    b: [f64; 3],
    s: [f64; 3],
    t: [f64; 3],
) -> bool {
    let u = u.map(Integer::from_f64);
    let v = v.map(Integer::from_f64);
    let offset = std::array::from_fn(|i| Integer::from_f64(b[i]).sub(&Integer::from_f64(a[i])));
    triple(&u, &v, &s.map(Integer::from_f64)) == 0
        && triple(&u, &v, &t.map(Integer::from_f64)) == 0
        && triple(&u, &v, &offset) == 0
}

// Every finite binary64 is an integer multiple of 2^-1074. Coordinate
// differences and two/three-factor products fit in fewer than 132/198 u32 limbs.
// Little-endian magnitudes keep the fallback bounded without external crates.
struct Integer {
    negative: bool,
    words: Vec<u32>,
}
impl Integer {
    fn from_f64(value: f64) -> Self {
        let bits = value.to_bits();
        let exponent = ((bits >> 52) & 0x7ff) as usize;
        let mantissa = (bits & ((1u64 << 52) - 1)) | if exponent == 0 { 0 } else { 1u64 << 52 };
        if mantissa == 0 {
            return Self {
                negative: false,
                words: Vec::new(),
            };
        }
        let shift = exponent.saturating_sub(1);
        let mut words = vec![0; shift / 32 + 3];
        let index = shift / 32;
        let offset = shift % 32;
        let low = (mantissa as u32 as u64) << offset;
        let high = (mantissa >> 32) << offset;
        words[index] = low as u32;
        words[index + 1] = ((low >> 32) | high) as u32;
        words[index + 2] = (high >> 32) as u32;
        Self::canonical(bits >> 63 != 0, words)
    }
    fn canonical(negative: bool, mut words: Vec<u32>) -> Self {
        while words.last() == Some(&0) {
            words.pop();
        }
        Self {
            negative: negative && !words.is_empty(),
            words,
        }
    }
    fn compare(&self, other: &Self) -> Ordering {
        self.words
            .len()
            .cmp(&other.words.len())
            .then_with(|| self.words.iter().rev().cmp(other.words.iter().rev()))
    }
    fn sub(&self, other: &Self) -> Self {
        if self.negative != other.negative {
            let mut words = Vec::new();
            let mut carry = 0u64;
            for i in 0..self.words.len().max(other.words.len()) {
                let total = self.words.get(i).copied().unwrap_or(0) as u64
                    + other.words.get(i).copied().unwrap_or(0) as u64
                    + carry;
                words.push(total as u32);
                carry = total >> 32;
            }
            if carry != 0 {
                words.push(carry as u32);
            }
            return Self::canonical(self.negative, words);
        }
        let (large, small, negative) = if self.compare(other) == Ordering::Less {
            (other, self, !self.negative)
        } else {
            (self, other, self.negative)
        };
        let mut words = Vec::with_capacity(large.words.len());
        let mut borrow = 0u64;
        for (i, &word) in large.words.iter().enumerate() {
            let subtract = small.words.get(i).copied().unwrap_or(0) as u64 + borrow;
            words.push((word as u64).wrapping_sub(subtract) as u32);
            borrow = u64::from((word as u64) < subtract);
        }
        Self::canonical(negative, words)
    }
    fn mul(&self, other: &Self) -> Self {
        if self.words.is_empty() || other.words.is_empty() {
            return Self::canonical(false, Vec::new());
        }
        let mut words = vec![0u32; self.words.len() + other.words.len()];
        for (i, &a) in self.words.iter().enumerate() {
            if a == 0 {
                continue;
            }
            let mut carry = 0u64;
            for (j, &b) in other.words.iter().enumerate() {
                let product = a as u64 * b as u64 + words[i + j] as u64 + carry;
                words[i + j] = product as u32;
                carry = product >> 32;
            }
            words[i + other.words.len()] = carry as u32;
        }
        Self::canonical(self.negative != other.negative, words)
    }
}

fn on_box(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> bool {
    (0..2).all(|i| p[i] >= a[i].min(b[i]) && p[i] <= a[i].max(b[i]))
}
/// Exact closed-segment intersection test, including point segments, endpoint
/// contact and collinear overlap. No metric tolerance is applied.
pub fn segments_intersect2d(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> Result<bool> {
    let s1 = orient2d(a, b, c)?.sign();
    let s2 = orient2d(a, b, d)?.sign();
    let s3 = orient2d(c, d, a)?.sign();
    let s4 = orient2d(c, d, b)?.sign();
    Ok((s1 * s2 < 0 && s3 * s4 < 0)
        || (s1 == 0 && on_box(c, a, b))
        || (s2 == 0 && on_box(d, a, b))
        || (s3 == 0 && on_box(a, c, d))
        || (s4 == 0 && on_box(b, c, d)))
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointLocation {
    Outside,
    Boundary,
    Inside,
}
/// Exact boundary/even-odd classification for a finite polygon ring with
/// 3..4096 corners. This is not a simplicity or nondegeneracy validator;
/// self-crossing rings follow the even-odd fill rule. No tolerance is applied.
pub fn locate_point_in_polygon(p: [f64; 2], polygon: &[[f64; 2]]) -> Result<PointLocation> {
    if polygon.len() < 3
        || polygon.len() > 4096
        || p.iter()
            .chain(polygon.iter().flatten())
            .any(|v| !v.is_finite())
    {
        return Err(Error::InvalidInput(
            "point location requires finite points and 3..4096 polygon corners",
        ));
    }
    let mut inside = false;
    for i in 0..polygon.len() {
        let a = polygon[i];
        let b = polygon[(i + 1) % polygon.len()];
        let orientation = orient2d(a, b, p)?;
        if orientation == Orientation::Collinear && on_box(p, a, b) {
            return Ok(PointLocation::Boundary);
        }
        if (a[1] > p[1]) != (b[1] > p[1])
            && ((b[1] > a[1] && orientation == Orientation::CounterClockwise)
                || (b[1] < a[1] && orientation == Orientation::Clockwise))
        {
            inside = !inside;
        }
    }
    Ok(if inside {
        PointLocation::Inside
    } else {
        PointLocation::Outside
    })
}

/// Native/WASM numerical-policy fixture, independent of the display mesh.
pub fn predicates_demo_json(scale: f64, angle: f64) -> Result<String> {
    use crate::{
        intersect_line_plane, GeometryTolerance, LinePlaneIntersection, Point3, Surface, Vec3,
    };
    if !scale.is_finite() || scale <= 0.0 || !angle.is_finite() {
        return Err(Error::InvalidInput(
            "positive finite scale and finite angle required",
        ));
    }
    let tolerance = GeometryTolerance::default();
    let orientation = orient2d(
        [0.0, 0.0],
        [134217729.0, 134217728.0],
        [134217728.0, 134217727.0],
    )?
    .sign();
    let plane = Surface::Plane {
        origin: Point3::new(0.0, 0.0, 0.0),
        u: Vec3::new(1.0, 0.0, 0.0),
        v: Vec3::new(0.0, 1.0, 0.0),
    };
    let result = intersect_line_plane(
        Point3::new(0.0, 0.0, scale),
        Vec3::new(angle.cos(), 0.0, -angle.sin()),
        &plane,
        tolerance,
    )?;
    let (classification, point, parameter) = match result {
        LinePlaneIntersection::Point { point, parameter } => (
            "point",
            format!("[{},{},{}]", point.x, point.y, point.z),
            parameter.to_string(),
        ),
        LinePlaneIntersection::Parallel => ("parallel", "null".into(), "null".into()),
        LinePlaneIntersection::Coincident => ("coincident", "null".into(), "null".into()),
    };
    Ok(format!("{{\"orientation\":{},\"length_budget\":{},\"line_plane\":\"{}\",\"point\":{},\"parameter\":{}}}",orientation,tolerance.length_at_scale(scale)?,classification,point,parameter))
}
