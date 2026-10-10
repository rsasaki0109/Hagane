use hagane::*;
use std::io::Read;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() > 2 {
        return Err("usage: nurbs_frustum_cardinal_step_import_demo [PATH [chord_error]]".into());
    }
    let policy = GeometryTolerance::default();
    let text = if let Some(path) = args.first() {
        let mut text = String::new();
        std::fs::File::open(path)?
            .take(STEP_IMPORT_MAX_BYTES as u64 + 1)
            .read_to_string(&mut text)?;
        if text.len() > STEP_IMPORT_MAX_BYTES {
            return Err("cardinal frustum STEP import exceeds 1 MiB".into());
        }
        text
    } else {
        NurbsFrustumSolid::new(
            Frame3::new_with_tolerance(
                Point3::new(12., -5., 8.),
                [
                    Vec3::new(0., 1., 0.),
                    Vec3::new(0., 0., 1.),
                    Vec3::new(1., 0., 0.),
                ],
                policy,
            )?,
            [16., 8.],
            24.,
            policy,
        )?
        .split_axial(10., policy)?
        .upper
        .export_step_mm(policy)?
    };
    let chord = args.get(1).map(|v| v.parse()).transpose()?.unwrap_or(0.1);
    println!(
        "{}",
        nurbs_frustum_cardinal_step_import_demo_json(&text, chord)?
    );
    Ok(())
}
