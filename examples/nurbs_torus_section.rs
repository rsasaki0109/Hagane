use hagane::*;
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let v: Vec<f64> = match std::env::args().nth(1) {
        Some(s) => serde_json::from_str(&s)?,
        None => vec![16., 4., 0., 0., 0., 0., 1e-6, 0.1, 1.],
    };
    if v.len() != 9 {
        return Err("torus section requires nine values".into());
    }
    let mut s = NurbsTorusDemoSession::new();
    s.apply(&v[..8])?;
    println!("{}", s.section_horizontal(v[8], v[7])?);
    Ok(())
}
