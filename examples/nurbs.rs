fn main() -> hagane::Result<()> {
    let mut arguments = std::env::args().skip(1);
    let mut value = |default: f64| -> hagane::Result<f64> {
        arguments
            .next()
            .map(|s| s.parse())
            .transpose()
            .map_err(|_| hagane::Error::InvalidInput("expected numeric weight and parameter"))
            .map(|v| v.unwrap_or(default))
    };
    let weight = value(std::f64::consts::FRAC_1_SQRT_2)?;
    let parameter = value(0.5)?;
    println!("{}", hagane::nurbs_demo_json(weight, parameter)?);
    Ok(())
}
