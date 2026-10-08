fn main() -> hagane::Result<()> {
    let mut args = std::env::args().skip(1);
    let mode = args
        .next()
        .map(|x| x.parse::<u32>())
        .transpose()
        .map_err(|_| hagane::Error::InvalidInput("mode must be an integer"))?
        .unwrap_or(0);
    let mut number = |default| {
        args.next()
            .map(|x| x.parse::<f64>())
            .transpose()
            .map_err(|_| hagane::Error::InvalidInput("offset and placement must be numbers"))
            .map(|x| x.unwrap_or(default))
    };
    let offset = number(1.0)?;
    let placement = number(0.7)?;
    println!(
        "{}",
        hagane::intersections_demo_json(mode, offset, placement)?
    );
    Ok(())
}
