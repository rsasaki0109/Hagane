use hagane::*;
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let mut values = [35., 1., 0.1, 0.3];
    for (i, value) in values.iter_mut().enumerate() {
        if let Some(arg) = args.get(i) {
            *value = arg.parse()?;
        }
    }
    let crease: u32 = args.get(4).map(|s| s.parse()).transpose()?.unwrap_or(0);
    if crease > 1 {
        return Err("crease mode must be 0 or 1".into());
    }
    println!(
        "{}",
        nurbs_surface_hole_demo_json(values[0], values[1], values[2], values[3], crease == 1)?
    );
    Ok(())
}
