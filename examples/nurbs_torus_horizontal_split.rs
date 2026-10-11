use hagane::*;
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let v: Vec<f64> = match std::env::args().nth(1) {
        Some(s) => serde_json::from_str(&s)?,
        None => vec![16., 4., 0., 0., 0., 0., 1e-6, 0.1, 1., 0.1],
    };
    if v.len() != 10 {
        return Err("torus horizontal split requires ten values".into());
    }
    let mut session = NurbsTorusDemoSession::new();
    session.apply(&v[..8])?;
    println!("{}", session.split_horizontal(v[8], v[9])?);
    Ok(())
}
