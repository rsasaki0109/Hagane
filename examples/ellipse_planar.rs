use hagane::*;
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let value = |i: usize, default: f64| -> Result<f64> {
        args.get(i).map_or(Ok(default), |s| {
            s.parse()
                .map_err(|_| Error::InvalidInput("offset/placement must be numeric"))
        })
    };
    println!(
        "{}",
        ellipse_planar_demo_json(value(0, 14.)?, value(1, 0.)?)?
    );
    Ok(())
}
