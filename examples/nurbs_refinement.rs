use hagane::*;
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let number = |i: usize, default: f64| -> std::result::Result<f64, Box<dyn std::error::Error>> {
        Ok(if let Some(text) = args.get(i) {
            text.parse()?
        } else {
            default
        })
    };
    println!(
        "{}",
        nurbs_tessellation_demo_json(
            number(0, std::f64::consts::FRAC_1_SQRT_2)?,
            number(1, 0.5)?,
            number(2, 0.001)?
        )?
    );
    Ok(())
}
