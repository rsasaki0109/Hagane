//! Scoped plane partitions of certified normal line/arc prisms.
use crate::*;
#[derive(Clone, Debug)]
pub struct NormalArcLinePrismPlaneSplit {
    negative: Solid,
    positive: Solid,
    section: PlanarFacePatch,
    plane: Surface,
}
impl NormalArcLinePrismPlaneSplit {
    pub fn negative(&self) -> &Solid {
        &self.negative
    }
    pub fn positive(&self) -> &Solid {
        &self.positive
    }
    /// One actual rectangular section, oriented along the input plane normal.
    pub fn section(&self) -> &PlanarFacePatch {
        &self.section
    }
    pub fn plane(&self) -> &Surface {
        &self.plane
    }
    pub fn into_solids(self) -> (Solid, Solid) {
        (self.negative, self.positive)
    }
}
fn length(v: Vec3) -> f64 {
    v.x.hypot(v.y).hypot(v.z)
}
fn curve_gap(a: &Curve, b: &Curve, reverse: bool) -> Option<f64> {
    match (a, b) {
        (Curve::Line { a, b }, Curve::Line { a: c, b: d }) => {
            let (c, d) = if reverse { (*d, *c) } else { (*c, *d) };
            Some(length(*a - c).max(length(*b - d)))
        }
        (
            Curve::Arc {
                frame: a,
                radius: ra,
                sweep: sa,
            },
            Curve::Arc {
                frame: b,
                radius: rb,
                sweep: sb,
            },
        ) => {
            let aa = a.axes();
            let bb = b.axes();
            let (u, v) = if reverse {
                let (s, c) = sb.sin_cos();
                (bb[0] * c + bb[1] * s, bb[0] * s - bb[1] * c)
            } else {
                (bb[0], bb[1])
            };
            Some(
                length(a.origin() - b.origin())
                    + (ra - rb).abs()
                    + ra.max(*rb) * (length(aa[0] - u) + length(aa[1] - v) + (sa - sb).abs()),
            )
        }
        _ => None,
    }
}
fn line_covered(a: Point3, b: Point3, children: &[Solid; 2], budget: f64) -> Result<bool> {
    let extent = b - a;
    let total = length(extent);
    if !total.is_finite() || total <= 0. {
        return Ok(false);
    }
    let unit = extent * (1. / total);
    let mut ranges = Vec::new();
    for edge in children.iter().flat_map(|s| &s.edges) {
        if let Curve::Line { a: c, b: d } = edge.curve {
            let tc = (c - a).dot(unit);
            let td = (d - a).dot(unit);
            if !tc.is_finite() || !td.is_finite() {
                continue;
            }
            if length(c - (a + unit * tc)) < budget && length(d - (a + unit * td)) < budget {
                let lower = tc.min(td).max(0.);
                let upper = tc.max(td).min(total);
                if lower < upper {
                    ranges.push([lower, upper]);
                }
            }
        }
    }
    ranges.sort_by(|a, b| a[0].total_cmp(&b[0]));
    let mut covered = 0.;
    for [lower, upper] in ranges {
        if lower > covered + budget {
            return Ok(false);
        }
        covered = covered.max(upper);
    }
    Ok(covered + budget >= total)
}
fn arc_covered(
    frame: Frame3,
    radius: f64,
    sweep: f64,
    children: &[Solid; 2],
    budget: f64,
) -> Result<bool> {
    let axes = frame.axes();
    let mut ranges = Vec::new();
    for edge in children.iter().flat_map(|s| &s.edges) {
        let Curve::Arc {
            frame: child,
            radius: r,
            sweep: span,
        } = edge.curve
        else {
            continue;
        };
        let child_axes = child.axes();
        let normal_dot = axes[2].dot(child_axes[2]);
        let sign = if normal_dot >= 0. { 1. } else { -1. };
        let phase = child_axes[0].dot(axes[1]).atan2(child_axes[0].dot(axes[0]));
        for shift in [-std::f64::consts::TAU, 0., std::f64::consts::TAU] {
            let alpha = phase + shift;
            let (sin, cos) = alpha.sin_cos();
            let expected_u = axes[0] * cos + axes[1] * sin;
            let expected_v = (axes[1] * cos - axes[0] * sin) * sign;
            let gap = length(frame.origin() - child.origin())
                + (radius - r).abs()
                + radius.max(r)
                    * (length(expected_u - child_axes[0]) + length(expected_v - child_axes[1]));
            if !gap.is_finite() || gap >= budget {
                continue;
            }
            let end = alpha + sign * span;
            let lower = alpha.min(end);
            let upper = alpha.max(end);
            if lower * radius < -budget || (upper - sweep) * radius > budget {
                continue;
            }
            ranges.push([lower.max(0.) * radius, upper.min(sweep) * radius]);
        }
    }
    ranges.sort_by(|a, b| a[0].total_cmp(&b[0]));
    let mut covered = 0.;
    for [lower, upper] in ranges {
        if lower > covered + budget {
            return Ok(false);
        }
        covered = covered.max(upper);
    }
    Ok(covered + budget >= sweep * radius)
}
fn retains_original_edges(source: &Solid, children: &[Solid; 2], budget: f64) -> Result<()> {
    for edge in &source.edges {
        let covered = match edge.curve {
            Curve::Line { a, b } => line_covered(a, b, children, budget)?,
            Curve::Arc {
                frame,
                radius,
                sweep,
            } => arc_covered(frame, radius, sweep, children, budget)?,
            _ => false,
        };
        if !covered {
            return Err(Error::Unsupported(
                "prism partition original curve restriction exceeds tolerance",
            ));
        }
    }
    Ok(())
}
fn retains_refined_edges(refined: &Solid, children: &[Solid; 2], budget: f64) -> Result<()> {
    for edge in &refined.edges {
        if let Curve::Line { a, b } = edge.curve {
            if line_covered(a, b, children, budget)? {
                continue;
            }
        }
        if !children.iter().flat_map(|s| &s.edges).any(|e| {
            [false, true].iter().any(|&reverse| {
                curve_gap(&edge.curve, &e.curve, reverse)
                    .is_some_and(|d| d.is_finite() && d < budget)
            })
        }) {
            return Err(Error::Unsupported(
                "prism split retained curve reconstruction exceeds tolerance",
            ));
        }
    }
    Ok(())
}
fn certify_child(s: &Solid, t: Tolerance) -> Result<()> {
    s.validate(t)?;
    if s.edges.iter().any(|e| matches!(e.curve, Curve::Arc { .. })) {
        crate::arc_line_prism_validation::certify_validated_arc_line_prism(s, t)
    } else {
        crate::prism_validation::certify_validated_planar_prism(s, t).map(|_| ())
    }
}
/// Split a certified normal line/arc prism by a plane parallel to its extrusion.
/// Requires exactly one interior interval, two distinct outer-edge crossings,
/// and no crossed inner wire. Contacts, tangent/vertex hits and meaningful skew
/// reject. Tiny rigid-frame arithmetic drift is bounded at the caller tolerance;
/// these conservative guards are engineering checks, not interval proofs.
pub fn split_normal_arc_line_prism_by_plane(
    source: &Solid,
    plane: &Surface,
    tolerance: GeometryTolerance,
) -> Result<NormalArcLinePrismPlaneSplit> {
    source.validate(tolerance.absolute())?;
    let certified = crate::arc_line_prism_validation::recognize_validated_arc_line_prism(
        source,
        tolerance.absolute(),
    )?;
    let Surface::Plane { origin, u, v } = *plane else {
        return Err(Error::InvalidInput("prism partition requires a plane"));
    };
    if !origin.finite() || !crate::geometry::plane_basis_valid(u, v, tolerance) {
        return Err(Error::InvalidInput("invalid prism partition plane basis"));
    }
    let normal = u.cross(v).normalized()?;
    let axis = certified.frame.axes()[2];
    let extent = source.bounds().max - source.bounds().min;
    let scale = length(extent);
    let band = tolerance.length_at_scale(scale)?;
    let mut world = scale
        .max(origin.x.abs())
        .max(origin.y.abs())
        .max(origin.z.abs());
    for vertex in &source.vertices {
        let p = vertex.point;
        world = world.max(p.x.abs()).max(p.y.abs()).max(p.z.abs());
    }
    for face in &source.shell.faces {
        let p = match face.surface {
            Surface::Plane { origin, .. } => origin,
            Surface::FramedCylinder { frame, .. } => frame.origin(),
            Surface::Cylinder { center, .. } => center,
            _ => return Err(Error::Unsupported("unsupported prism partition surface")),
        };
        world = world.max(p.x.abs()).max(p.y.abs()).max(p.z.abs());
    }
    for edge in &source.edges {
        if let Curve::Arc { frame, .. } = edge.curve {
            let p = frame.origin();
            world = world.max(p.x.abs()).max(p.y.abs()).max(p.z.abs());
        }
    }
    let arithmetic = 4096. * f64::EPSILON * world;
    let drift = normal.dot(axis).abs() * scale;
    if !arithmetic.is_finite()
        || !drift.is_finite()
        || arithmetic + drift >= tolerance.linear() / 256.
        || normal.dot(axis).abs() > 128. * f64::EPSILON
    {
        return Err(Error::Unsupported(
            "prism partition plane alignment or world precision is unresolved",
        ));
    }
    let base_origin = certified.frame.origin();
    let bases: Vec<_> = source
        .shell
        .faces
        .iter()
        .enumerate()
        .filter(|(_, f)| match f.surface {
            Surface::Plane { origin, u, v } => {
                length(origin - base_origin) < tolerance.linear() / 32.
                    && (u.cross(v) * f64::from(f.orientation)).dot(axis) < -0.99
            }
            _ => false,
        })
        .map(|(i, _)| i)
        .collect();
    if bases.len() != 1 {
        return Err(Error::Unsupported("prism partition base cap is ambiguous"));
    }
    let projected_normal = normal - axis * normal.dot(axis);
    let offset = (base_origin - origin).dot(normal);
    let anchor = base_origin - projected_normal * (offset / projected_normal.dot(projected_normal));
    let direction = axis.cross(normal).normalized()?;
    if !anchor.finite() || !offset.is_finite() {
        return Err(Error::Unsupported(
            "prism partition base intersection exceeds finite arithmetic",
        ));
    }
    let effective = GeometryTolerance::new(band, tolerance.angular(), 0.)?;
    let split = split_planar_face(source, bases[0], anchor, direction, effective)?;
    let delta = axis * certified.height;
    let mut children = Vec::new();
    for index in split.faces {
        let face = &split.solid.shell.faces[index];
        let mut rings = crate::face_intersections::rings(face)?;
        // Source cap UV may be reflected relative to the certified right-handed frame.
        if face.orientation == 1 {
            for s in rings.iter_mut().flatten() {
                match s {
                    PlanarSegment::Line { a, b } => {
                        a[1] = -a[1];
                        b[1] = -b[1];
                    }
                    PlanarSegment::Arc {
                        center,
                        start_angle,
                        sweep,
                        ..
                    } => {
                        center[1] = -center[1];
                        *start_angle = -*start_angle;
                        *sweep = -*sweep;
                    }
                }
            }
        }
        let outer = rings.remove(0);
        let profile = ArcLineRegion {
            origin: Point3::new(0., 0., 0.),
            outer,
            holes: rings,
        };
        let loops: Vec<_> = std::iter::once(&profile.outer)
            .chain(&profile.holes)
            .cloned()
            .collect();
        crate::mixed::validate_mixed_region(&loops, Tolerance::new(band)?)?;
        let area = face
            .wires
            .iter()
            .map(crate::topology::try_wire_area)
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .sum::<f64>()
            .abs();
        let volume = area * certified.height;
        if !volume.is_finite() || volume <= 0. {
            return Err(Error::Unsupported(
                "prism partition child measure is unresolved",
            ));
        }
        let child = extrude_arc_line_region_in_frame(
            &profile,
            delta,
            certified.frame,
            tolerance.absolute(),
        )?;
        certify_child(&child, tolerance.absolute())?;
        let actual = child.volume()?;
        if !actual.is_finite()
            || actual <= 0.
            || (actual - volume).abs() > 8192. * f64::EPSILON * actual.abs().max(volume)
        {
            return Err(Error::Unsupported(
                "prism partition child volume arithmetic is unresolved",
            ));
        }
        children.push(child);
    }
    let children: [Solid; 2] = children
        .try_into()
        .map_err(|_| Error::InvalidTopology("missing partition child"))?;
    retains_refined_edges(&split.solid, &children, tolerance.linear() / 64.)?;
    // Basis mismatch plus interval coverage each reserve half of Tol/32.
    retains_original_edges(source, &children, tolerance.linear() / 64.)?;
    let original = source.volume()?;
    let combined = children[0].volume()? + children[1].volume()?;
    if !original.is_finite()
        || !combined.is_finite()
        || (combined - original).abs() > 8192. * f64::EPSILON * original.abs().max(combined.abs())
    {
        return Err(Error::Unsupported(
            "prism partition volume conservation is unresolved",
        ));
    }
    let a = split.solid.vertices[split.cut_vertices[0]].point;
    let b = split.solid.vertices[split.cut_vertices[1]].point;
    let mut points = [a, b, b + delta, a + delta];
    if (b - a).cross(delta).dot(normal) < 0. {
        points.reverse();
    }
    if points.iter().any(|p| {
        !p.finite()
            || ((*p - origin).dot(normal)).abs() + arithmetic + drift >= tolerance.linear() / 32.
    }) {
        return Err(Error::Unsupported(
            "prism partition section plane residual is unresolved",
        ));
    }
    let signs: Vec<_> = children
        .iter()
        .map(|child| {
            let gaps: Vec<_> = child
                .vertices
                .iter()
                .map(|p| (p.point - origin).dot(normal))
                .collect();
            let negative = gaps.iter().any(|d| *d < -10. * band);
            let positive = gaps.iter().any(|d| *d > 10. * band);
            match (negative, positive) {
                (true, false) => Ok(-1),
                (false, true) => Ok(1),
                _ => Err(Error::Unsupported(
                    "prism partition side ownership is unresolved",
                )),
            }
        })
        .collect::<Result<_>>()?;
    if signs[0] == signs[1] {
        return Err(Error::InvalidTopology(
            "prism partition children lie on one side",
        ));
    }
    let [a, b] = children;
    let (negative, positive) = if signs[0] < 0 { (a, b) } else { (b, a) };
    let sections: Vec<_> = negative
        .shell
        .faces
        .iter()
        .filter_map(|face| {
            let Surface::Plane { u, v, .. } = face.surface else {
                return None;
            };
            if face.wires.len() != 1
                || face.wires[0].coedges.len() != 4
                || (u.cross(v) * f64::from(face.orientation)).dot(normal) < 0.99
            {
                return None;
            }
            let actual: Vec<_> = face.wires[0]
                .coedges
                .iter()
                .map(|c| {
                    negative.vertices[negative.edges[c.edge].vertices[usize::from(!c.forward)]]
                        .point
                })
                .collect();
            if !actual.iter().all(|a| {
                points
                    .iter()
                    .any(|b| length(*a - *b) < tolerance.linear() / 32.)
            }) {
                return None;
            }
            Some(PlanarFacePatch {
                surface: face.surface.clone(),
                orientation: face.orientation,
                rings: vec![actual],
            })
        })
        .collect();
    if sections.len() != 1 {
        return Err(Error::Unsupported(
            "prism partition actual section face is ambiguous",
        ));
    }
    let section = sections
        .into_iter()
        .next()
        .ok_or(Error::InvalidTopology("missing partition section"))?;
    if section.rings[0].iter().any(|p| {
        !p.finite()
            || ((*p - origin).dot(normal)).abs() + arithmetic + drift >= tolerance.linear() / 32.
    }) {
        return Err(Error::Unsupported(
            "prism partition actual section residual is unresolved",
        ));
    }
    Ok(NormalArcLinePrismPlaneSplit {
        negative,
        positive,
        section,
        plane: plane.clone(),
    })
}
