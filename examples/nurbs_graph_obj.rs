use hagane::*;
fn main() -> Result<()> {
    let tol = Tolerance::default();
    let graph = NurbsGraphSolid::new([80., 60., 24.], 36., tol)?;
    let rotation = Transform::rotation(Vec3::new(1., 2., 3.), 0.4)?;
    let placement = Transform::translation(Vec3::new(12., -7., 4.))?.compose(rotation)?;
    let graph = graph.transformed(placement, tol)?;
    let result = graph.tessellate_bounded(0.1, 65536, tol)?;
    for p in &result.mesh.positions {
        println!("v {} {} {}", p.x, p.y, p.z);
    }
    for n in &result.mesh.normals {
        println!("vn {} {} {}", n.x, n.y, n.z);
    }
    for t in &result.mesh.triangles {
        println!("f {0}//{0} {1}//{1} {2}//{2}", t[0] + 1, t[1] + 1, t[2] + 1);
    }
    Ok(())
}
