use hagane::*;
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let defaults = [35., 1., 0.5, 0.5, 0.1, 0.2, 0.8, 0.15, 0.85];
    let mut values = defaults;
    for (i, value) in values.iter_mut().enumerate() {
        if let Some(arg) = args.get(i) {
            *value = arg.parse()?;
        }
    }
    let crease: u32 = args.get(9).map(|s| s.parse()).transpose()?.unwrap_or(0);
    if crease > 1 {
        return Err("crease mode must be 0 or 1".into());
    }
    println!(
        "{}",
        nurbs_surface_trim_demo_json(
            values[0],
            values[1],
            values[2],
            values[3],
            values[4],
            [[values[5], values[6]], [values[7], values[8]]],
            crease == 1
        )?
    );
    Ok(())
}
