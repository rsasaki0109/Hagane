fn main() -> hagane::Result<()> {
    let mut args = std::env::args().skip(1);
    let mut number = |default| {
        args.next()
            .map(|s| s.parse::<f64>())
            .transpose()
            .map_err(|_| hagane::Error::InvalidInput("offset and placement must be numbers"))
            .map(|x| x.unwrap_or(default))
    };
    let offset = number(0.5)?;
    let placement = number(0.7)?;
    println!("{}", hagane::face_clipping_demo_json(offset, placement)?);
    Ok(())
}
