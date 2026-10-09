use hagane::*;
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let number = |i: usize, default: f64| -> std::result::Result<f64, Box<dyn std::error::Error>> {
        Ok(if let Some(v) = args.get(i) {
            v.parse()?
        } else {
            default
        })
    };
    println!(
        "{}",
        nurbs_surface_crease_demo_json(
            number(0, 35.)?,
            number(1, 1.)?,
            number(2, 0.5)?,
            number(3, 0.5)?,
            number(4, 0.05)?,
            args.get(5)
                .map(|s| s.parse::<u32>())
                .transpose()?
                .unwrap_or(0)
        )?
    );
    Ok(())
}
