//! Scoped solid intersection using checked convex half-space clipping.
use crate::*;
#[derive(Clone, Debug)]
pub enum SolidIntersection {
    Empty,
    Solid(Solid),
}
fn convex_planes(solid: &Solid, tol: GeometryTolerance) -> Result<Vec<Surface>> {
    let patches = planar_face_patches(solid, tol.absolute())?;
    if patches.len() > 128 {
        return Err(Error::Unsupported(
            "convex intersection supports at most 128 faces per operand",
        ));
    }
    let mut planes = Vec::new();
    for patch in patches {
        if patch.rings.len() != 1 {
            return Err(Error::Unsupported("convex operands cannot have face holes"));
        }
        let ring: Vec<_> = patch.rings[0]
            .iter()
            .map(|&p| patch.surface.parameters(p))
            .collect();
        for i in 0..ring.len() {
            if orient2d(
                ring[i],
                ring[(i + 1) % ring.len()],
                ring[(i + 2) % ring.len()],
            )? == Orientation::Clockwise
            {
                return Err(Error::Unsupported(
                    "convex operands require convex face rings",
                ));
            }
        }
        let Surface::Plane { origin, u, v } = patch.surface else {
            unreachable!()
        };
        let v = v * patch.orientation as f64;
        let normal = u.cross(v);
        for vertex in &solid.vertices {
            let delta = vertex.point - origin;
            let d = delta.dot(normal);
            let roundoff = (64.0 * f64::EPSILON * delta.norm()).min(tol.linear());
            if !d.is_finite() || d > roundoff {
                return Err(Error::Unsupported(
                    "operand is not convex under its outward supporting planes",
                ));
            }
        }
        planes.push(Surface::Plane { origin, u, v });
    }
    Ok(planes)
}
/// Intersect two validated convex planar straight-edge solids.
/// Proper cuts must clear all current vertices by ten local length budgets.
/// Contact/coplanar/near-contact arrangements are rejected. Strict containment
/// legitimately returns the contained solid; strict separation returns Empty.
/// Curved/nonconvex operands and general union/difference are unsupported.
pub fn intersect_convex_solids(
    first: &Solid,
    second: &Solid,
    tol: GeometryTolerance,
) -> Result<SolidIntersection> {
    convex_planes(first, tol)?;
    let planes = convex_planes(second, tol)?;
    let a = first.bounds();
    let b = second.bounds();
    let scale = (a.max - a.min).norm().max((b.max - b.min).norm());
    let budget = tol.length_at_scale(scale)?;
    if a.max.x < b.min.x - 10.0 * budget
        || b.max.x < a.min.x - 10.0 * budget
        || a.max.y < b.min.y - 10.0 * budget
        || b.max.y < a.min.y - 10.0 * budget
        || a.max.z < b.min.z - 10.0 * budget
        || b.max.z < a.min.z - 10.0 * budget
    {
        return Ok(SolidIntersection::Empty);
    }
    let mut result = first.clone();
    for plane in &planes {
        let Surface::Plane { origin, u, v } = *plane else {
            unreachable!()
        };
        let normal = u.cross(v);
        let distances: Vec<_> = result
            .vertices
            .iter()
            .map(|p| (p.point - origin).dot(normal))
            .collect();
        if distances
            .iter()
            .any(|d| !d.is_finite() || d.abs() <= 10.0 * budget)
        {
            return Err(Error::Unsupported("convex intersection requires cuts clear of current vertices; contact/coplanar arrangements are unsupported"));
        }
        if distances.iter().all(|&d| d > 0.) {
            return Ok(SolidIntersection::Empty);
        }
        if distances.iter().any(|&d| d > 0.) {
            result = split_solid_by_plane(&result, plane, tol)?.negative;
        }
    }
    result.validate(tol.absolute())?;
    for plane in &planes {
        let Surface::Plane { origin, u, v } = *plane else {
            unreachable!()
        };
        if result
            .vertices
            .iter()
            .any(|p| (p.point - origin).dot(u.cross(v)) > budget)
        {
            return Err(Error::InvalidTopology(
                "intersection leaves material outside cutter half-spaces",
            ));
        }
    }
    if result.volume()? > first.volume()?.min(second.volume()?) * (1.0 + 1e-10) {
        return Err(Error::InvalidTopology(
            "intersection volume exceeds an operand",
        ));
    }
    Ok(SolidIntersection::Solid(result))
}
pub(crate) fn convex_intersection_demo(offset: f64) -> Result<SolidIntersection> {
    let t = GeometryTolerance::default();
    let a = make_box(
        BoxSpec {
            min: Point3::new(-40., -30., -12.),
            size: Vec3::new(80., 60., 24.),
        },
        t.absolute(),
    )?;
    let b = make_box(
        BoxSpec {
            min: Point3::new(-32., -32., -20.),
            size: Vec3::new(64., 64., 40.),
        },
        t.absolute(),
    )?
    .transformed(
        Transform::translation(Vec3::new(offset, 0., 0.))?.compose(Transform::rotation(
            Vec3::new(0., 0., 1.),
            std::f64::consts::FRAC_PI_4,
        )?)?,
        t.absolute(),
    )?;
    intersect_convex_solids(&a, &b, t)
}
pub fn convex_intersection_demo_json(offset: f64) -> Result<String> {
    match convex_intersection_demo(offset)? {
        SolidIntersection::Empty => Ok("{\"kind\":\"empty\"}".into()),
        SolidIntersection::Solid(s) => Ok(format!(
            "{{\"kind\":\"solid\",\"mesh\":{}}}",
            s.mesh_json(0.05, Tolerance::default())?
        )),
    }
}
