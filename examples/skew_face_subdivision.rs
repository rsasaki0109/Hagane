use hagane::*;
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let number = |index: usize, default: f64| -> Result<f64> {
        args.get(index).map_or(Ok(default), |s| {
            s.parse().map_err(|_| {
                Error::InvalidInput("expected numeric subdivision fraction and placement")
            })
        })
    };
    println!(
        "{}",
        skew_face_subdivision_demo_json(number(0, 14. / 32.)?, number(1, 0.)?)?
    );
    Ok(())
}
