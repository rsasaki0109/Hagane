use hagane::*;
use std::io::Read;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() > 1 {
        return Err("usage: nurbs_frustum_translated_step_import [PATH]".into());
    }
    let policy = GeometryTolerance::default();
    let text = if let Some(path) = args.first() {
        let mut text = String::new();
        std::fs::File::open(path)?
            .take(STEP_IMPORT_MAX_BYTES as u64 + 1)
            .read_to_string(&mut text)?;
        if text.len() > STEP_IMPORT_MAX_BYTES {
            return Err("translated frustum STEP import exceeds 1 MiB".into());
        }
        text
    } else {
        let source = NurbsFrustumSolid::new(
            Transform::translation(Vec3::new(12., -5., 8.))?,
            [16., 8.],
            24.,
            policy,
        )?;
        source
            .split_axial(10., policy)?
            .upper
            .export_step_mm(policy)?
    };
    let imported = import_step_nurbs_frustum_translated_mm(&text, policy)?;
    imported.validate(policy)?;
    let mass = imported.mass_properties(policy)?;
    let inertia = imported.inertia_properties(policy)?;
    let frame = imported.frame();
    let body = imported.solid();
    let xyz = |v: Vec3| [v.x, v.y, v.z];
    println!(
        "{}",
        serde_json::json!({
            "units":"mm", "radii":imported.radii(), "height":imported.height(),
            "frame":{"origin":xyz(frame.origin()), "axes":frame.axes().map(xyz)},
            "brep":{"vertices":body.vertices.len(), "edges":body.edges.len(), "faces":body.shell.faces.len()},
            "volume":mass.volume, "centroid":xyz(mass.centroid), "inertia":inertia.inertia,
            "scope":"checked translated identity-axis rational frustum STEP import"
        })
    );
    Ok(())
}
