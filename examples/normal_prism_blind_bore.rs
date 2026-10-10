fn main() -> Result<(), Box<dyn std::error::Error>> {
    let values: Vec<f64> = match std::env::args().nth(1) {
        Some(json) => serde_json::from_str(&json)?,
        None => vec![
            80., 60., 20., 8., 0., 0., 8., 12., 0., 0., 0., 0., 0., 1e-6, 0.1,
        ],
    };
    println!("{}", hagane::normal_prism_blind_bore_demo_json(&values)?);
    Ok(())
}
