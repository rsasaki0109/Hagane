//! Exact uniform-material centroid of retained polynomial graph columns.
use hagane::*;
fn main() -> Result<()> {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([80., 60., 20.], 36., tol)?
        .trimmed_uv([[0.1, 0.9], [0.2, 0.8]], tol)?
        .transformed(
            Transform::translation(Vec3::new(15., -10., 25.))?
                .compose(Transform::rotation(Vec3::new(0., 1., 0.), 0.4)?)?,
            tol,
        )?;
    let body = NurbsGraphHoledSolid::new(&source, [[0.25, 0.45], [0.35, 0.65]], tol)?;
    for (name, mass) in [
        ("plain", source.mass_properties(tol)?),
        ("holed", body.mass_properties(tol)?),
    ] {
        println!(
            "{name}: volume {:.6} mm³, world centroid ({:.6}, {:.6}, {:.6}) mm (uniform material)",
            mass.volume, mass.centroid.x, mass.centroid.y, mass.centroid.z
        );
    }
    Ok(())
}
