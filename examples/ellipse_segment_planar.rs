use hagane::*;
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let number = |i: usize, default: f64| -> Result<f64> {
        args.get(i).map_or(Ok(default), |s| {
            s.parse()
                .map_err(|_| Error::InvalidInput("sweep/offset/placement must be numeric"))
        })
    };
    let mode = args.get(1).map_or(Ok(0), |s| {
        s.parse()
            .map_err(|_| Error::InvalidInput("mode must be integer"))
    })?;
    println!(
        "{}",
        ellipse_segment_planar_demo_json(
            number(0, std::f64::consts::PI / 2.)?,
            mode,
            number(2, 20.)?,
            number(3, 0.)?
        )?
    );
    Ok(())
}
