use hagane::*;
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let number = |i: usize, default: f64| -> Result<f64> {
        args.get(i).map_or(Ok(default), |s| {
            s.parse()
                .map_err(|_| Error::InvalidInput("tilt/offset/placement must be numeric"))
        })
    };
    println!(
        "{}",
        tilted_bore_demo_json(number(0, 0.5)?, number(1, 3.)?, number(2, 0.)?)?
    );
    Ok(())
}
