//! Independent analytic circular plate with a tilted cylindrical through bore.
use crate::*;
use std::f64::consts::PI;

/// Restricted closed fixture, not a general Boolean: a circular plate with a
/// centered cylindrical bore tilted about Y. Horizontal caps retain unequal-axis
/// ellipse holes. The bore must stay strictly inside the outer cylinder at both
/// caps; finite tilt magnitudes at most pi/3 are accepted when resolved.
pub fn tilted_bore_demo_solid(
    radius: f64,
    bore_radius: f64,
    height: f64,
    tilt: f64,
    tol: GeometryTolerance,
) -> Result<Solid> {
    if !radius.is_finite()
        || !bore_radius.is_finite()
        || !height.is_finite()
        || !tilt.is_finite()
        || radius <= 10. * tol.linear()
        || bore_radius <= 10. * tol.linear()
        || height <= 10. * tol.linear()
        || tilt.abs() > PI / 3.
    {
        return Err(Error::InvalidInput(
            "tilted bore requires resolved positive dimensions and tilt in [-pi/3, pi/3]",
        ));
    }
    let clearance = radius - height / 2. * tilt.tan().abs() - bore_radius / tilt.cos();
    if !clearance.is_finite() || clearance <= 20. * tol.linear() {
        return Err(Error::Unsupported(
            "tilted bore touches or leaves the circular plate",
        ));
    }
    let mut solid = Solid {
        vertices: Vec::new(),
        edges: Vec::new(),
        shell: Shell { faces: Vec::new() },
    };
    let mut cap_wires = [Vec::new(), Vec::new()];
    for (ring, (r, angle)) in [(radius, 0.), (bore_radius, tilt)].into_iter().enumerate() {
        let (sin, cos) = angle.sin_cos();
        let axis = Vec3::new(sin, 0., cos);
        let basis = Vec3::new(cos, 0., -sin);
        let length = 2. * (height + r) / cos;
        let frame_origin = axis * (-length);
        let a = Vec3::new(r / cos, 0., 0.);
        let b = Vec3::new(0., r, 0.);
        let centers = [
            Point3::new(-height / 2. * angle.tan(), 0., -height / 2.),
            Point3::new(height / 2. * angle.tan(), 0., height / 2.),
        ];
        let vertex = solid.vertices.len();
        let edge = solid.edges.len();
        for center in centers {
            solid
                .vertices
                .extend([Vertex { point: center + a }, Vertex { point: center - a }]);
        }
        for (level, center) in centers.into_iter().enumerate() {
            for (half, sign) in [1., -1.].into_iter().enumerate() {
                solid.edges.push(Edge {
                    vertices: [vertex + level * 2 + half, vertex + level * 2 + (1 - half)],
                    curve: Curve::EllipseArc {
                        center,
                        cosine: a * sign,
                        sine: b * sign,
                        sweep: PI,
                    },
                });
            }
        }
        for half in 0..2 {
            let lower = solid.vertices[vertex + half].point;
            let upper = solid.vertices[vertex + 2 + half].point;
            solid.edges.push(Edge {
                vertices: [vertex + half, vertex + 2 + half],
                curve: Curve::Line { a: lower, b: upper },
            });
        }
        for (level, wires) in cap_wires.iter_mut().enumerate() {
            let mut coedges = [1., -1.]
                .into_iter()
                .enumerate()
                .map(|(half, sign)| Coedge {
                    edge: edge + level * 2 + half,
                    forward: ring == 0,
                    pcurve: PCurve::EllipseArc {
                        center: [centers[level].x, 0.],
                        cosine: [a.x * sign, 0.],
                        sine: [0., r * sign],
                        sweep: PI,
                    },
                })
                .collect::<Vec<_>>();
            if ring != 0 {
                coedges.reverse();
            }
            wires.push(Wire { coedges });
        }
        for (half, sign) in [1., -1.].into_iter().enumerate() {
            let frame = Frame3::new(
                frame_origin,
                [basis * sign, Vec3::new(0., sign, 0.), axis],
                tol.absolute(),
            )?;
            let bottom = length - height / (2. * cos);
            let top = length + height / (2. * cos);
            let coefficient = sign * r * angle.tan();
            let graph = |offset| PCurve::HeightGraph {
                offset,
                cosine: coefficient,
                sine: 0.,
                sweep: PI,
            };
            let generator = |u: f64, forward: bool, index: usize| Coedge {
                edge: edge + 4 + index,
                forward,
                pcurve: PCurve::Affine {
                    origin: [u, bottom + coefficient * u.cos()],
                    direction: [0., top - bottom],
                },
            };
            solid.shell.faces.push(Face {
                surface: Surface::FramedCylinder {
                    frame,
                    radius: r,
                    height: 2. * length,
                },
                orientation: if ring == 0 { 1 } else { -1 },
                wires: vec![Wire {
                    coedges: vec![
                        Coedge {
                            edge: edge + half,
                            forward: true,
                            pcurve: graph(bottom),
                        },
                        generator(PI, true, 1 - half),
                        Coedge {
                            edge: edge + 2 + half,
                            forward: false,
                            pcurve: graph(top),
                        },
                        generator(0., false, half),
                    ],
                }],
            });
        }
    }
    for (level, wires) in cap_wires.into_iter().enumerate() {
        solid.shell.faces.push(Face {
            surface: Surface::Plane {
                origin: Point3::new(
                    0.,
                    0.,
                    if level == 0 {
                        -height / 2.
                    } else {
                        height / 2.
                    },
                ),
                u: Vec3::new(1., 0., 0.),
                v: Vec3::new(0., 1., 0.),
            },
            orientation: if level == 0 { -1 } else { 1 },
            wires,
        });
    }
    solid.validate(tol.absolute())?;
    Ok(solid)
}
/// Native/WASM top-cap line clipping; tilt and placement in radians, V offset in mm.
pub fn tilted_bore_demo_json(tilt: f64, offset: f64, placement: f64) -> Result<String> {
    if !offset.is_finite() || !placement.is_finite() {
        return Err(Error::InvalidInput(
            "tilted bore query requires finite offset and placement",
        ));
    }
    let t = GeometryTolerance::default();
    let solid = tilted_bore_demo_solid(24., 6., 16., tilt, t)?.transformed(
        Transform::rotation(Vec3::new(1., 2., 3.), placement)?,
        t.absolute(),
    )?;
    let Surface::Plane { origin, u, v } = solid.shell.faces[5].surface else {
        unreachable!()
    };
    crate::ellipse_planar::ellipse_planar_query_json(
        &solid,
        5,
        origin - u * 34. + v * offset,
        u * 2.,
    )
}
