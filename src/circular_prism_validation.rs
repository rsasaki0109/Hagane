//! Bounded geometric certificate for complete round cylinders and concentric tubes.
use crate::*;
use std::collections::BTreeSet;
/// A certified circular translation, independent of original modeling intent.
#[derive(Debug, Clone, PartialEq)]
pub struct CircularPrismCertificate {
    pub axis: Frame3,
    pub height: f64,
    pub outer_radius: f64,
    pub inner_radius: Option<f64>,
}
const DOMAIN: Error =
    Error::Unsupported("solid is not a certified complete cylinder or concentric tube");
/// Validate topology plus complete concentric cap/wall correspondence.
/// Geometry agreement uses bounded arithmetic roundoff; no snapping or repair.
pub fn certify_circular_prism(s: &Solid, t: Tolerance) -> Result<CircularPrismCertificate> {
    s.validate(t)?;
    certify_validated_circular_prism(s, t)
}
pub(crate) fn certify_validated_circular_prism(
    s: &Solid,
    t: Tolerance,
) -> Result<CircularPrismCertificate> {
    let planes: Vec<_> = s
        .shell
        .faces
        .iter()
        .filter(|f| matches!(f.surface, Surface::Plane { .. }))
        .collect();
    let walls: Vec<_> = s
        .shell
        .faces
        .iter()
        .filter(|f| !matches!(f.surface, Surface::Plane { .. }))
        .collect();
    if planes.len() != 2
        || !(1..=2).contains(&walls.len())
        || s.vertices.len() != 2 * walls.len()
        || s.edges.len() != 3 * walls.len()
    {
        return Err(DOMAIN);
    }
    let angular = 64. * f64::EPSILON;
    let diagonal = (s.bounds().max - s.bounds().min).norm();
    let budget = (64. * f64::EPSILON * diagonal).min(t.linear / 1024.);
    if !diagonal.is_finite() || budget <= 0. || !s.volume()?.is_finite() {
        return Err(DOMAIN);
    }
    let outer = walls.iter().find(|f| f.orientation == 1).ok_or(DOMAIN)?;
    let (axis, outer_radius, height, _) = crate::circular_trims::surface_data(&outer.surface)?;
    if !matches!(
        outer.surface,
        Surface::Cylinder { .. } | Surface::FramedCylinder { .. }
    ) || height <= 10. * t.linear
    {
        return Err(DOMAIN);
    }
    let mut rims = BTreeSet::new();
    let mut inner_radius = None;
    for wall in &walls {
        let (frame, radius, h, _) = crate::circular_trims::surface_data(&wall.surface)?;
        if !matches!(
            wall.surface,
            Surface::Cylinder { .. } | Surface::FramedCylinder { .. }
        ) || (frame.origin() - axis.origin()).norm() > budget
            || (h - height).abs() > budget
            || frame
                .axes()
                .iter()
                .zip(axis.axes())
                .any(|(a, b)| (*a - b).norm() > angular)
            || wall.cylinder_span()? != std::f64::consts::TAU
        {
            return Err(DOMAIN);
        }
        if wall.orientation == -1 {
            if inner_radius.replace(radius).is_some() || outer_radius - radius <= 10. * t.linear {
                return Err(DOMAIN);
            }
        } else if !std::ptr::eq(*wall, *outer) {
            return Err(DOMAIN);
        }
        let c = &wall.wires[0].coedges;
        if c[1].edge != c[3].edge {
            return Err(DOMAIN);
        }
        let line = &s.edges[c[1].edge];
        let Curve::Line { a, b } = line.curve else {
            return Err(DOMAIN);
        };
        if (a - s.vertices[line.vertices[0]].point).norm() > budget
            || (b - s.vertices[line.vertices[1]].point).norm() > budget
        {
            return Err(DOMAIN);
        }
        for (index, z) in [(0, 0.), (2, height)] {
            let edge = &s.edges[c[index].edge];
            let (center, axes, r) = match edge.curve {
                Curve::Circle { center, radius } => (center, Frame3::IDENTITY.axes(), radius),
                Curve::FramedCircle { frame, radius } => (frame.origin(), frame.axes(), radius),
                _ => return Err(DOMAIN),
            };
            if edge.vertices[0] != edge.vertices[1]
                || (r - radius).abs() > budget
                || (center - axis.point(Vec3::new(0., 0., z))).norm() > budget
                || axes
                    .iter()
                    .zip(axis.axes())
                    .any(|(a, b)| (*a - b).norm() > angular)
                || !rims.insert(c[index].edge)
            {
                return Err(DOMAIN);
            }
            let vertex = s.vertices[edge.vertices[0]].point;
            if (vertex - axis.point(Vec3::new(radius, 0., z))).norm() > budget
                || line.vertices[if index == 0 { 0 } else { 1 }] != edge.vertices[0]
            {
                return Err(DOMAIN);
            }
        }
        for seam in [1, 3] {
            let PCurve::Affine { origin, direction } = c[seam].pcurve else {
                return Err(DOMAIN);
            };
            if origin[0] != if seam == 1 { std::f64::consts::TAU } else { 0. }
                || origin[1] != 0.
                || direction[0] != 0.
                || (direction[1] - height).abs() > budget
            {
                return Err(DOMAIN);
            }
        }
    }
    let mut cap_rims = BTreeSet::new();
    let mut lower = false;
    let mut upper = false;
    for cap in planes {
        let Surface::Plane { origin, u, v } = cap.surface else {
            return Err(DOMAIN);
        };
        let local = axis.local_point(origin);
        if (u - axis.axes()[0]).norm() > angular || (v - axis.axes()[1]).norm() > angular {
            return Err(DOMAIN);
        }

        let outward = u.cross(v).normalized()? * f64::from(cap.orientation);
        let sign = outward.dot(axis.axes()[2]);
        if outward.cross(axis.axes()[2]).norm() > angular || cap.wires.len() != walls.len() {
            return Err(DOMAIN);
        }
        let z = if sign < 0. {
            if lower {
                return Err(DOMAIN);
            }
            lower = true;
            0.
        } else {
            if upper {
                return Err(DOMAIN);
            }
            upper = true;
            height
        };
        if !local.finite() || (local.z - z).abs() > budget {
            return Err(DOMAIN);
        }
        for (i, wire) in cap.wires.iter().enumerate() {
            if wire.coedges.len() != 1 {
                return Err(DOMAIN);
            }
            let edge = wire.coedges[0].edge;
            if !rims.contains(&edge) || !cap_rims.insert(edge) {
                return Err(DOMAIN);
            }
            let (center, radius) = match s.edges[edge].curve {
                Curve::Circle { center, radius } => (center, radius),
                Curve::FramedCircle { frame, radius } => (frame.origin(), radius),
                _ => return Err(DOMAIN),
            };

            let PCurve::Circle {
                center: uv,
                radius: uv_radius,
            } = wire.coedges[0].pcurve
            else {
                return Err(DOMAIN);
            };
            let expected_uv = cap.surface.parameters(center);
            if (uv[0] - expected_uv[0]).hypot(uv[1] - expected_uv[1]) > budget
                || (uv_radius - radius).abs() > budget
            {
                return Err(DOMAIN);
            }
            let expected = if i == 0 {
                outer_radius
            } else {
                inner_radius.ok_or(DOMAIN)?
            };
            if (radius - expected).abs() > budget
                || (center - axis.point(Vec3::new(0., 0., z))).norm() > budget
            {
                return Err(DOMAIN);
            }
        }
    }
    if !lower || !upper || cap_rims != rims {
        return Err(DOMAIN);
    }
    Ok(CircularPrismCertificate {
        axis,
        height,
        outer_radius,
        inner_radius,
    })
}
