use hagane::*;
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let v: Vec<f64> = match std::env::args().nth(1) {
        Some(s) => serde_json::from_str(&s)?,
        None => vec![16., 4., 0., 0., 0., 0., 1e-6, 0.1, 0., 0., 0., 2., 0., 0.],
    };
    if v.len() != 14 {
        return Err("torus line/ray requires fourteen values".into());
    }
    let mut session = NurbsTorusDemoSession::new();
    session.apply(&v[..8])?;
    println!(
        "{}",
        session.intersect_ray(
            Point3::new(v[8], v[9], v[10]),
            Vec3::new(v[11], v[12], v[13])
        )?
    );
    Ok(())
}
