//! Exact harmonic height bands on circular translation surfaces.
use crate::*;
use std::f64::consts::{PI, TAU};
pub(crate) fn surface_data(surface: &Surface) -> Result<(Frame3, f64, f64, [f64; 2])> {
    match *surface {
        Surface::Cylinder {
            center,
            radius,
            height,
        } => Ok((Frame3::translation(center)?, radius, height, [0., 0.])),
        Surface::FramedCylinder {
            frame,
            radius,
            height,
        } => Ok((frame, radius, height, [0., 0.])),
        Surface::ExtrudedCircle {
            frame,
            radius,
            height,
            drift,
        } => Ok((frame, radius, height, drift)),
        _ => Err(Error::Unsupported(
            "height bands require a circular surface",
        )),
    }
}
pub(crate) fn value(coeff: [f64; 3], u: f64) -> f64 {
    coeff[0] + coeff[1] * u.cos() + coeff[2] * u.sin()
}
pub(crate) fn extrema(coeff: [f64; 3], span: f64) -> Result<(f64, f64)> {
    if coeff.iter().any(|x| !x.is_finite()) || !span.is_finite() || span <= 0. || span > TAU {
        return Err(Error::InvalidInput("invalid harmonic height band"));
    }
    let angle = coeff[2].atan2(coeff[1]).rem_euclid(TAU);
    let mut lo = value(coeff, 0.).min(value(coeff, span));
    let mut hi = value(coeff, 0.).max(value(coeff, span));
    for u in [angle, (angle + PI).rem_euclid(TAU)] {
        if u <= span {
            let v = value(coeff, u);
            lo = lo.min(v);
            hi = hi.max(v);
        }
    }
    if !lo.is_finite() || !hi.is_finite() {
        return Err(Error::InvalidInput("height band exceeds finite range"));
    }
    Ok((lo, hi))
}
impl Face {
    pub(crate) fn circular_bands(&self) -> Result<[[f64; 3]; 2]> {
        let span = self.circular_span()?;
        let coefficients = |c: &Coedge| -> Result<[f64; 3]> {
            match c.pcurve {
                PCurve::Affine { origin, direction }
                    if origin[0] == 0. && direction == [1., 0.] =>
                {
                    Ok([origin[1], 0., 0.])
                }
                PCurve::HeightGraph {
                    offset,
                    cosine,
                    sine,
                    sweep,
                } if sweep == span => Ok([offset, cosine, sine]),
                _ => Err(Error::Unsupported(
                    "circular rims require angular affine or harmonic height pcurves",
                )),
            }
        };
        Ok([
            coefficients(&self.wires[0].coedges[0])?,
            coefficients(&self.wires[0].coedges[2])?,
        ])
    }
}
pub(crate) fn validate_face(face: &Face, tol: Tolerance) -> Result<()> {
    let span = face.circular_span()?;
    let bands = face.circular_bands()?;
    let (_, _, height, _) = surface_data(&face.surface)?;
    let difference = std::array::from_fn(|i| bands[1][i] - bands[0][i]);
    if extrema(bands[0], span)?.0 < -tol.linear
        || extrema(bands[1], span)?.1 > height + tol.linear
        || extrema(difference, span)?.0 <= tol.linear
    {
        return Err(Error::Unsupported(
            "circular height bands leave the surface or overlap",
        ));
    }
    let c = &face.wires[0].coedges;
    if !c[0].forward || c[2].forward {
        return Err(Error::Unsupported("circular rim traversal is inconsistent"));
    }
    for (index, u, forward) in [(1, span, true), (3, 0., false)] {
        let PCurve::Affine { origin, direction } = c[index].pcurve else {
            return Err(Error::Unsupported(
                "circular band generators require affine pcurves",
            ));
        };
        let start = value(bands[0], u);
        let end = value(bands[1], u);
        if c[index].forward != forward
            || (origin[0] - u).abs() > tol.linear
            || (origin[1] - start).abs() > tol.linear
            || direction[0].abs() > tol.linear
            || (direction[1] - (end - start)).abs() > tol.linear
        {
            return Err(Error::Unsupported(
                "circular height-band generator endpoints disagree",
            ));
        }
    }
    Ok(())
}
pub(crate) fn volume_term(face: &Face, reference: Point3) -> Result<f64> {
    let (frame, radius, _, drift) = surface_data(&face.surface)?;
    let span = face.circular_span()?;
    let bands = face.circular_bands()?;
    let d: [f64; 3] = std::array::from_fn(|i| bands[1][i] - bands[0][i]);
    let delta = frame.local_point(reference) * (-1.);
    let a = delta.x - drift[0] * delta.z;
    let b = delta.y - drift[1] * delta.z;
    let (ic, is, icc, iss, ics) = if span == TAU {
        (0., 0., span / 2., span / 2., 0.)
    } else {
        (
            span.sin(),
            1. - span.cos(),
            span / 2. + (2. * span).sin() / 4.,
            span / 2. - (2. * span).sin() / 4.,
            span.sin().powi(2) / 2.,
        )
    };
    Ok(radius / 3.
        * (radius * (d[0] * span + d[1] * ic + d[2] * is)
            + a * (d[0] * ic + d[1] * icc + d[2] * ics)
            + b * (d[0] * is + d[1] * ics + d[2] * iss)))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn individual_band_flux_matches_independent_surface_quadrature() {
        let solid = oblique_boundary_demo(0.13, 0.7).unwrap();
        let reference = solid.vertices[0].point;
        for face in &solid.shell.faces {
            if !face
                .wires
                .iter()
                .flat_map(|w| &w.coedges)
                .any(|c| matches!(c.pcurve, PCurve::HeightGraph { .. }))
            {
                continue;
            }
            let (frame, radius, _, drift) = surface_data(&face.surface).unwrap();
            let bands = face.circular_bands().unwrap();
            let span = face.circular_span().unwrap();
            let samples = 4096;
            let integrand = |u| {
                let lower = value(bands[0], u);
                let upper = value(bands[1], u);
                let p = face.surface.evaluate(u, (lower + upper) / 2.);
                let area_normal = frame.vector(Vec3::new(
                    radius * u.cos(),
                    radius * u.sin(),
                    -radius * (drift[0] * u.cos() + drift[1] * u.sin()),
                ));
                (p - reference).dot(area_normal) * (upper - lower) / 3.
            };
            let sum = (0..=samples)
                .map(|i| {
                    let weight = if i == 0 || i == samples {
                        1.
                    } else if i % 2 == 0 {
                        2.
                    } else {
                        4.
                    };
                    weight * integrand(span * i as f64 / samples as f64)
                })
                .sum::<f64>()
                * span
                / (3. * samples as f64);
            let analytic = volume_term(face, reference).unwrap();
            assert!((analytic - sum).abs() <= sum.abs().max(1.) * 1e-10);
        }
    }
}
