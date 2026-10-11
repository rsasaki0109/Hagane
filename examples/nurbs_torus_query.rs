use hagane::*;
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let values: Vec<f64> = match std::env::args().nth(1) {
        Some(s) => serde_json::from_str(&s)?,
        None => vec![16., 4., 0., 0., 0., 0., 1e-6, 0.1, 16., 0., 0.],
    };
    if values.len() != 11 {
        return Err("torus query requires eleven values".into());
    }
    let mut session = NurbsTorusDemoSession::new();
    session.apply(&values[..8])?;
    println!(
        "{}",
        session.query_point(Point3::new(values[8], values[9], values[10]))?
    );
    Ok(())
}
