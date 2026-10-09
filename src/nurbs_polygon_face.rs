//! Convex straight-UV trimmed open faces with retained exact boundary geometry.
use crate::*;

#[derive(Clone, Debug)]
pub struct NurbsPolygonFace {
    pub boundary: NurbsSurfaceWire,
    /// Face wires reference the edges/vertices in `boundary`.
    pub face: Face,
}
impl NurbsPolygonFace {
    /// Outer UV wire must be counterclockwise, independently of face orientation.
    pub fn new(
        surface: NurbsSurface,
        corners: Vec<[f64; 2]>,
        orientation: i8,
        tol: Tolerance,
    ) -> Result<Self> {
        let boundary = NurbsSurfaceWire::new(surface.clone(), corners, tol)?;
        let face = Face {
            surface: Surface::Nurbs(Box::new(surface)),
            wires: vec![boundary.wire.clone()],
            orientation,
        };
        let result = Self { boundary, face };
        result.validate(tol)?;
        Ok(result)
    }
    pub fn validate(&self, tol: Tolerance) -> Result<()> {
        self.boundary.validate(tol)?;
        if self.boundary.orientation()? != Orientation::CounterClockwise
            || !matches!(self.face.orientation, -1 | 1)
        {
            return Err(Error::InvalidTopology(
                "polygon face requires CCW outer UV wire and signed orientation",
            ));
        }
        let Surface::Nurbs(surface) = &self.face.surface else {
            return Err(Error::InvalidTopology(
                "polygon face requires its retained NURBS surface",
            ));
        };
        let original = &self.boundary.surface;
        if surface.degrees() != original.degrees()
            || surface.control_points() != original.control_points()
            || surface.weights() != original.weights()
            || surface.knots(0)? != original.knots(0)?
            || surface.knots(1)? != original.knots(1)?
        {
            return Err(Error::InvalidTopology(
                "polygon face supporting surfaces disagree",
            ));
        }
        if self.face.wires.len() != 1
            || self.face.wires[0].coedges.len() != self.boundary.wire.coedges.len()
        {
            return Err(Error::InvalidTopology("polygon face outer wire disagrees"));
        }
        for (a, b) in self.face.wires[0]
            .coedges
            .iter()
            .zip(&self.boundary.wire.coedges)
        {
            let (
                PCurve::Affine {
                    origin: ao,
                    direction: ad,
                },
                PCurve::Affine {
                    origin: bo,
                    direction: bd,
                },
            ) = (&a.pcurve, &b.pcurve)
            else {
                return Err(Error::InvalidTopology(
                    "polygon face requires affine pcurves",
                ));
            };
            if a.edge != b.edge || a.forward != b.forward || ao != bo || ad != bd {
                return Err(Error::InvalidTopology("polygon face coedge disagrees"));
            }
        }
        Ok(())
    }
    /// Initial interior display supports only one equal-weight bilinear patch
    /// whose control net is a parallelogram within floating arithmetic.
    pub fn tessellate_affine(&self, error: f64, tol: Tolerance) -> Result<Mesh> {
        self.validate(tol)?;
        if !error.is_finite() || error <= 0. {
            return Err(Error::InvalidInput(
                "polygon display needs positive finite error",
            ));
        }
        let s = &self.boundary.surface;
        if s.degrees() != [1, 1]
            || s.control_points().len() != 4
            || s.weights().iter().any(|w| *w != s.weights()[0])
        {
            return Err(Error::Unsupported(
                "polygon interior display currently requires an affine bilinear patch",
            ));
        }
        let p = s.control_points();
        let twist = (p[3] - p[2]) - (p[1] - p[0]);
        let scale = p
            .iter()
            .flat_map(|p| [p.x.abs(), p.y.abs(), p.z.abs()])
            .fold(f64::MIN_POSITIVE, f64::max);
        let arithmetic = 4096. * f64::EPSILON * scale;
        if !twist.finite() || twist.norm() > arithmetic {
            return Err(Error::Unsupported(
                "non-affine polygon interior display is not implemented",
            ));
        }
        // Affine interpolation cancels exactly in real arithmetic; two bounded
        // twist contributions and a world-coordinate roundoff reserve remain.
        if !arithmetic.is_finite() || 2. * twist.norm() + arithmetic > error {
            return Err(Error::Tessellation(
                "polygon display cannot resolve requested precision",
            ));
        }
        let mut positions = Vec::new();
        let mut normals = Vec::new();
        for uv in self.boundary.uv_corners() {
            let evaluation = s.evaluate_with_partials(uv[0], uv[1], [KnotSide::Right; 2])?;
            positions.push(evaluation.point);
            normals.push(
                evaluation.du.cross(evaluation.dv).normalized()? * self.face.orientation as f64,
            );
        }
        let mut triangles = Vec::new();
        for i in 1..positions.len() - 1 {
            let mut triangle = [0, i, i + 1];
            if self.face.orientation < 0 {
                triangle.swap(1, 2);
            }
            let cross = (positions[triangle[1]] - positions[0])
                .cross(positions[triangle[2]] - positions[0]);
            if !cross.finite() || cross.norm() == 0. {
                return Err(Error::Tessellation(
                    "polygon display triangle is numerically degenerate",
                ));
            }
            triangles.push(triangle);
        }
        let face_ids = vec![0; triangles.len()];
        Ok(Mesh {
            positions,
            normals,
            triangles,
            face_ids,
        })
    }
}
