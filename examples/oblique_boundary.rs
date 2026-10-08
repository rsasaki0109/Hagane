use hagane::*;
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let number = |i: usize, default: f64| {
        args.get(i).map_or(Ok(default), |s| {
            s.parse()
                .map_err(|_| Error::InvalidInput("expected numeric slope and placement"))
        })
    };
    println!(
        "{}",
        oblique_boundary_demo_json(number(0, -1. / 24.)?, number(1, 0.)?)?
    );
    Ok(())
}
