use hagane::*;
fn main() -> Result<()> {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([80., 60., 24.], 36., tol)?
        .trimmed_uv([[0.1, 0.9], [0.2, 0.8]], tol)?
        .transformed(
            Transform::translation(Vec3::new(12., -7., 4.))?
                .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.4)?)?,
            tol,
        )?;
    let body = NurbsGraphHoledSolid::new(&source, [[0.3, 0.7], [0.35, 0.65]], tol)?;
    print!("{}", body.export_step_mm(tol)?);
    Ok(())
}
