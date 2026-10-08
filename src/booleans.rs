//! Scoped convex operand intersection/difference/union using checked half-space clipping.
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
            "convex Boolean operations support at most 128 faces per operand",
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
/// Curved/nonconvex operands, contacts and coplanar overlays are unsupported.
pub fn intersect_convex_solids(
    first: &Solid,
    second: &Solid,
    tol: GeometryTolerance,
) -> Result<SolidIntersection> {
    let (common, _) = convex_partition(first, second, tol)?;
    Ok(match common {
        Some(s) => SolidIntersection::Solid(s),
        None => SolidIntersection::Empty,
    })
}
fn convex_partition(
    first: &Solid,
    second: &Solid,
    tol: GeometryTolerance,
) -> Result<(Option<Solid>, Vec<Solid>)> {
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
        return Ok((None, vec![first.clone()]));
    }
    let mut result = first.clone();
    let mut outside = Vec::new();
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
            return Err(Error::Unsupported("convex clipping requires cuts clear of current vertices; contact/coplanar arrangements are unsupported"));
        }
        if distances.iter().all(|&d| d > 0.) {
            outside.push(result);
            return Ok((None, outside));
        }
        if distances.iter().any(|&d| d > 0.) {
            let split = split_solid_by_plane(&result, plane, tol)?;
            outside.push(split.positive);
            result = split.negative;
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
    Ok((Some(result), outside))
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

#[derive(Clone, Debug)]
pub enum SolidDifference {
    Empty,
    Solid(Solid),
}
fn original_plane(surface: &Surface, source: &Solid) -> bool {
    source
        .shell
        .faces
        .iter()
        .any(|f| match (&f.surface, surface) {
            (
                Surface::Plane {
                    origin: a,
                    u: b,
                    v: c,
                },
                Surface::Plane {
                    origin: d,
                    u: e,
                    v: g,
                },
            ) => a == d && b == e && c == g,
            _ => false,
        })
}
/// Subtract checked convex planar operands and sew the retained outer boundary.
/// Results may be nonconvex or have through-holes. One closed connected shell
/// is required; enclosed cavities and disconnected material are unsupported.
/// Contacts/coplanar/near-contact cases follow convex intersection's contract.
pub fn subtract_convex_solids(
    first: &Solid,
    second: &Solid,
    tol: GeometryTolerance,
) -> Result<SolidDifference> {
    let (common, outside) = convex_partition(first, second, tol)?;
    let Some(common) = common else {
        return Ok(SolidDifference::Solid(first.clone()));
    };
    if outside.is_empty() {
        return Ok(SolidDifference::Empty);
    }
    let mut retained = Vec::new();
    for piece in outside {
        retained.extend(
            planar_face_patches(&piece, tol.absolute())?
                .into_iter()
                .filter(|p| original_plane(&p.surface, first)),
        );
    }
    for mut patch in planar_face_patches(&common, tol.absolute())? {
        if !original_plane(&patch.surface, first) {
            patch.orientation *= -1;
            retained.push(patch);
        }
    }
    let solid = crate::sewing::sew_generated_planar_faces(&retained, tol)?;
    if (solid.volume()? + common.volume()? - first.volume()?).abs() > first.volume()?.abs() * 1e-10
    {
        return Err(Error::InvalidTopology(
            "convex difference does not conserve removed volume",
        ));
    }
    Ok(SolidDifference::Solid(solid))
}
pub(crate) fn convex_difference_demo(offset: f64) -> Result<SolidDifference> {
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
            min: Point3::new(offset - 10., -8., -20.),
            size: Vec3::new(20., 16., 40.),
        },
        t.absolute(),
    )?;
    subtract_convex_solids(&a, &b, t)
}
pub fn convex_difference_demo_json(offset: f64) -> Result<String> {
    match convex_difference_demo(offset)? {
        SolidDifference::Empty => Ok("{\"kind\":\"empty\"}".into()),
        SolidDifference::Solid(s) => Ok(format!(
            "{{\"kind\":\"solid\",\"mesh\":{}}}",
            s.mesh_json(0.05, Tolerance::default())?
        )),
    }
}

/// Union checked convex planar straight-edge operands into one closed shell.
/// Nonconvex output is supported; contacts, coplanar boundaries and disjoint
/// operands are unsupported. Strict containment returns the enclosing operand.
pub fn union_convex_solids(first: &Solid, second: &Solid, tol: GeometryTolerance) -> Result<Solid> {
    let (common, first_outside) = convex_partition(first, second, tol)?;
    let Some(common) = common else {
        return Err(Error::Unsupported(
            "union requires overlapping operands; disconnected solids are unsupported",
        ));
    };
    if first_outside.is_empty() {
        return Ok(second.clone());
    }
    let (reverse_common, second_outside) = convex_partition(second, first, tol)?;
    let Some(reverse_common) = reverse_common else {
        return Err(Error::InvalidTopology(
            "union operand partitions disagree on overlap",
        ));
    };
    let expected = first.volume()? + second.volume()? - common.volume()?;
    if (common.volume()? - reverse_common.volume()?).abs() > expected.abs() * 1e-10 {
        return Err(Error::InvalidTopology(
            "union operand partitions disagree on volume",
        ));
    }
    if second_outside.is_empty() {
        return Ok(first.clone());
    }
    let mut retained = Vec::new();
    for (pieces, source) in [(first_outside, first), (second_outside, second)] {
        for piece in pieces {
            retained.extend(
                planar_face_patches(&piece, tol.absolute())?
                    .into_iter()
                    .filter(|p| original_plane(&p.surface, source)),
            );
        }
    }
    let result = crate::sewing::sew_generated_planar_faces(&retained, tol)?;
    if (result.volume()? - expected).abs() > expected.abs() * 1e-10 {
        return Err(Error::InvalidTopology(
            "union does not conserve combined volume",
        ));
    }
    Ok(result)
}
pub(crate) fn convex_union_demo(offset: f64) -> Result<Solid> {
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
            min: Point3::new(10. + offset, -10., -4.),
            size: Vec3::new(50., 50., 32.),
        },
        t.absolute(),
    )?;
    union_convex_solids(&a, &b, t)
}
pub fn convex_union_demo_json(offset: f64) -> Result<String> {
    convex_union_demo(offset)?.mesh_json(0.05, Tolerance::default())
}
