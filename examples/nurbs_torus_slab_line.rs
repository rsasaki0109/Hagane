use hagane::*;
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let v: Vec<f64> = match std::env::args().nth(1) {
        Some(s) => serde_json::from_str(&s)?,
        None => vec![
            16., 4., 0., 0., 0., 0., 1e-6, 0.1, -1., 1., 0.1, 16., 0., -8., 0., 0., 2., 0.,
        ],
    };
    if v.len() != 18 || !(v[17] == 0. || v[17] == 1.) {
        return Err("torus slab line/ray requires eighteen values and mode zero/one".into());
    }
    let mut s = NurbsTorusDemoSession::new();
    s.apply(&v[..8])?;
    s.slab_horizontal(v[8], v[9], v[10])?;
    let o = Point3::new(v[11], v[12], v[13]);
    let d = Vec3::new(v[14], v[15], v[16]);
    println!(
        "{}",
        if v[17] == 1. {
            s.intersect_slab_ray(o, d)?
        } else {
            s.intersect_slab_line(o, d)?
        }
    );
    Ok(())
}
