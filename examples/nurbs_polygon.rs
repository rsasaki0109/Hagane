use hagane::*;
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let height = args.first().map(|v| v.parse()).transpose()?.unwrap_or(35.);
    let weight = args.get(1).map(|v| v.parse()).transpose()?.unwrap_or(1.);
    let error = args.get(2).map(|v| v.parse()).transpose()?.unwrap_or(0.5);
    let mode = args.get(3).map(|v| v.parse()).transpose()?.unwrap_or(0);
    let mut uv = [0.125, 0.125, 0.875, 0.25, 0.25, 0.875];
    for (i, v) in uv.iter_mut().enumerate() {
        if let Some(arg) = args.get(i + 4) {
            *v = arg.parse()?;
        }
    }
    println!(
        "{}",
        nurbs_polygon_demo_json(
            height,
            weight,
            error,
            mode,
            vec![[uv[0], uv[1]], [uv[2], uv[3]], [uv[4], uv[5]]]
        )?
    );
    Ok(())
}
