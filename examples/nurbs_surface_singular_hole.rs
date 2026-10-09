use hagane::*;
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let mut values = [35., 0.1, 0.3];
    for (i, value) in values.iter_mut().enumerate() {
        if let Some(arg) = args.get(i) {
            *value = arg.parse()?;
        }
    }
    println!(
        "{}",
        nurbs_surface_singular_hole_demo_json(values[0], values[1], values[2])?
    );
    Ok(())
}
