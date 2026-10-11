use hagane::*;
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let v: Vec<f64> = match std::env::args().nth(1) {
        Some(s) => serde_json::from_str(&s)?,
        None => vec![16., 4., 0., 0., 0., 0., 1e-6, 0.1, 1., 0.1, 1., 16., 0., 1.],
    };
    if v.len() != 14 || !(v[10] == 0. || v[10] == 1.) {
        return Err(
            "torus child point query requires fourteen values and exact zero/one mode".into(),
        );
    }
    let mut s = NurbsTorusDemoSession::new();
    s.apply(&v[..8])?;
    s.split_horizontal(v[8], v[9])?;
    println!(
        "{}",
        s.query_horizontal_point(v[10] == 1., Point3::new(v[11], v[12], v[13]))?
    );
    Ok(())
}
