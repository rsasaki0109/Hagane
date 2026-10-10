//! Normal circular through bores in structurally certified line/arc prisms.
use crate::*;

#[derive(Clone, Debug)]
pub struct NormalArcLinePrismBore {
    kept: Solid,
    removed: Solid,
    hole_faces: [usize; 4],
    direct_removed_volume: f64,
}
impl NormalArcLinePrismBore {
    pub fn kept(&self) -> &Solid {
        &self.kept
    }
    pub fn removed(&self) -> &Solid {
        &self.removed
    }
    /// Indices of the four actual quarter-cylinder faces in `kept`.
    pub fn hole_faces(&self) -> &[usize; 4] {
        &self.hole_faces
    }
    /// Positive analytic cylinder volume, without subtracting nearly equal totals.
    pub fn direct_removed_volume(&self) -> f64 {
        self.direct_removed_volume
    }
    pub fn into_solids(self) -> (Solid, Solid) {
        (self.kept, self.removed)
    }
}
fn length(v: Vec3) -> f64 {
    v.x.hypot(v.y).hypot(v.z)
}
fn curve_distance(a: &Curve, b: &Curve, reverse: bool) -> Option<f64> {
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
fn retained_curves(source: &Solid, kept: &Solid, budget: f64) -> Result<()> {
    let mut used = vec![false; kept.edges.len()];
    for edge in &source.edges {
        let found = kept.edges.iter().enumerate().find(|(i, e)| {
            !used[*i]
                && [false, true].iter().any(|&reverse| {
                    curve_distance(&edge.curve, &e.curve, reverse)
                        .is_some_and(|d| d.is_finite() && d < budget)
                })
        });
        let Some((i, _)) = found else {
            return Err(Error::Unsupported(
                "bore retained curve reconstruction exceeds tolerance",
            ));
        };
        used[i] = true;
    }
    Ok(())
}
/// Cut a normal circular through bore in a certified simple line/arc extrusion.
/// `center` is a world-space point on the bore axis; its axial coordinate is
/// unrestricted. Existing holes must remain disjoint and strictly contained.
/// Skew extrusion, full-circle rim representations and general booleans are
/// unsupported. Arithmetic guards are engineering checks, not interval proofs.
pub fn bore_normal_arc_line_prism(
    source: &Solid,
    center: Point3,
    radius: f64,
    tolerance: GeometryTolerance,
) -> Result<NormalArcLinePrismBore> {
    source.validate(tolerance.absolute())?;
    if !center.finite() || !radius.is_finite() || radius <= 0. {
        return Err(Error::InvalidInput(
            "bore center and radius must be finite with positive radius",
        ));
    }
    let certified = crate::arc_line_prism_validation::recognize_validated_arc_line_prism(
        source,
        tolerance.absolute(),
    )?;
    let extent = source.bounds().max - source.bounds().min;
    let scale = length(extent);
    let band = tolerance.length_at_scale(scale)?;
    let mut world = scale
        .max(radius)
        .max(center.x.abs())
        .max(center.y.abs())
        .max(center.z.abs());
    let o = certified.frame.origin();
    world = world.max(o.x.abs()).max(o.y.abs()).max(o.z.abs());
    let arithmetic = 4096. * f64::EPSILON * world;
    if !arithmetic.is_finite() || arithmetic >= tolerance.linear() / 256. {
        return Err(Error::Unsupported(
            "bore axis projection precision is unresolved",
        ));
    }
    if radius <= 10. * band + arithmetic {
        return Err(Error::InvalidInput(
            "bore radius is unresolved at caller tolerance",
        ));
    }
    let local = certified.frame.local_point(center);
    if !local.finite() {
        return Err(Error::Unsupported(
            "bore axis projection exceeds finite arithmetic",
        ));
    }
    let total =
        certified.region.outer.len() + certified.region.holes.iter().map(Vec::len).sum::<usize>();
    if certified.region.holes.len() >= 16 || total > 124 {
        return Err(Error::Unsupported(
            "bore supports at most 16 holes and 128 profile segments",
        ));
    }
    let circle: Vec<_> = (0..4)
        .map(|i| PlanarSegment::Arc {
            center: [local.x, local.y],
            radius,
            start_angle: f64::from(i) * std::f64::consts::FRAC_PI_2,
            sweep: std::f64::consts::FRAC_PI_2,
        })
        .collect();
    let mut region = certified.region;
    region.holes.push(circle.clone());
    let direction = certified.frame.axes()[2] * certified.height;
    let effective = Tolerance::new(band)?;
    // The trusted region constructor rejects intersections, contacts, nested
    // holes and holes outside the material; no visual-only hole is generated.
    let kept = extrude_arc_line_region_in_frame(&region, direction, certified.frame, effective)?;
    let removed = extrude_arc_line_region_in_frame(
        &ArcLineRegion {
            origin: Point3::new(0., 0., 0.),
            outer: circle,
            holes: vec![],
        },
        direction,
        certified.frame,
        effective,
    )?;
    kept.validate(tolerance.absolute())?;
    removed.validate(tolerance.absolute())?;
    crate::arc_line_prism_validation::certify_validated_arc_line_prism(
        &kept,
        tolerance.absolute(),
    )?;
    crate::arc_line_prism_validation::certify_validated_arc_line_prism(
        &removed,
        tolerance.absolute(),
    )?;
    retained_curves(source, &kept, tolerance.linear() / 32.)?;
    let direct_removed_volume = std::f64::consts::PI * radius * radius * certified.height;
    let before = source.volume()?;
    let after = kept.volume()?;
    let cut = removed.volume()?;
    let volume_budget = 8192. * f64::EPSILON * before.abs().max(after.abs()).max(cut.abs());
    let cut_budget = 8192. * f64::EPSILON * direct_removed_volume.abs().max(cut.abs());
    if !direct_removed_volume.is_finite()
        || direct_removed_volume <= 0.
        || !before.is_finite()
        || !after.is_finite()
        || !cut.is_finite()
        || !volume_budget.is_finite()
        || after <= 0.
        || cut <= 0.
        || !cut_budget.is_finite()
        || (cut - direct_removed_volume).abs() > cut_budget
        || (after + cut - before).abs() > volume_budget
    {
        return Err(Error::Unsupported("bore volume arithmetic is unresolved"));
    }
    Ok(NormalArcLinePrismBore {
        kept,
        removed,
        hole_faces: [total + 2, total + 3, total + 4, total + 5],
        direct_removed_volume,
    })
}
