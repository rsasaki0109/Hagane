use hagane::*;
use std::io::Read;
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() > 3 {
        return Err("usage: nurbs_torus_step_import [PATH|- [chord [linear]]]".into());
    }
    let chord: f64 = args.get(1).map(|s| s.parse()).transpose()?.unwrap_or(0.1);
    let linear: f64 = args.get(2).map(|s| s.parse()).transpose()?.unwrap_or(1e-6);
    if !chord.is_finite() || chord <= 0. || !linear.is_finite() || linear <= 0. {
        return Err("torus STEP chord and linear tolerance must be finite and positive".into());
    }
    let policy = GeometryTolerance::new(linear, GeometryTolerance::default().angular(), 0.)?;
    let text = match args.first() {
        Some(path) => {
            let mut text = String::new();
            let mut reader: Box<dyn Read> = if path == "-" {
                Box::new(std::io::stdin())
            } else {
                Box::new(std::fs::File::open(path)?)
            };
            reader
                .by_ref()
                .take(STEP_IMPORT_MAX_BYTES as u64 + 1)
                .read_to_string(&mut text)?;
            if text.len() > STEP_IMPORT_MAX_BYTES {
                return Err("torus STEP import exceeds 1 MiB".into());
            }
            text
        }
        None => NurbsTorusSolid::new(
            Frame3::new_with_tolerance(
                Point3::new(12., -5., 8.),
                [
                    Vec3::new(0., 1., 0.),
                    Vec3::new(0., 0., 1.),
                    Vec3::new(1., 0., 0.),
                ],
                policy,
            )?,
            [16., 4.],
            policy,
        )?
        .export_step_mm(policy)?,
    };
    let mut session = NurbsTorusDemoSession::new();
    println!("{}", session.import_step_cardinal(&text, chord, policy)?);
    Ok(())
}
