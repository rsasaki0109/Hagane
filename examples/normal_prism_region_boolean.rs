use hagane::*;
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let values: Vec<f64> = match std::env::args().nth(1) {
        Some(text) => serde_json::from_str(&text)?,
        None => vec![
            80., 60., 20., 8., 12., 6., 0., 0., 0.17, 0., 0., 0., 0., 1e-6, 0.1,
        ],
    };
    println!("{}", normal_prism_region_boolean_demo_json(&values)?);
    Ok(())
}
