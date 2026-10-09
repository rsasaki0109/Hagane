//! Scoped closed graph partition by a source-vertical affine-UV line plane.
use crate::*;
#[derive(Clone, Debug)]
pub struct NurbsGraphPolygonSplit {
    pub negative: NurbsGraphPolygonSolid,
    pub positive: NurbsGraphPolygonSolid,
    pub section: NurbsFace,
    pub plane: Surface,
    source: NurbsGraphSolid,
    polygon: Vec<[f64; 2]>,
    line: [[f64; 2]; 2],
}
fn unsupported() -> Error {
    Error::Unsupported(
        "graph line split requires a resolved strict two-sided cut without vertex contact",
    )
}
fn build(
    source: &NurbsGraphSolid,
    polygon: &[[f64; 2]],
    start: [f64; 2],
    end: [f64; 2],
    tol: Tolerance,
) -> Result<NurbsGraphPolygonSplit> {
    source.validate(tol)?;
    Tolerance::new(tol.linear)?;
    if !start.iter().chain(end.iter()).all(|x| x.is_finite()) {
        return Err(Error::InvalidInput("split line endpoints must be finite"));
    }
    let [l, w, _] = source.dimensions();
    let delta = [end[0] - start[0], end[1] - start[1]];
    let physical = Vec3::new(l * delta[0], w * delta[1], 0.);
    let length = physical.norm();
    let margin = 4. * tol.linear + source.arithmetic_budget()?;
    if !physical.finite() || !length.is_finite() || length <= margin {
        return Err(unsupported());
    }
    for a in 0..2 {
        if delta[a] != 0.
            && delta[a].abs() <= 128. * f64::EPSILON * start[a].abs().max(end[a].abs()).max(1.)
        {
            return Err(unsupported());
        }
    }
    let line_scale = (l * start[0])
        .abs()
        .max((w * start[1]).abs())
        .max((l * end[0]).abs())
        .max((w * end[1]).abs());
    let line_guard = 4096. * f64::EPSILON * line_scale * 8.;
    if !line_guard.is_finite() || line_guard >= tol.linear / 4. {
        return Err(unsupported());
    }
    let normal = Vec3::new(-physical.y / length, physical.x / length, 0.);
    let mut distances = Vec::new();
    for p in polygon {
        let distance = l * (p[0] - start[0]) * normal.x + w * (p[1] - start[1]) * normal.y;
        if !distance.is_finite() || distance.abs() <= margin + line_guard {
            return Err(unsupported());
        }
        distances.push(distance);
    }
    if !distances.iter().any(|d| *d < 0.) || !distances.iter().any(|d| *d > 0.) {
        return Err(unsupported());
    }
    let mut negative = Vec::new();
    let mut positive = Vec::new();
    let mut crossings = Vec::new();
    for i in 0..polygon.len() {
        let j = (i + 1) % polygon.len();
        let p = polygon[i];
        if distances[i] < 0. {
            negative.push(p);
        } else {
            positive.push(p);
        }
        if (distances[i] < 0.) != (distances[j] < 0.) {
            let fraction = distances[i] / (distances[i] - distances[j]);
            if !fraction.is_finite() || fraction <= 0. || fraction >= 1. {
                return Err(unsupported());
            }
            let point =
                std::array::from_fn::<_, 2, _>(|a| p[a] + fraction * (polygon[j][a] - p[a]));
            if point == p || point == polygon[j] || point.iter().any(|x| !x.is_finite()) {
                return Err(unsupported());
            }
            negative.push(point);
            positive.push(point);
            crossings.push(point);
        }
    }
    if crossings.len() != 2 || negative.len() > 16 || positive.len() > 16 {
        return Err(unsupported());
    }
    let negative = NurbsGraphPolygonSolid::new(source, negative, tol)?;
    let positive = NurbsGraphPolygonSolid::new(source, positive, tol)?;
    let cut_index = |body: &NurbsGraphPolygonSolid| -> Result<usize> {
        let p = body.polygon();
        (0..p.len())
            .find(|i| crossings.contains(&p[*i]) && crossings.contains(&p[(*i + 1) % p.len()]))
            .ok_or(Error::InvalidTopology("split cut edge missing"))
    };
    let ni = cut_index(&negative)?;
    let pi = cut_index(&positive)?;
    let Surface::Nurbs(ns) = &negative.brep().shell.faces[2 + ni].surface else {
        return Err(unsupported());
    };
    let Surface::Nurbs(ps) = &positive.brep().shell.faces[2 + pi].surface else {
        return Err(unsupported());
    };
    if ns.degrees() != ps.degrees()
        || ns.control_counts() != ps.control_counts()
        || ns.knots(0)? != ps.knots(0)?
        || ns.knots(1)? != ps.knots(1)?
    {
        return Err(Error::InvalidTopology("split cut bases disagree"));
    }
    let [nu, nv] = ns.control_counts();
    for i in 0..nu {
        for j in 0..nv {
            let a = i * nv + j;
            let b = (nu - 1 - i) * nv + j;
            if ns.control_points()[a] != ps.control_points()[b]
                || ns.weights()[a] != ps.weights()[b]
            {
                return Err(Error::InvalidTopology(
                    "split cut control nets do not share exact reversed geometry",
                ));
            }
        }
    }
    let section = NurbsFace::new((**ns).clone(), 1, tol)?;
    let placement = source.placement();
    let tangent = physical * (1. / length);
    let plane = Surface::Plane {
        origin: placement.point(Point3::new(l * start[0], w * start[1], 0.)),
        u: placement.vector(Vec3::new(0., 0., 1.)),
        v: placement.vector(tangent),
    };
    let Surface::Plane { origin, u, v } = &plane else {
        unreachable!()
    };
    let plane_normal = u.cross(*v);
    let world_scale = ns
        .control_points()
        .iter()
        .map(|p| p.x.abs().max(p.y.abs()).max(p.z.abs()))
        .fold(
            origin.x.abs().max(origin.y.abs()).max(origin.z.abs()),
            f64::max,
        );
    let guard = source
        .arithmetic_budget()?
        .max(4096. * f64::EPSILON * world_scale * 8.);
    for point in ns.control_points() {
        let residual = (*point - *origin).dot(plane_normal).abs();
        if !residual.is_finite() || residual + guard > tol.linear / 4. {
            return Err(unsupported());
        }
    }
    let original = NurbsGraphPolygonSolid::new(source, polygon.to_vec(), tol)?.volume()?;
    let combined = negative.volume()? + positive.volume()?;
    if !combined.is_finite() || (combined - original).abs() > 8192. * f64::EPSILON * original {
        return Err(Error::InvalidInput(
            "split volume conservation is unresolved",
        ));
    }
    Ok(NurbsGraphPolygonSplit {
        negative,
        positive,
        section,
        plane,
        source: source.clone(),
        polygon: polygon.to_vec(),
        line: [start, end],
    })
}
impl NurbsGraphSolid {
    pub fn split_uv_line(
        &self,
        start: [f64; 2],
        end: [f64; 2],
        tol: Tolerance,
    ) -> Result<NurbsGraphPolygonSplit> {
        let [u, v] = self.source_domain();
        build(
            self,
            &[[u[0], v[0]], [u[1], v[0]], [u[1], v[1]], [u[0], v[1]]],
            start,
            end,
            tol,
        )
    }
}
impl NurbsGraphPolygonSolid {
    pub fn split_uv_line(
        &self,
        start: [f64; 2],
        end: [f64; 2],
        tol: Tolerance,
    ) -> Result<NurbsGraphPolygonSplit> {
        self.validate(tol)?;
        build(self.source(), self.polygon(), start, end, tol)
    }
}
fn equal_surface(a: &Surface, b: &Surface) -> Result<bool> {
    match (a, b) {
        (Surface::Nurbs(a), Surface::Nurbs(b)) => Ok(a.degrees() == b.degrees()
            && a.control_counts() == b.control_counts()
            && a.control_points() == b.control_points()
            && a.weights() == b.weights()
            && a.knots(0)? == b.knots(0)?
            && a.knots(1)? == b.knots(1)?),
        _ => Ok(false),
    }
}
fn equal_curve(a: &Curve, b: &Curve) -> bool {
    match (a, b) {
        (Curve::Nurbs(a), Curve::Nurbs(b)) => {
            a.degree() == b.degree()
                && a.knots() == b.knots()
                && a.weights() == b.weights()
                && a.control_points() == b.control_points()
        }
        _ => false,
    }
}
impl NurbsGraphPolygonSplit {
    pub fn source(&self) -> &NurbsGraphSolid {
        &self.source
    }
    pub fn polygon(&self) -> &[[f64; 2]] {
        &self.polygon
    }
    pub fn line(&self) -> [[f64; 2]; 2] {
        self.line
    }
    pub fn validate(&self, tol: Tolerance) -> Result<()> {
        self.negative.validate(tol)?;
        self.positive.validate(tol)?;
        let expected = build(&self.source, &self.polygon, self.line[0], self.line[1], tol)?;
        for (actual, wanted) in [
            (&self.negative, &expected.negative),
            (&self.positive, &expected.positive),
        ] {
            if actual.polygon() != wanted.polygon()
                || actual.source().dimensions() != wanted.source().dimensions()
                || actual.source().bulge() != wanted.source().bulge()
                || actual.source().placement() != wanted.source().placement()
                || actual.source().source_domain() != wanted.source().source_domain()
            {
                return Err(Error::InvalidTopology("split body certificate changed"));
            }
        }
        let a = &self.section;
        let b = &expected.section;
        if a.face.orientation != b.face.orientation
            || !equal_surface(&a.face.surface, &b.face.surface)?
            || a.face.wires.len() != 1
            || a.face.wires[0].coedges.len() != 4
        {
            return Err(Error::InvalidTopology("split section changed"));
        }
        for (a, b) in a.vertices.iter().zip(&b.vertices) {
            if a.point != b.point {
                return Err(Error::InvalidTopology("split section vertex changed"));
            }
        }
        for (a, b) in a.edges.iter().zip(&b.edges) {
            if a.vertices != b.vertices || !equal_curve(&a.curve, &b.curve) {
                return Err(Error::InvalidTopology("split section edge changed"));
            }
        }
        for (a, b) in a.face.wires[0].coedges.iter().zip(&b.face.wires[0].coedges) {
            let same = matches!((&a.pcurve,&b.pcurve),(PCurve::Affine{origin:a,direction:da},PCurve::Affine{origin:b,direction:db})if a==b&&da==db);
            if a.edge != b.edge || a.forward != b.forward || !same {
                return Err(Error::InvalidTopology("split section pcurve changed"));
            }
        }
        let same = matches!((&self.plane,&expected.plane),(Surface::Plane{origin:a,u:au,v:av},Surface::Plane{origin:b,u:bu,v:bv})if a==b&&au==bu&&av==bv);
        if !same {
            return Err(Error::InvalidTopology("split plane changed"));
        }
        Ok(())
    }
}
