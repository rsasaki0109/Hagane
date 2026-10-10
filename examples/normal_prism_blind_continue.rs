use hagane::*;
use std::io::Read;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.is_empty() || args.len() > 2 {
        return Err("usage: normal_prism_blind_continue STEP_PATH [JSON_VALUES]".into());
    }
    let mut input = String::new();
    std::fs::File::open(&args[0])?
        .take(STEP_IMPORT_MAX_BYTES as u64 + 1)
        .read_to_string(&mut input)?;
    if input.len() > STEP_IMPORT_MAX_BYTES {
        return Err("continued blind STEP input exceeds 1 MiB".into());
    }
    let values: Vec<f64> = if let Some(values) = args.get(1) {
        serde_json::from_str(values)?
    } else {
        vec![0., 0., 1., 1e-6, 0.1, 0., 0., 20., 5., 8., 0.]
    };
    println!(
        "{}",
        normal_prism_blind_continue_demo_json(&input, &values)?
    );
    Ok(())
}
