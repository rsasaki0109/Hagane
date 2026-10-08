use hagane::*;
fn main() -> Result<()> {
    let offset = std::env::args().nth(1).map_or(Ok(-2.0), |s| {
        s.parse()
            .map_err(|_| Error::InvalidInput("expected plane offset"))
    })?;
    println!("{}", solid_split_demo_json(offset)?);
    Ok(())
}
