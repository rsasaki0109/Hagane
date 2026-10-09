use hagane::*;
use std::io::Read;
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let path = args
        .first()
        .ok_or("usage: nurbs_graph_holed_step_import PATH [chord_error]")?;
    let mut text = String::new();
    std::fs::File::open(path)?
        .take(STEP_IMPORT_MAX_BYTES as u64 + 1)
        .read_to_string(&mut text)?;
    if text.len() > STEP_IMPORT_MAX_BYTES {
        return Err("graph STEP import exceeds 1 MiB".into());
    }
    let error = args.get(1).map(|v| v.parse()).transpose()?.unwrap_or(0.2);
    println!("{}", nurbs_graph_holed_step_import_demo_json(&text, error)?);
    Ok(())
}
