use hagane::*;
fn main() -> Result<()> {
    let tol = GeometryTolerance::new(1e-4, 1e-10, 1e-12)?;
    let source = NurbsGraphSolid::new([80., 60., 24.], 36., tol.absolute())?;
    let solid = NurbsGraphHoledSolid::new(&source, [[0.35, 0.65], [0.35, 0.65]], tol.absolute())?;
    for point in [
        Point3::new(20., 15., 10.),
        Point3::new(40., 30., 10.),
        Point3::new(28., 30., 10.),
        Point3::new(20., 15., 0.),
    ] {
        println!("{point:?}: {:?}", solid.classify_point(point, tol)?);
    }
    Ok(())
}
