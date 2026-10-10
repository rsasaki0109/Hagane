use hagane::*;
use std::io::Read;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() > 2 {
        return Err("usage: nurbs_frustum_step_import [PATH [chord_error]]".into());
    }
    let text = if let Some(path) = args.first() {
        let mut text = String::new();
        std::fs::File::open(path)?
            .take(2 * 1024 * 1024 + 1)
            .read_to_string(&mut text)?;
        if text.len() > 2 * 1024 * 1024 {
            return Err("frustum STEP import transport exceeds 2 MiB".into());
        }
        text
    } else {
        let policy = GeometryTolerance::default();
        NurbsFrustumSolid::new(Frame3::IDENTITY, [16., 8.], 24., policy)?.export_step_mm(policy)?
    };
    let chord = args.get(1).map(|v| v.parse()).transpose()?.unwrap_or(0.1);
    println!("{}", nurbs_frustum_step_import_demo_json(&text, chord)?);
    Ok(())
}
