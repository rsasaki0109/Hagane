use hagane::*;
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let integer = |i: usize, default: &str| -> Result<u32> {
        args.get(i)
            .map(String::as_str)
            .unwrap_or(default)
            .parse()
            .map_err(|_| Error::InvalidInput("selection/mode must be integer"))
    };
    let number = |i: usize, default: &str| -> Result<f64> {
        args.get(i)
            .map(String::as_str)
            .unwrap_or(default)
            .parse()
            .map_err(|_| Error::InvalidInput("offset/placement must be numeric"))
    };
    println!(
        "{}",
        harmonic_face_intersections_demo_json(
            integer(0, "0")?,
            integer(1, "0")?,
            number(2, "14")?,
            number(3, "0")?
        )?
    );
    Ok(())
}
