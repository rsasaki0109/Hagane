use hagane::*;
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let mode = args.first().map_or(Ok(0), |s| {
        s.parse()
            .map_err(|_| Error::InvalidInput("mode must be integer"))
    })?;
    let value = |i: usize, default: f64| -> Result<f64> {
        args.get(i).map_or(Ok(default), |s| {
            s.parse()
                .map_err(|_| Error::InvalidInput("offset/placement must be numeric"))
        })
    };
    println!(
        "{}",
        half_ellipse_planar_demo_json(mode, value(1, 14.)?, value(2, 0.)?)?
    );
    Ok(())
}
