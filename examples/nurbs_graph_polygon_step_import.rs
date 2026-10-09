use hagane::*;
use std::io::Read;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if !(1..=2).contains(&args.len()) {
        return Err("usage: nurbs_graph_polygon_step_import PATH [chord_error]".into());
    }
    let mut text = String::new();
    std::fs::File::open(&args[0])?
        .take(STEP_IMPORT_MAX_BYTES as u64 + 1)
        .read_to_string(&mut text)?;
    if text.len() > STEP_IMPORT_MAX_BYTES {
        return Err("polygon STEP import exceeds 1 MiB".into());
    }
    let error = args.get(1).map(|v| v.parse()).transpose()?.unwrap_or(0.2);
    println!(
        "{}",
        nurbs_graph_polygon_step_import_demo_json(&text, error)?
    );
    Ok(())
}
