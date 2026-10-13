use hagane::*;
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let v: Vec<f64> = match std::env::args().nth(1) {
        Some(s) => serde_json::from_str(&s)?,
        None => vec![
            16., 4., 0., 0., 0., 0., 1e-6, 0.1, 1., 0.1, 1., 16., 0., 0., 0., 0., 2.,
        ],
    };
    if v.len() != 17 || !(v[10] == 0. || v[10] == 1.) {
        return Err(
            "torus child line/ray requires seventeen values and exact zero/one mode".into(),
        );
    }
    let mut s = NurbsTorusDemoSession::new();
    s.apply(&v[..8])?;
    s.split_horizontal(v[8], v[9])?;
    println!(
        "{}",
        s.intersect_horizontal_ray(
            v[10] == 1.,
            Point3::new(v[11], v[12], v[13]),
            Vec3::new(v[14], v[15], v[16])
        )?
    );
    Ok(())
}
