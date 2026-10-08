//! Checked ray classification against exact planar B-rep boundaries.
use crate::*;
struct PlanarTrim<'a> {
    face: &'a Face,
    normal: Vec3,
    rings: Vec<Vec<[f64; 2]>>,
}
impl PlanarTrim<'_> {
    fn location(&self, p: [f64; 2]) -> Result<PointLocation> {
        let outer = locate_point_in_polygon(p, &self.rings[0])?;
        if outer != PointLocation::Inside {
            return Ok(outer);
        }
        for hole in &self.rings[1..] {
            match locate_point_in_polygon(p, hole)? {
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
            for i in 0..ring.len() {
                distance = distance.min(crate::planar::segment_distance(
                    p,
                    ring[i],
                    ring[(i + 1) % ring.len()],
                )?);
            }
        }
        Ok(distance)
    }
}
/// Classify a finite point against a validated all-planar, straight-edge solid.
/// Boundary uses Euclidean distance to trimmed faces and a local length budget.
/// Two independent nondegenerate rays must agree; ambiguous hits retry, and
/// exhausted or inconsistent rays return errors. No display mesh is consulted.
/// Curved faces/edges and general self-intersection detection are unsupported.
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
    for face in &solid.shell.faces {
        let Surface::Plane { u, v, .. } = face.surface else {
            return Err(Error::Unsupported(
                "solid classification requires planar faces",
            ));
        };
        let mut rings = Vec::new();
        for w in &face.wires {
            let mut ring = Vec::new();
            for c in &w.coedges {
                if !matches!(solid.edges[c.edge].curve, Curve::Line { .. }) {
                    return Err(Error::Unsupported(
                        "solid classification requires straight boundaries",
                    ));
                }
                ring.push(c.pcurve.evaluate(if c.forward { 0.0 } else { 1.0 }));
            }
            rings.push(ring);
        }
        faces.push(PlanarTrim {
            face,
            normal: u.cross(v),
            rings,
        });
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
