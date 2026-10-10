use hagane::*;
use std::io::Read;
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() > 1 {
        return Err("usage: step_bounded_import [PATH]".into());
    }
    let input = if args.first().is_some_and(|s| s == "--sample-openings") {
        bounded_analytic_openings_sample_step()?
    } else if let Some(path) = args.first() {
        let mut text = String::new();
        std::fs::File::open(path)?
            .take(STEP_IMPORT_MAX_BYTES as u64 + 1)
            .read_to_string(&mut text)?;
        if text.len() > STEP_IMPORT_MAX_BYTES {
            return Err("bounded STEP import exceeds 1 MiB".into());
        }
        text
    } else {
        let tolerance = GeometryTolerance::new(1e-6, 1e-10, 0.)?;
        let source = make_box(
            BoxSpec {
                min: Point3::new(0., 0., 0.),
                size: Vec3::new(80., 60., 20.),
            },
            tolerance.absolute(),
        )?;
        let fillet =
            fillet_parallel_box_edges(&source, &[(8, 3.), (9, 3.), (10, 3.), (11, 3.)], tolerance)?;
        export_step_bounded_analytic_mm(fillet.solid(), 1e-6)?
    };
    println!("{}", import_step_bounded_analytic_json(&input)?);
    Ok(())
}
