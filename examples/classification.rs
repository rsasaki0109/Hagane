use hagane::*;
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|s| s == "--mesh") {
        println!("{}", classification_mesh_demo_json()?);
        return Ok(());
    }
    let value = |i: usize, default: f64| -> Result<f64> {
        args.get(i).map_or(Ok(default), |s| {
            s.parse()
                .map_err(|_| Error::InvalidInput("expected point coordinates"))
        })
    };
    println!(
        "{}",
        classification_demo_json(value(0, -10.)?, value(1, 0.)?, value(2, 0.)?)?
    );
    Ok(())
}
