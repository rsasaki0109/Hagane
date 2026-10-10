use hagane::*;
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let values: Vec<f64> = match std::env::args().nth(1) {
        Some(text) => serde_json::from_str(&text)?,
        None => vec![
            16., 8., 24., 0., 0., 0., 0., 1e-6, 0.1, -25., 3., 12., 1., 0., 0., 0.,
        ],
    };
    println!("{}", nurbs_frustum_line_demo_json(&values)?);
    Ok(())
}
