use hagane::*;
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let model = args.first().map_or(Ok(0), |s| {
        s.parse()
            .map_err(|_| Error::InvalidInput("expected model id"))
    })?;
    if args.get(1).is_some_and(|s| s == "--mesh") {
        println!("{}", curved_classification_mesh_json(model)?);
        return Ok(());
    }
    let value = |i: usize| -> Result<f64> {
        args.get(i).map_or(Ok(0.), |s| {
            s.parse()
                .map_err(|_| Error::InvalidInput("expected point coordinates"))
        })
    };
    println!(
        "{}",
        curved_classification_demo_json(model, value(1)?, value(2)?, value(3)?)?
    );
    Ok(())
}
