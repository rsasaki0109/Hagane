//! Exact source-axis plane partition for the scoped graph-solid family.
use crate::*;
#[derive(Clone, Debug)]
pub struct NurbsGraphPlaneSplit {
    pub negative: NurbsGraphSolid,
    pub positive: NurbsGraphSolid,
    pub section: NurbsFace,
    pub plane: Surface,
    source: NurbsGraphSolid,
    axis: usize,
    parameter: f64,
}
fn section(body: &NurbsGraphSolid, axis: usize, tol: Tolerance) -> Result<NurbsFace> {
    let face = &body.brep().shell.faces[if axis == 0 { 3 } else { 5 }];
    let Surface::Nurbs(surface) = &face.surface else {
        return Err(Error::InvalidTopology(
            "graph partition section must retain a NURBS surface",
        ));
    };
    NurbsFace::new((**surface).clone(), face.orientation, tol)
}
fn plane(source: &NurbsGraphSolid, axis: usize, parameter: f64) -> Surface {
    let frame = source.placement();
    let axes = frame.axes();
    let d = source.dimensions();
    let local = if axis == 0 {
        Point3::new(d[0] * parameter, 0., 0.)
    } else {
        Point3::new(0., d[1] * parameter, 0.)
    };
    Surface::Plane {
        origin: frame.point(local),
        u: axes[if axis == 0 { 1 } else { 2 }],
        v: axes[if axis == 0 { 2 } else { 0 }],
    }
}
fn same_curve(a: &Curve, b: &Curve) -> bool {
    let (Curve::Nurbs(a), Curve::Nurbs(b)) = (a, b) else {
        return false;
    };
    a.degree() == b.degree()
        && a.knots() == b.knots()
        && a.weights() == b.weights()
        && a.control_points() == b.control_points()
}
fn same_surface(a: &Surface, b: &Surface) -> Result<bool> {
    let (Surface::Nurbs(a), Surface::Nurbs(b)) = (a, b) else {
        return Ok(false);
    };
    Ok(a.degrees() == b.degrees()
        && a.control_counts() == b.control_counts()
        && a.knots(0)? == b.knots(0)?
        && a.knots(1)? == b.knots(1)?
        && a.weights() == b.weights()
        && a.control_points() == b.control_points())
}
fn same_section(a: &NurbsFace, b: &NurbsFace) -> Result<bool> {
    if a.face.orientation != b.face.orientation
        || a.face.wires.len() != 1
        || a.face.wires[0].coedges.len() != 4
        || !same_surface(&a.face.surface, &b.face.surface)?
    {
        return Ok(false);
    }
    if a.vertices
        .iter()
        .zip(&b.vertices)
        .any(|(a, b)| a.point != b.point)
        || a.edges
            .iter()
            .zip(&b.edges)
            .any(|(a, b)| a.vertices != b.vertices || !same_curve(&a.curve, &b.curve))
    {
        return Ok(false);
    }
    for (a, b) in a.face.wires[0].coedges.iter().zip(&b.face.wires[0].coedges) {
        let (
            PCurve::Affine {
                origin: a_origin,
                direction: a_direction,
            },
            PCurve::Affine {
                origin: b_origin,
                direction: b_direction,
            },
        ) = (&a.pcurve, &b.pcurve)
        else {
            return Ok(false);
        };
        if a.edge != b.edge
            || a.forward != b.forward
            || a_origin != b_origin
            || a_direction != b_direction
        {
            return Ok(false);
        }
    }
    Ok(true)
}
impl NurbsGraphSolid {
    /// Partition by a plane at an interior original-source U or V coordinate.
    /// This does not classify arbitrary solids or implement a general Boolean.
    pub fn split_uv(
        &self,
        axis: usize,
        parameter: f64,
        tol: Tolerance,
    ) -> Result<NurbsGraphPlaneSplit> {
        self.validate(tol)?;
        if axis > 1 || !parameter.is_finite() {
            return Err(Error::InvalidInput(
                "graph split requires axis 0 or 1 and a finite parameter",
            ));
        }
        let ranges = self.source_domain();
        let [a, b] = ranges[axis];
        let guard = 128. * f64::EPSILON * a.abs().max(b.abs()).max(b - a);
        if parameter - a <= guard || b - parameter <= guard {
            return Err(Error::Unsupported(
                "graph split requires a resolved strictly interior plane",
            ));
        }
        let mut left = ranges;
        left[axis][1] = parameter;
        let mut right = ranges;
        right[axis][0] = parameter;
        let negative = self.trimmed_uv(left, tol)?;
        let positive = self.trimmed_uv(right, tol)?;
        let result = NurbsGraphPlaneSplit {
            section: section(&negative, axis, tol)?,
            plane: plane(self, axis, parameter),
            negative,
            positive,
            source: self.clone(),
            axis,
            parameter,
        };
        result.validate(tol)?;
        Ok(result)
    }
}
impl NurbsGraphPlaneSplit {
    pub fn axis(&self) -> usize {
        self.axis
    }
    pub fn parameter(&self) -> f64 {
        self.parameter
    }
    /// Check retained exact section, placed cutting plane, both canonical bodies,
    /// and source-relative partition metadata. Volume summation has an f64 guard.
    pub fn validate(&self, tol: Tolerance) -> Result<()> {
        self.source.validate(tol)?;
        self.negative.validate(tol)?;
        self.positive.validate(tol)?;
        let bad =
            || Error::InvalidTopology("graph plane partition differs from its source certificate");
        let mut left = self.source.source_domain();
        let mut right = left;
        left[self.axis][1] = self.parameter;
        right[self.axis][0] = self.parameter;
        for (body, ranges) in [(&self.negative, left), (&self.positive, right)] {
            if body.dimensions() != self.source.dimensions()
                || body.bulge() != self.source.bulge()
                || body.placement() != self.source.placement()
                || body.source_domain() != ranges
            {
                return Err(bad());
            }
        }
        let negative_face = &self.negative.brep().shell.faces[if self.axis == 0 { 3 } else { 5 }];
        let positive_face = &self.positive.brep().shell.faces[if self.axis == 0 { 2 } else { 4 }];
        let (Surface::Nurbs(negative_surface), Surface::Nurbs(positive_surface)) =
            (&negative_face.surface, &positive_face.surface)
        else {
            return Err(bad());
        };
        if negative_face.orientation != -positive_face.orientation
            || negative_surface.degrees() != positive_surface.degrees()
            || negative_surface.control_counts() != positive_surface.control_counts()
            || negative_surface.knots(0)? != positive_surface.knots(0)?
            || negative_surface.knots(1)? != positive_surface.knots(1)?
            || negative_surface.weights() != positive_surface.weights()
            || negative_surface
                .control_points()
                .iter()
                .zip(positive_surface.control_points())
                .any(|(a, b)| {
                    let distance = (*a - *b).norm();
                    !distance.is_finite() || distance > tol.linear / 4.
                })
        {
            return Err(Error::InvalidTopology(
                "graph partition cut faces do not coincide with opposite orientations",
            ));
        }
        let expected = section(&self.negative, self.axis, tol)?;
        if !same_section(&self.section, &expected)? {
            return Err(bad());
        }
        self.section.validate_boundary(tol)?;
        let Surface::Plane { origin, u, v } = self.plane else {
            return Err(bad());
        };
        let Surface::Plane {
            origin: expected_origin,
            u: expected_u,
            v: expected_v,
        } = plane(&self.source, self.axis, self.parameter)
        else {
            unreachable!()
        };
        if origin != expected_origin || u != expected_u || v != expected_v || !origin.finite() {
            return Err(bad());
        }
        // Positive rational weights make the signed distance to a plane a
        // convex combination of control distances. Separately reserve an
        // engineering allowance for world-coordinate subtraction/projection.
        let normal = u.cross(v).normalized()?;
        let component_scale = |p: Point3| p.x.abs().max(p.y.abs()).max(p.z.abs());
        let controls_scale = negative_surface
            .control_points()
            .iter()
            .copied()
            .map(component_scale)
            .fold(0., f64::max);
        let arithmetic = 4096. * f64::EPSILON * (controls_scale + component_scale(origin));
        if !arithmetic.is_finite() || arithmetic >= tol.linear / 4. {
            return Err(Error::InvalidInput(
                "graph split world precision cannot certify its cutting plane",
            ));
        }
        for point in negative_surface
            .control_points()
            .iter()
            .chain(positive_surface.control_points())
        {
            let residual = (*point - origin).dot(normal).abs();
            if !residual.is_finite() || residual + arithmetic > tol.linear / 4. {
                return Err(Error::InvalidTopology(
                    "graph split section does not lie on its cutting plane within tolerance",
                ));
            }
        }
        let total = self.source.volume()?;
        let negative = self.negative.volume()?;
        let positive = self.positive.volume()?;
        let sum = negative + positive;
        let guard = 256. * f64::EPSILON * total.abs().max(negative.abs()).max(positive.abs());
        if !sum.is_finite() || !guard.is_finite() || (sum - total).abs() > guard {
            return Err(Error::InvalidInput(
                "graph partition volume conservation is unresolved",
            ));
        }
        Ok(())
    }
}
