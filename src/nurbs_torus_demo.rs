//! Standalone actual rational ring-torus native/WASM report.
use crate::*;
/// Major/minor radii, Y rotation, translation XYZ, linear/chord tolerances.
/// Transactional accepted actual torus. Failed display/model edits leave it intact.
#[derive(Default)]
pub struct NurbsTorusDemoSession {
    accepted: Option<(NurbsTorusSolid, GeometryTolerance)>,
    horizontal: Option<(NurbsTorusHorizontalSplit, GeometryTolerance)>,
    slab: Option<(NurbsTorusHorizontalSlab, GeometryTolerance)>,
}
impl NurbsTorusDemoSession {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn accepted_body(&self) -> Option<&NurbsTorusSolid> {
        self.accepted.as_ref().map(|v| &v.0)
    }
    pub fn apply(&mut self, x: &[f64]) -> Result<String> {
        let (body, policy) = torus_body(x)?;
        let report = serialize_torus(&body, policy, x[7])?.to_string();
        self.accepted = Some((body, policy));
        self.horizontal = None;
        self.slab = None;
        Ok(report)
    }
    /// Import an exact bounded cardinal STEP body transactionally.
    pub fn import_step_cardinal(
        &mut self,
        text: &str,
        chord: f64,
        policy: GeometryTolerance,
    ) -> Result<String> {
        if !chord.is_finite() || chord <= 0. {
            return Err(Error::InvalidInput(
                "torus STEP display chord must be finite and positive",
            ));
        }
        let body = import_step_nurbs_torus_cardinal_mm(text, policy)?;
        let report = serialize_torus(&body, policy, chord)?.to_string();
        self.accepted = Some((body, policy));
        self.horizontal = None;
        self.slab = None;
        Ok(report)
    }
    /// Mesh-free, read-only query of the accepted typed B-rep and its policy.
    pub fn query_point(&self, point: Point3) -> Result<String> {
        let (body, policy) = self.accepted.as_ref().ok_or(Error::InvalidInput(
            "torus point query requires an accepted body",
        ))?;
        let q = body.point_query(point, *policy)?;
        let xyz = |p: Point3| [p.x, p.y, p.z];
        let location = match q.location {
            PointLocation::Inside => "inside",
            PointLocation::Outside => "outside",
            PointLocation::Boundary => "boundary",
        };
        let boundary=q.boundary.map(|b|serde_json::json!({"face":b.face,"uv":b.uv,"point":xyz(b.point),"distance":b.distance,"normal":[b.normal.x,b.normal.y,b.normal.z]}));
        Ok(
            serde_json::json!({"point":xyz(point),"location":location,"boundary":boundary})
                .to_string(),
        )
    }
    /// Read-only finite segment intersection on the accepted actual B-rep.
    pub fn intersect_segment(&self, start: Point3, end: Point3) -> Result<String> {
        let (body, policy) = self.accepted.as_ref().ok_or(Error::InvalidInput(
            "torus segment query requires an accepted body",
        ))?;
        let result = body.intersect_segment(start, end, *policy)?;
        let xyz = |p: Point3| [p.x, p.y, p.z];
        let hits=result.hits.into_iter().map(|h| {let b=h.boundary;serde_json::json!({"parameter":h.parameter,"point":xyz(h.point),"boundary":{"face":b.face,"uv":b.uv,"point":xyz(b.point),"distance":b.distance,"normal":[b.normal.x,b.normal.y,b.normal.z]}})}).collect::<Vec<_>>();
        Ok(serde_json::json!({"start":xyz(start),"end":xyz(end),"hits":hits,"material_intervals":result.material_intervals,"mesh_free":true}).to_string())
    }
    /// Return two actual closed children without replacing the accepted full torus.
    pub fn split_horizontal(&mut self, height: f64, chord: f64) -> Result<String> {
        if !height.is_finite() || !chord.is_finite() {
            return Err(Error::InvalidInput(
                "torus horizontal split values must be finite",
            ));
        }
        let (body, policy) = self.accepted.as_ref().ok_or(Error::InvalidInput(
            "torus horizontal split requires an accepted body",
        ))?;
        let split = body.split_by_local_z(height, *policy)?;
        let report = crate::nurbs_torus_horizontal_demo::serialize_horizontal_split(
            body, &split, chord, *policy,
        )?
        .to_string();
        self.horizontal = Some((split, *policy));
        Ok(report)
    }
    pub fn accepted_horizontal_split(&self) -> Option<&NurbsTorusHorizontalSplit> {
        self.horizontal.as_ref().map(|v| &v.0)
    }
    pub fn query_horizontal_point(&self, is_lower: bool, point: Point3) -> Result<String> {
        let (split, policy) = self.horizontal.as_ref().ok_or(Error::InvalidInput(
            "torus child point query requires an accepted horizontal split",
        ))?;
        let body = if is_lower { &split.lower } else { &split.upper };
        let q = body.point_query(point, *policy)?;
        let xyz = |p: Point3| [p.x, p.y, p.z];
        let location = match q.location {
            PointLocation::Inside => "inside",
            PointLocation::Outside => "outside",
            PointLocation::Boundary => "boundary",
        };
        let boundary=q.boundary.map(|b|serde_json::json!({"face":b.face,"uv":b.uv,"point":xyz(b.point),"distance":b.distance,"normal":[b.normal.x,b.normal.y,b.normal.z]}));
        Ok(serde_json::json!({"child":if is_lower {"lower"} else {"upper"},"cut_height":body.cut_height(),"point":xyz(point),"location":location,"boundary":boundary}).to_string())
    }
    /// Mesh-free finite segment on the cached actual selected horizontal child.
    pub fn intersect_horizontal_segment(
        &self,
        is_lower: bool,
        start: Point3,
        end: Point3,
    ) -> Result<String> {
        let (split, policy) = self.horizontal.as_ref().ok_or(Error::InvalidInput(
            "torus child segment requires an accepted horizontal split",
        ))?;
        let body = if is_lower { &split.lower } else { &split.upper };
        let result = body.intersect_segment(start, end, *policy)?;
        let xyz = |p: Point3| [p.x, p.y, p.z];
        let hits=result.hits.into_iter().map(|h|{let b=h.boundary;serde_json::json!({"parameter":h.parameter,"point":xyz(h.point),"boundary":{"face":b.face,"uv":b.uv,"point":xyz(b.point),"distance":b.distance,"normal":[b.normal.x,b.normal.y,b.normal.z]}})}).collect::<Vec<_>>();
        Ok(serde_json::json!({"child":if is_lower {"lower"}else{"upper"},"cut_height":body.cut_height(),"start":xyz(start),"end":xyz(end),"hits":hits,"material_intervals":result.material_intervals,"mesh_free":true}).to_string())
    }
    pub fn intersect_horizontal_line(
        &self,
        is_lower: bool,
        origin: Point3,
        direction: Vec3,
    ) -> Result<String> {
        self.horizontal_line_query(is_lower, origin, direction, false)
    }
    pub fn intersect_horizontal_ray(
        &self,
        is_lower: bool,
        origin: Point3,
        direction: Vec3,
    ) -> Result<String> {
        self.horizontal_line_query(is_lower, origin, direction, true)
    }
    fn horizontal_line_query(
        &self,
        is_lower: bool,
        origin: Point3,
        direction: Vec3,
        ray: bool,
    ) -> Result<String> {
        let (split, policy) = self.horizontal.as_ref().ok_or(Error::InvalidInput(
            "torus child line/ray requires an accepted horizontal split",
        ))?;
        let body = if is_lower { &split.lower } else { &split.upper };
        let result = if ray {
            body.intersect_ray(origin, direction, *policy)?
        } else {
            body.intersect_line(origin, direction, *policy)?
        };
        let xyz = |p: Point3| [p.x, p.y, p.z];
        let hits=result.hits.into_iter().map(|h|{let b=h.boundary;serde_json::json!({"parameter":h.parameter,"point":xyz(h.point),"boundary":{"face":b.face,"uv":b.uv,"point":xyz(b.point),"distance":b.distance,"normal":[b.normal.x,b.normal.y,b.normal.z]}})}).collect::<Vec<_>>();
        Ok(serde_json::json!({"child":if is_lower {"lower"}else{"upper"},"cut_height":body.cut_height(),"query_kind":if ray {"ray"}else{"line"},"origin":xyz(origin),"direction":[direction.x,direction.y,direction.z],"hits":hits,"material_intervals":result.material_intervals,"mesh_free":true}).to_string())
    }
    /// Exact section and bounded curve display without modifying accepted state.
    /// Local height and display chord are in source millimetres.
    pub fn section_horizontal(&self, height: f64, chord: f64) -> Result<String> {
        let (body, policy) = self.accepted.as_ref().ok_or(Error::InvalidInput(
            "torus section requires an accepted body",
        ))?;
        crate::nurbs_torus_section_demo::serialize_section(body, height, chord, *policy)
            .map(|v| v.to_string())
    }
    /// Create and display an actual slab while retaining source and cut-pair caches.
    /// Heights and display chord are in source millimetres.
    pub fn slab_horizontal(&mut self, lo: f64, hi: f64, chord: f64) -> Result<String> {
        if !lo.is_finite() || !hi.is_finite() || !chord.is_finite() || chord <= 0. {
            return Err(Error::InvalidInput(
                "torus slab requires finite bounds and positive finite chord",
            ));
        }
        let (source, policy) = self.accepted.as_ref().ok_or(Error::InvalidInput(
            "torus slab requires an accepted source",
        ))?;
        let body = source.slab_between_local_z([lo, hi], *policy)?;
        let report =
            crate::nurbs_torus_horizontal_slab_demo::serialize_slab(&body, chord, *policy)?
                .to_string();
        self.slab = Some((body, *policy));
        Ok(report)
    }
    pub fn accepted_slab(&self) -> Option<&NurbsTorusHorizontalSlab> {
        self.slab.as_ref().map(|v| &v.0)
    }
    pub fn accepted_slab_policy(&self) -> Option<GeometryTolerance> {
        self.slab.as_ref().map(|v| v.1)
    }
    /// Exact STEP of the accepted actual slab, without display or reconstruction.
    pub fn slab_step(&self) -> Result<String> {
        let (body, policy) = self.slab.as_ref().ok_or(Error::InvalidInput(
            "torus slab STEP requires an accepted slab",
        ))?;
        let step = body.export_step_mm(*policy)?;
        Ok(serde_json::json!({"kind":"nurbs_torus_horizontal_slab_step","heights":body.heights(),"step":step,"step_exact":true}).to_string())
    }
    /// Read-only point query of the accepted actual slab and its policy.
    pub fn query_slab_point(&self, point: Point3) -> Result<String> {
        let (body, policy) = self.slab.as_ref().ok_or(Error::InvalidInput(
            "torus slab point query requires an accepted slab",
        ))?;
        let result = body.point_query(point, *policy)?;
        let xyz = |p: Point3| [p.x, p.y, p.z];
        let location = match result.location {
            PointLocation::Inside => "inside",
            PointLocation::Outside => "outside",
            PointLocation::Boundary => "boundary",
        };
        let boundary=result.boundary.map(|b|serde_json::json!({"face":b.face,"uv":b.uv,"point":xyz(b.point),"distance":b.distance,"normal":[b.normal.x,b.normal.y,b.normal.z]}));
        Ok(serde_json::json!({"kind":"nurbs_torus_horizontal_slab_point","heights":body.heights(),"point":xyz(point),"location":location,"boundary":boundary}).to_string())
    }
    pub fn intersect_line(&self, origin: Point3, direction: Vec3) -> Result<String> {
        self.line_query(origin, direction, false)
    }
    pub fn intersect_ray(&self, origin: Point3, direction: Vec3) -> Result<String> {
        self.line_query(origin, direction, true)
    }
    fn line_query(&self, origin: Point3, direction: Vec3, ray: bool) -> Result<String> {
        let (body, policy) = self.accepted.as_ref().ok_or(Error::InvalidInput(
            "torus line/ray query requires an accepted body",
        ))?;
        let result = if ray {
            body.intersect_ray(origin, direction, *policy)?
        } else {
            body.intersect_line(origin, direction, *policy)?
        };
        let xyz = |p: Point3| [p.x, p.y, p.z];
        let hits=result.hits.into_iter().map(|h|{let b=h.boundary;serde_json::json!({"parameter":h.parameter,"point":xyz(h.point),"boundary":{"face":b.face,"uv":b.uv,"point":xyz(b.point),"distance":b.distance,"normal":[b.normal.x,b.normal.y,b.normal.z]}})}).collect::<Vec<_>>();
        Ok(serde_json::json!({"origin":xyz(origin),"direction":[direction.x,direction.y,direction.z],"hits":hits,"material_intervals":result.material_intervals,"mesh_free":true,"query_kind":if ray {"ray"} else {"line"}}).to_string())
    }
}
pub fn nurbs_torus_demo_json(x: &[f64]) -> Result<String> {
    NurbsTorusDemoSession::new().apply(x)
}
fn torus_body(x: &[f64]) -> Result<(NurbsTorusSolid, GeometryTolerance)> {
    if x.len() != 8 || x.iter().any(|v| !v.is_finite()) {
        return Err(Error::InvalidInput(
            "NURBS torus requires exactly eight finite values",
        ));
    }
    let policy = GeometryTolerance::new(x[6], GeometryTolerance::default().angular(), 0.)?;
    let rotation = Transform::rotation(Vec3::new(0., 1., 0.), x[2])?;
    let frame = Frame3::new_with_tolerance(
        Point3::new(x[3], x[4], x[5]),
        [
            rotation.vector(Vec3::new(1., 0., 0.)),
            rotation.vector(Vec3::new(0., 1., 0.)),
            rotation.vector(Vec3::new(0., 0., 1.)),
        ],
        policy,
    )?;
    let body = NurbsTorusSolid::new(frame, [x[0], x[1]], policy)?;
    Ok((body, policy))
}
pub(crate) fn serialize_torus(
    body: &NurbsTorusSolid,
    policy: GeometryTolerance,
    chord: f64,
) -> Result<serde_json::Value> {
    let xyz = |p: Point3| [p.x, p.y, p.z];
    let frame = body.frame();
    let axes = frame.axes().map(|v| [v.x, v.y, v.z]);
    let bounds = body.bounds(policy)?;
    let mesh = body.tessellate(chord, policy)?;
    let mass = match body.inertia_properties(policy) {
        Ok(m) => {
            serde_json::json!({"ok":true,"volume":m.volume,"centroid":xyz(m.centroid),"inertia":m.inertia,"reference":"centroid","tensor_axes":"world","volume_units":"mm^3","centroid_units":"mm","inertia_units":"mm^5","density":1})
        }
        Err(Error::Unsupported(reason)) => serde_json::json!({"ok":false,"error":reason}),
        Err(e) => return Err(e),
    };
    Ok(
        serde_json::json!({"units":"mm","kind":"nurbs_ring_torus","radii":body.radii(),"major_radius":body.radii()[0],"minor_radius":body.radii()[1],"volume":body.volume(policy)?,"mass_properties":mass,
        "bounds":{"min":xyz(bounds.min),"max":xyz(bounds.max)},"bounds_kind":"conservative positive-weight rational control hull","placement":{"translation":xyz(frame.origin()),"axes":axes},"axis":axes[2],"linear_tolerance":policy.linear(),"display_chord_tolerance":chord,
        "mesh":{"positions":mesh.positions.iter().map(|p|xyz(*p)).collect::<Vec<_>>(),"normals":mesh.normals.iter().map(|n|[n.x,n.y,n.z]).collect::<Vec<_>>(),"triangles":mesh.triangles,"face_ids":mesh.face_ids},
        "brep":crate::nurbs_graph_polygon_demo::serialize_graph_brep(body.solid())?,"closed":true,"genus":1,
        "step":body.export_step_mm(policy)?,"step_exact":true,"step_schema":"AUTOMOTIVE_DESIGN","import_supported":false,
        "scope":"exact regular ring torus with sixteen rational tensor patches; bounded B-rep display and exact STEP export; mesh-free actual accepted-body point, finite segment, line and ray queries; bounded exact cardinal millimetre STEP import; general torus import and Booleans unsupported"}),
    )
}
