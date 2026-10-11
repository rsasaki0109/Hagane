//! Finite transverse torus intersections using bounded polynomial sign proofs.
use crate::*;
#[derive(Clone, Debug)]
pub struct NurbsTorusSegmentHit {
    pub parameter: f64,
    pub point: Point3,
    pub boundary: NurbsTorusBoundaryWitness,
}
#[derive(Clone, Debug)]
pub struct NurbsTorusSegmentIntersection {
    pub hits: Vec<NurbsTorusSegmentHit>,
    pub material_intervals: Vec<[f64; 2]>,
}
fn unresolved() -> Error {
    Error::Unsupported(
        "torus segment roots, tangency or separation are unresolved at caller tolerance",
    )
}
fn norm(v: Vec3) -> f64 {
    v.x.hypot(v.y).hypot(v.z)
}
struct Query<'a> {
    body: &'a NurbsTorusSolid,
    start: Point3,
    delta: Vec3,
    local: Point3,
    d: Vec3,
    length: f64,
    band: f64,
    error: f64,
    policy: GeometryTolerance,
}
impl Query<'_> {
    fn signed(&self, t: f64) -> Result<f64> {
        let p = self.local + self.d * t;
        let [r, s] = self.body.radii();
        let f = (p.x.hypot(p.y) - r).hypot(p.z) - s;
        if !f.is_finite() {
            return Err(unresolved());
        }
        Ok(f)
    }
    fn point(&self, t: f64) -> Point3 {
        self.start + self.delta * t
    }
    fn root(&self, mut lo: f64, mut hi: f64) -> Result<f64> {
        let mut fl = self.signed(lo)?;
        for _ in 0..128 {
            let mid = lo + (hi - lo) * 0.5;
            if self.length * (hi - lo) <= self.policy.linear() / 16. {
                return Ok(mid);
            }
            if mid == lo || mid == hi {
                return Err(unresolved());
            }
            let fm = self.signed(mid)?;
            if fm.abs() <= self.error {
                let dt = self.policy.linear() / (64. * self.length);
                let a = mid - dt;
                let b = mid + dt;
                if a <= lo || b >= hi {
                    return Err(unresolved());
                }
                let fa = self.signed(a)?;
                let fb = self.signed(b)?;
                if fa.abs() > self.error && fb.abs() > self.error && fa.signum() != fb.signum() {
                    return Ok(mid);
                }
                return Err(unresolved());
            }
            if fm.signum() == fl.signum() {
                lo = mid;
                fl = fm;
            } else {
                hi = mid;
            }
        }
        Err(unresolved())
    }
}
// Every signed Bernstein coefficient must clear the propagated operand guard.
// Mixed cells form collars; no root can be hidden in an accepted collar because
// physical signed tube distance is one-Lipschitz along the original segment.
fn monotonic(coeff: [f64; 4], error: f64, target: f64) -> Result<Vec<(f64, f64, i8)>> {
    let mut stack = vec![(0., 1., coeff, error, 0usize)];
    let mut leaves = Vec::new();
    let mut work = 0;
    while let Some((a, b, c, e, depth)) = stack.pop() {
        work += 1;
        if work > 4096 {
            return Err(Error::Unsupported(
                "torus derivative isolation work budget exceeded",
            ));
        }
        let sign = if c.iter().all(|x| *x > e) {
            1
        } else if c.iter().all(|x| *x < -e) {
            -1
        } else {
            0
        };
        if sign != 0 || b - a <= target {
            leaves.push((a, b, sign));
            continue;
        }
        if depth >= 64 {
            return Err(unresolved());
        }
        let ab = (c[0] + c[1]) * 0.5;
        let bc = (c[1] + c[2]) * 0.5;
        let cd = (c[2] + c[3]) * 0.5;
        let abc = (ab + bc) * 0.5;
        let bcd = (bc + cd) * 0.5;
        let center = (abc + bcd) * 0.5;
        let next = e + 32. * f64::EPSILON * c.iter().map(|x| x.abs()).fold(0., f64::max);
        let mid = a + (b - a) * 0.5;
        if !next.is_finite() || mid == a || mid == b {
            return Err(unresolved());
        }
        stack.push((mid, b, [center, bcd, cd, c[3]], next, depth + 1));
        stack.push((a, mid, [c[0], ab, abc, center], next, depth + 1));
    }
    let mut merged: Vec<(f64, f64, i8)> = Vec::new();
    for (a, b, s) in leaves {
        if let Some(last) = merged.last_mut() {
            if last.2 == s && last.1 == a {
                last.1 = b;
                continue;
            }
        }
        merged.push((a, b, s));
    }
    Ok(merged)
}
impl NurbsTorusSolid {
    pub fn intersect_segment(
        &self,
        start: Point3,
        end: Point3,
        policy: GeometryTolerance,
    ) -> Result<NurbsTorusSegmentIntersection> {
        self.validate(policy)?;
        if !start.finite() || !end.finite() {
            return Err(Error::InvalidInput(
                "torus segment endpoints must be finite",
            ));
        }
        let locations = [
            self.classify_point(start, policy)?,
            self.classify_point(end, policy)?,
        ];
        if locations.contains(&PointLocation::Boundary) {
            return Err(Error::Unsupported(
                "torus segment endpoint lies in the boundary band; move it away",
            ));
        }
        let delta = end - start;
        let length = norm(delta);
        let frame = self.frame();
        let [major, minor] = self.radii();
        let scale = 2. * (major + minor).hypot(minor);
        let band = policy.length_at_scale(scale)?;
        if !delta.finite() || !length.is_finite() || length <= 10. * band {
            return Err(Error::Unsupported(
                "torus segment is too short or unresolved",
            ));
        }
        let axes = frame.axes();
        let mut gram = 0f64;
        for (i, a) in axes.iter().enumerate() {
            for (j, b) in axes.iter().enumerate() {
                gram = gram.max((a.dot(*b) - if i == j { 1. } else { 0. }).abs());
            }
        }
        let world = [start, end, frame.origin()]
            .into_iter()
            .fold(scale, |s, p| s.max(p.x.abs()).max(p.y.abs()).max(p.z.abs()));
        let error =
            65536. * f64::EPSILON * world + 8. * gram * (norm(start - frame.origin()) + length);
        if !error.is_finite() || error >= policy.linear() / 64. {
            return Err(Error::Unsupported(
                "torus segment world or frame precision is unresolved",
            ));
        }
        let local = frame.local_point(start);
        let local_end = frame.local_point(end);
        let d = local_end - local;
        let q = Query {
            body: self,
            start,
            delta,
            local,
            d,
            length,
            band,
            error,
            policy,
        };
        let start_signed = q.signed(0.)?;
        if start_signed.abs() > length + band + 4. * error {
            return Ok(NurbsTorusSegmentIntersection {
                hits: Vec::new(),
                material_intervals: if start_signed < 0. {
                    vec![[0., 1.]]
                } else {
                    Vec::new()
                },
            });
        }
        let s = major
            .max(minor)
            .max(norm(local - Point3::new(0., 0., 0.)))
            .max(norm(d));
        if !s.is_normal() {
            return Err(unresolved());
        }
        let p = Vec3::new(local.x / s, local.y / s, local.z / s);
        let v = Vec3::new(d.x / s, d.y / s, d.z / s);
        let r = major / s;
        let tube = minor / s;
        let a = v.dot(v);
        let b = 2. * p.dot(v);
        let c = p.dot(p) + r * r - tube * tube;
        let rr = 4. * r * r;
        let xy = p.x * v.x + p.y * v.y;
        let powers = [
            2. * b * c - 2. * rr * xy,
            2. * (b * b + 2. * a * c - rr * (v.x * v.x + v.y * v.y)),
            6. * a * b,
            4. * a * a,
        ];
        let operand = (p.dot(p) + r * r + tube * tube + b.abs() + a + rr + xy.abs()).powi(2);
        let coeff_error = 8192. * f64::EPSILON * operand + 64. * error / s * operand;
        if !coeff_error.is_finite() || r == 0. || tube == 0. {
            return Err(unresolved());
        }
        let bern = [
            powers[0],
            powers[0] + powers[1] / 3.,
            powers[0] + 2. * powers[1] / 3. + powers[2] / 3.,
            powers.iter().sum(),
        ];
        let regions = monotonic(
            bern,
            coeff_error,
            (policy.linear() / (64. * length)).max(1. / 65536.),
        )?;
        let mut roots = Vec::new();
        for (a, b, sign) in regions {
            if sign == 0 {
                let mid = a + (b - a) * 0.5;
                if q.signed(mid)?.abs() <= q.band + q.error + q.length * (b - a) * 0.5 {
                    return Err(Error::Unsupported("torus critical-point collar approaches the boundary; tangency is unresolved"));
                }
                continue;
            }
            let fa = q.signed(a)?;
            let fb = q.signed(b)?;
            if fa.abs() <= error || fb.abs() <= error {
                return Err(unresolved());
            }
            if fa.signum() != fb.signum() {
                roots.push(q.root(a, b)?);
            }
        }
        if roots.len() > 4 {
            return Err(unresolved());
        }
        let mut hits = Vec::new();
        let mut intervals = Vec::new();
        let mut inside = locations[0] == PointLocation::Inside;
        let mut entered = if inside { Some(0.) } else { None };
        let mut previous = 0.;
        for t in roots {
            if !(0. ..1.).contains(&t)
                || (t - previous) * length <= 10. * band
                || (1. - t) * length <= 10. * band
            {
                return Err(unresolved());
            }
            let point = q.point(t);
            let query = self.point_query(point, policy)?;
            let boundary = query.boundary.ok_or_else(unresolved)?;
            let crossing = boundary.normal.dot(delta);
            // The actual normal-distance witness is insufficient alone near a
            // shallow crossing. Reserve original absolute travel precision too.
            let directional = crossing.abs() / length - 64. * error / minor.min(major - minor);
            let forward_error = (boundary.distance + error) / directional;
            if !directional.is_finite()
                || directional <= 0.
                || !forward_error.is_finite()
                || forward_error >= policy.linear() / 4.
            {
                return Err(Error::Unsupported(
                    "torus segment forward root precision is unresolved near a shallow crossing",
                ));
            }
            if !crossing.is_finite() || crossing.abs() <= error * 16. || (crossing < 0.) == inside {
                return Err(Error::Unsupported(
                    "torus segment boundary crossing normal is unresolved",
                ));
            }
            if inside {
                intervals.push([entered.take().ok_or_else(unresolved)?, t]);
            } else {
                entered = Some(t);
            }
            inside = !inside;
            previous = t;
            hits.push(NurbsTorusSegmentHit {
                parameter: t,
                point,
                boundary,
            });
        }
        if inside != (locations[1] == PointLocation::Inside) {
            return Err(unresolved());
        }
        if inside {
            intervals.push([entered.ok_or_else(unresolved)?, 1.]);
        }
        if intervals.len() > 2 {
            return Err(unresolved());
        }
        for interval in &intervals {
            if (interval[1] - interval[0]) * length <= 10. * band
                || self.classify_point(q.point((interval[0] + interval[1]) * 0.5), policy)?
                    != PointLocation::Inside
            {
                return Err(unresolved());
            }
        }
        Ok(NurbsTorusSegmentIntersection {
            hits,
            material_intervals: intervals,
        })
    }
}
