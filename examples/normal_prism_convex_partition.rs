use hagane::*;
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let values: Vec<f64> = match std::env::args().nth(1) {
        Some(text) => serde_json::from_str(&text)?,
        None => vec![
            80., 60., 20., 8., 100., 12., 0., 30., 0., 0., 0., 0., 0., 1e-6, 0.1,
        ],
    };
    println!("{}", normal_prism_convex_partition_demo_json(&values)?);
    Ok(())
}
