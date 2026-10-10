fn main() -> Result<(), Box<dyn std::error::Error>> {
    let values: Vec<f64> = match std::env::args().nth(1) {
        Some(json) => serde_json::from_str(&json)?,
        None => vec![
            80., 60., 20., 8., 0., 0., 0., 0., 1e-6, 0.1, -20., 0., 6., 12., 0., 0., 0., 5., 8.,
            0., 20., 0., 4., 6., 0.,
        ],
    };
    println!("{}", hagane::normal_prism_blind_bores_demo_json(&values)?);
    Ok(())
}
