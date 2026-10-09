use hagane::*;
fn main() -> Result<()> {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([80., 60., 24.], 36., tol)?
        .transformed(Transform::rotation(Vec3::new(1., 2., 3.), 0.4)?, tol)?;
    let solid = NurbsGraphHoledSolid::new(&source, [[0.3, 0.7], [0.2, 0.6]], tol)?;
    let mesh = solid.tessellate_bounded(0.1, 65536, tol)?.mesh;
    for p in &mesh.positions {
        println!("v {} {} {}", p.x, p.y, p.z);
    }
    for n in &mesh.normals {
        println!("vn {} {} {}", n.x, n.y, n.z);
    }
    for t in &mesh.triangles {
        println!("f {0}//{0} {1}//{1} {2}//{2}", t[0] + 1, t[1] + 1, t[2] + 1);
    }
    Ok(())
}
