//! Standalone actual rational ring-torus native/WASM report.
use crate::*;
/// Major/minor radii, Y rotation, translation XYZ, linear/chord tolerances.
pub fn nurbs_torus_demo_json(x: &[f64]) -> Result<String> {
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
    serialize_torus(&body, policy, x[7]).map(|v| v.to_string())
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
        "scope":"exact regular ring torus with sixteen rational tensor patches; bounded B-rep display and exact STEP export; torus import, queries and general Booleans unsupported"}),
    )
}
