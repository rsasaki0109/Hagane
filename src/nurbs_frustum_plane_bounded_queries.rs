//! Actual finite-face queries reuse checked retained-body engines.
use crate::nurbs_frustum_slab_classification::classify;
use crate::nurbs_frustum_slab_intersection::{intersect, Geometry};
use crate::*;

impl NurbsFrustumPlaneBoundedSolid {
    pub fn classify_point(
        &self,
        point: Point3,
        policy: GeometryTolerance,
    ) -> Result<PointLocation> {
        self.validate(policy)?;
        classify(
            point,
            policy,
            self.source(),
            self.solid(),
            [
                self.lower_section().is_some(),
                self.upper_section().is_some(),
            ],
            |p| self.source().classify_point(p, policy),
        )
    }

    pub fn intersect_segment(
        &self,
        start: Point3,
        end: Point3,
        policy: GeometryTolerance,
    ) -> Result<NurbsFrustumSegmentIntersection> {
        self.validate(policy)?;
        let result = intersect(
            start,
            end,
            policy,
            Geometry {
                canonical: self.source(),
                solid: self.solid(),
                present: [
                    self.lower_section().is_some(),
                    self.upper_section().is_some(),
                ],
            },
            |p| self.source().classify_point(p, policy),
            || {
                let result = self.source().intersect_segment(start, end, policy)?;
                Ok(NurbsBoredFrustumSegmentIntersection {
                    hits: result.hits,
                    material_intervals: result.material_interval.into_iter().collect(),
                })
            },
            |p| self.classify_point(p, policy),
        )?;
        if result.material_intervals.len() > 1 {
            return Err(Error::Unsupported(
                "plane-bounded convex frustum material interval correspondence is unresolved",
            ));
        }
        Ok(NurbsFrustumSegmentIntersection {
            hits: result.hits,
            material_interval: result.material_intervals.into_iter().next(),
        })
    }
}

impl NurbsBoredFrustumPlaneBoundedSolid {
    pub fn classify_point(
        &self,
        point: Point3,
        policy: GeometryTolerance,
    ) -> Result<PointLocation> {
        self.validate(policy)?;
        classify(
            point,
            policy,
            self.source().source(),
            self.solid(),
            [
                self.outer().lower_section().is_some(),
                self.outer().upper_section().is_some(),
            ],
            |p| self.source().classify_point(p, policy),
        )
    }

    pub fn intersect_segment(
        &self,
        start: Point3,
        end: Point3,
        policy: GeometryTolerance,
    ) -> Result<NurbsBoredFrustumSegmentIntersection> {
        self.validate(policy)?;
        intersect(
            start,
            end,
            policy,
            Geometry {
                canonical: self.source().source(),
                solid: self.solid(),
                present: [
                    self.outer().lower_section().is_some(),
                    self.outer().upper_section().is_some(),
                ],
            },
            |p| self.source().classify_point(p, policy),
            || self.source().intersect_segment(start, end, policy),
            |p| self.classify_point(p, policy),
        )
    }
}
