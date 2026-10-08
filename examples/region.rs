use hagane::{
    extrude_arc_line_region, rounded_rectangle_profile, ArcLineRegion, PlanarSegment, Point3,
    Tolerance,
};
use std::f64::consts::PI;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tol = Tolerance::default();
    let outer = rounded_rectangle_profile(Point3::new(0.0, 0.0, 0.0), 12.0, 10.0, 1.0, tol)?;
    let hole = [0.0, PI]
        .map(|start_angle| PlanarSegment::Arc {
            center: [0.0, 0.0],
            radius: 2.0,
            start_angle,
            sweep: PI,
        })
        .to_vec();
    let solid = extrude_arc_line_region(
        &ArcLineRegion {
            origin: outer.origin,
            outer: outer.segments,
            holes: vec![hole],
        },
        3.0,
        tol,
    )?;
    solid.validate(tol)?;
    let expected = (120.0 - (4.0 - PI) - 4.0 * PI) * 3.0;
    assert!((solid.volume()? - expected).abs() < 1e-10);
    println!(
        "Volume: {:.9}; faces: {}; display triangles: {}",
        solid.volume()?,
        solid.shell.faces.len(),
        solid.tessellate(0.01, tol)?.triangles.len()
    );
    Ok(())
}
