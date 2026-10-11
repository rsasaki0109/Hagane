//! Standalone actual rational ring-torus native/WASM report.
use crate::*;
/// Major/minor radii, Y rotation, translation XYZ, linear/chord tolerances.
/// Transactional accepted actual torus. Failed display/model edits leave it intact.
#[derive(Default)]
pub struct NurbsTorusDemoSession {
    accepted: Option<(NurbsTorusSolid, GeometryTolerance)>,
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
    pub fn split_horizontal(&self, height: f64, chord: f64) -> Result<String> {
        let (body, policy) = self.accepted.as_ref().ok_or(Error::InvalidInput(
            "torus horizontal split requires an accepted body",
        ))?;
        crate::nurbs_torus_horizontal_demo::serialize_horizontal_split(body, height, chord, *policy)
            .map(|v| v.to_string())
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
        "scope":"exact regular ring torus with sixteen rational tensor patches; bounded B-rep display and exact STEP export; mesh-free actual accepted-body point, finite segment, line and ray queries; torus import and general Booleans unsupported"}),
    )
}
