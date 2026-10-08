//! Checked ray classification against analytic planar and full-cylinder boundaries.
use crate::*;
enum AnalyticRing {
    Polygon(Vec<[f64; 2]>),
    Circle { center: [f64; 2], radius: f64 },
}
impl AnalyticRing {
    fn location(&self, p: [f64; 2]) -> Result<PointLocation> {
        match self {
            Self::Polygon(ring) => locate_point_in_polygon(p, ring),
            Self::Circle { center, radius } => {
                let r = (p[0] - center[0]).hypot(p[1] - center[1]);
                if !r.is_finite() {
                    return Err(Error::InvalidInput("circle query exceeds finite range"));
                }
                Ok(if r == *radius {
                    PointLocation::Boundary
                } else if r < *radius {
                    PointLocation::Inside
                } else {
                    PointLocation::Outside
                })
            }
        }
    }
    fn distance(&self, p: [f64; 2]) -> Result<f64> {
        match self {
            Self::Circle { center, radius } => {
                let d = ((p[0] - center[0]).hypot(p[1] - center[1]) - radius).abs();
                if !d.is_finite() {
                    return Err(Error::InvalidInput("circle distance exceeds finite range"));
                }
                Ok(d)
            }
            Self::Polygon(ring) => {
                let mut distance = f64::INFINITY;
                for i in 0..ring.len() {
                    distance = distance.min(crate::planar::segment_distance(
                        p,
                        ring[i],
                        ring[(i + 1) % ring.len()],
                    )?);
                }
                Ok(distance)
            }
        }
    }
}
struct PlanarTrim<'a> {
    face: &'a Face,
    normal: Vec3,
    rings: Vec<AnalyticRing>,
}
impl PlanarTrim<'_> {
    fn location(&self, p: [f64; 2]) -> Result<PointLocation> {
        let outer = self.rings[0].location(p)?;
        if outer != PointLocation::Inside {
            return Ok(outer);
        }
        for hole in &self.rings[1..] {
            match hole.location(p)? {
                PointLocation::Inside => return Ok(PointLocation::Outside),
                PointLocation::Boundary => return Ok(PointLocation::Boundary),
                PointLocation::Outside => (),
            }
        }
        Ok(PointLocation::Inside)
    }
    fn boundary_distance(&self, p: [f64; 2]) -> Result<f64> {
        let mut distance = f64::INFINITY;
        for ring in &self.rings {
            distance = distance.min(ring.distance(p)?);
        }
        Ok(distance)
    }
}
struct CylinderTrim<'a> {
    face: &'a Face,
    frame: Frame3,
    radius: f64,
    height: f64,
}
impl CylinderTrim<'_> {
    fn distance(&self, p: Point3) -> Result<f64> {
        let p = self.frame.local_point(p);
        if !p.finite() {
            return Err(Error::InvalidInput(
                "cylinder query exceeds finite local coordinates",
            ));
        }
        let radial = (p.x.hypot(p.y) - self.radius).abs();
        let axial = if p.z < 0. {
            -p.z
        } else if p.z > self.height {
            p.z - self.height
        } else {
            0.
        };
        let d = radial.hypot(axial);
        if !d.is_finite() {
            return Err(Error::InvalidInput(
                "cylinder boundary distance exceeds finite range",
            ));
        }
        Ok(d)
    }
}
/// Classify against validated planar polygon/full-circle trims and complete
/// periodic cylinder walls. Euclidean boundary distance uses a local budget.
/// Two independent resolved rays must agree. Ambiguous/tangent/rim hits retry.
/// Partial cylinders, bounded arcs and general self-intersection detection are
/// unsupported. No display mesh is consulted and periodic seams are not edges
/// of the material region.
pub fn classify_point_in_solid(
    solid: &Solid,
    p: Point3,
    tol: GeometryTolerance,
) -> Result<PointLocation> {
    if !p.finite() {
        return Err(Error::InvalidInput(
            "solid classification requires a finite point",
        ));
    }
    solid.validate(tol.absolute())?;
    let mut faces = Vec::new();
    let mut cylinders = Vec::new();
    for face in &solid.shell.faces {
        match face.surface {
            Surface::Cylinder {
                center,
                radius,
                height,
            } => {
                if face.cylinder_span()? != std::f64::consts::TAU {
                    return Err(Error::Unsupported(
                        "solid classification requires full periodic cylinder walls",
                    ));
                }
                cylinders.push(CylinderTrim {
                    face,
                    frame: Frame3::translation(center)?,
                    radius,
                    height,
                });
            }
            Surface::FramedCylinder {
                frame,
                radius,
                height,
            } => {
                if face.cylinder_span()? != std::f64::consts::TAU {
                    return Err(Error::Unsupported(
                        "solid classification requires full periodic cylinder walls",
                    ));
                }
                cylinders.push(CylinderTrim {
                    face,
                    frame,
                    radius,
                    height,
                });
            }
            Surface::Plane { u, v, .. } => {
                let mut rings = Vec::new();
                for w in &face.wires {
                    if w.coedges.len() == 1 {
                        let c = &w.coedges[0];
                        if let PCurve::Circle { center, radius } = c.pcurve {
                            if matches!(
                                solid.edges[c.edge].curve,
                                Curve::Circle { .. } | Curve::FramedCircle { .. }
                            ) {
                                rings.push(AnalyticRing::Circle { center, radius });
                                continue;
                            }
                        }
                    }
                    let mut ring = Vec::new();
                    for c in &w.coedges {
                        if !matches!(solid.edges[c.edge].curve, Curve::Line { .. })
                            || !matches!(c.pcurve, PCurve::Affine { .. })
                        {
                            return Err(Error::Unsupported("solid classification supports planar polygons and full-circle wires, not bounded arcs"));
                        }
                        ring.push(c.pcurve.evaluate(if c.forward { 0. } else { 1. }));
                    }
                    rings.push(AnalyticRing::Polygon(ring));
                }
                faces.push(PlanarTrim {
                    face,
                    normal: u.cross(v),
                    rings,
                });
            }
        }
    }
    let bounds = solid.bounds();
    let scale = (bounds.max - bounds.min).norm();
    let budget = tol.length_at_scale(scale)?;
    if p.x < bounds.min.x - budget
        || p.x > bounds.max.x + budget
        || p.y < bounds.min.y - budget
        || p.y > bounds.max.y + budget
        || p.z < bounds.min.z - budget
        || p.z > bounds.max.z + budget
    {
        return Ok(PointLocation::Outside);
    }
    for f in &faces {
        let Surface::Plane { origin, .. } = f.face.surface else {
            unreachable!()
        };
        let distance = (p - origin).dot(f.normal).abs();
        if distance <= budget {
            let uv = f.face.surface.parameters(p);
            let trim = f.location(uv)?;
            let lateral = if trim == PointLocation::Inside {
                0.0
            } else {
                f.boundary_distance(uv)?
            };
            if distance.hypot(lateral) <= budget {
                return Ok(PointLocation::Boundary);
            }
        }
    }
    for cylinder in &cylinders {
        if cylinder.distance(p)? <= budget {
            return Ok(PointLocation::Boundary);
        }
    }
    let directions = [
        [1., 2., 3.],
        [-2., 3., 5.],
        [3., -5., 7.],
        [5., 7., -11.],
        [-7., -11., 13.],
        [11., -13., -17.],
        [13., 17., 19.],
        [-17., 19., 23.],
        [19., -23., 29.],
        [23., 29., -31.],
        [-29., -31., 37.],
        [31., -37., -41.],
    ];
    let mut answer = None;
    let mut accepted = 0;
    for d in directions {
        let d = Vec3::new(d[0], d[1], d[2]).normalized()?;
        let mut hits = Vec::new();
        let mut ambiguous = false;
        for f in &faces {
            let Surface::Plane { origin, .. } = f.face.surface else {
                unreachable!()
            };
            let denominator = d.dot(f.normal);
            if denominator.abs() <= tol.angular().sin() {
                ambiguous = true;
                break;
            }
            let t = (origin - p).dot(f.normal) / denominator;
            if !t.is_finite() {
                return Err(Error::Unsupported(
                    "ray plane parameter exceeds finite range",
                ));
            }
            if t <= 0.0 {
                continue;
            }
            let q = p + d * t;
            let uv = f.face.surface.parameters(q);
            if !q.finite()
                || !tol
                    .absolute()
                    .coincident(q, f.face.surface.evaluate(uv[0], uv[1]))
            {
                ambiguous = true;
                break;
            }
            let location = f.location(uv)?;
            if location == PointLocation::Boundary || f.boundary_distance(uv)? <= budget {
                ambiguous = true;
                break;
            }
            if location == PointLocation::Inside {
                if t <= budget {
                    ambiguous = true;
                    break;
                }
                hits.push((t, (denominator * f.face.orientation as f64).signum() as i32));
            }
        }
        if !ambiguous {
            let ray_tol = GeometryTolerance::new(budget, tol.angular(), 0.)?;
            for cylinder in &cylinders {
                match intersect_line_cylinder(p, d, &cylinder.face.surface, ray_tol) {
                    Ok(LineCylinderIntersection::Empty) => (),
                    Ok(LineCylinderIntersection::Points(points)) => {
                        for hit in points {
                            if hit.parameter <= 0. {
                                continue;
                            }
                            let denominator = d.dot(cylinder.face.surface.normal(hit.uv[0]));
                            if hit.contact == IntersectionContact::Tangent
                                || denominator.abs() <= tol.angular().sin()
                                || hit.uv[1].min(cylinder.height - hit.uv[1]) <= budget
                                || hit.parameter <= budget
                            {
                                ambiguous = true;
                                break;
                            }
                            hits.push((
                                hit.parameter,
                                (denominator * cylinder.face.orientation as f64).signum() as i32,
                            ));
                        }
                    }
                    Ok(LineCylinderIntersection::Coincident { .. }) | Err(_) => {
                        ambiguous = true;
                    }
                }
                if ambiguous {
                    break;
                }
            }
        }
        if ambiguous {
            continue;
        }
        hits.sort_by(|a, b| a.0.total_cmp(&b.0));
        if hits.windows(2).any(|h| h[1].0 - h[0].0 <= budget) {
            continue;
        }
        let inside = hits.len() % 2 == 1;
        // The ray must alternate entry/exit, ending at infinity outside.
        if hits
            .iter()
            .enumerate()
            .any(|(i, h)| h.1 != if (i % 2 == 0) == inside { 1 } else { -1 })
        {
            return Err(Error::Unsupported(
                "solid ray crossings have inconsistent orientation",
            ));
        }
        let location = if inside {
            PointLocation::Inside
        } else {
            PointLocation::Outside
        };
        if answer.is_some_and(|a| a != location) {
            return Err(Error::Unsupported(
                "independent solid classification rays disagree",
            ));
        }
        answer = Some(location);
        accepted += 1;
        if accepted == 2 {
            return Ok(location);
        }
    }
    Err(Error::Unsupported(
        "no two resolved solid classification rays",
    ))
}
/// Native/WASM fixture: a concave extrusion with a polygon through-hole.
pub fn classification_demo_json(x: f64, y: f64, z: f64) -> Result<String> {
    let solid = classification_demo_solid()?;
    let location =
        classify_point_in_solid(&solid, Point3::new(x, y, z), GeometryTolerance::default())?;
    let name = match location {
        PointLocation::Inside => "inside",
        PointLocation::Outside => "outside",
        PointLocation::Boundary => "boundary",
    };
    Ok(format!(
        "{{\"location\":\"{name}\",\"volume\":{}}}",
        solid.volume()?
    ))
}
pub(crate) fn classification_demo_solid() -> Result<Solid> {
    extrude_polygon(
        &PolygonProfile {
            origin: Point3::new(0., 0., -12.),
            outer: vec![
                [-40., -30.],
                [40., -30.],
                [40., -5.],
                [-5., -5.],
                [-5., 30.],
                [-40., 30.],
            ],
            holes: vec![vec![[-32., -16.], [-20., -16.], [-20., 12.], [-32., 12.]]],
        },
        Vec3::new(0., 0., 24.),
        Tolerance::default(),
    )
}
/// Exact B-rep display fixture for the point classifier demo.
pub fn classification_mesh_demo_json() -> Result<String> {
    classification_demo_solid()?.mesh_json(0.05, Tolerance::default())
}
pub(crate) fn curved_classification_solid(model: u32) -> Result<Solid> {
    let t = Tolerance::default();
    match model {
        0 => subtract_through_cylinder(
            BoxSpec {
                min: Point3::new(-40., -30., -12.),
                size: Vec3::new(80., 60., 24.),
            },
            CylinderSpec {
                base: Point3::new(0., 0., -20.),
                radius: 14.,
                height: 40.,
            },
            t,
        ),
        1 => make_cylinder(
            CylinderSpec {
                base: Point3::new(0., 0., -12.),
                radius: 24.,
                height: 24.,
            },
            t,
        ),
        2 => make_tube(
            TubeSpec {
                base: Point3::new(0., 0., -12.),
                outer_radius: 24.,
                inner_radius: 14.,
                height: 24.,
            },
            t,
        ),
        _ => Err(Error::InvalidInput("unknown curved classification model")),
    }
}
pub fn curved_classification_demo_json(model: u32, x: f64, y: f64, z: f64) -> Result<String> {
    let s = curved_classification_solid(model)?;
    let location = classify_point_in_solid(&s, Point3::new(x, y, z), GeometryTolerance::default())?;
    let name = match location {
        PointLocation::Inside => "inside",
        PointLocation::Outside => "outside",
        PointLocation::Boundary => "boundary",
    };
    Ok(format!(
        "{{\"location\":\"{name}\",\"volume\":{}}}",
        s.volume()?
    ))
}
pub fn curved_classification_mesh_json(model: u32) -> Result<String> {
    curved_classification_solid(model)?.mesh_json(0.05, Tolerance::default())
}
