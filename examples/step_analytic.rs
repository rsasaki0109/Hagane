use hagane::*;
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mode = std::env::args().nth(1).unwrap_or("cylinder".into());
    let scale: f64 = std::env::args().nth(2).unwrap_or("1".into()).parse()?;
    let t = Tolerance::new(1e-8 * scale)?;
    let base = Point3::new(0., 0., -12. * scale);
    let solid = match mode.as_str() {
        "cylinder" => make_cylinder(
            CylinderSpec {
                base,
                radius: 8. * scale,
                height: 24. * scale,
            },
            t,
        )?,
        "tube" | "placed-tube" => {
            let tube = make_tube(
                TubeSpec {
                    base,
                    outer_radius: 8. * scale,
                    inner_radius: 4. * scale,
                    height: 24. * scale,
                },
                t,
            )?;
            if mode == "placed-tube" {
                tube.transformed(
                    Transform::translation(Vec3::new(20. * scale, -7. * scale, 12. * scale))?
                        .compose(Transform::rotation(Vec3::new(1., 2., 3.), 0.7)?)?,
                    t,
                )?
            } else {
                tube
            }
        }
        _ => return Err("mode must be cylinder, tube or placed-tube".into()),
    };
    print!("{}", export_step_mm(&solid, t)?);
    Ok(())
}
