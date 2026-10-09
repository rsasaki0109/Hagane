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
    tilted_bores_demo_solid(
        radius,
        height,
        &[TiltedBore {
            radius: bore_radius,
            tilt,
            center: [0., 0.],
        }],
        tol,
    )
}
/// Circular cylindrical tool, translated in XY and tilted about Y.
#[derive(Clone, Copy, Debug)]
pub struct TiltedBore {
    pub radius: f64,
    pub tilt: f64,
    pub center: [f64; 2],
}
/// Restricted analytic plate construction with up to sixteen separated bores.
/// Each independently chosen Y tilt must pass a full-height separation certificate.
/// Uses exact cylinder walls/ellipse caps, not a general Boolean operation.
pub fn tilted_bores_demo_solid(
    radius: f64,
    height: f64,
    bores: &[TiltedBore],
    tol: GeometryTolerance,
) -> Result<Solid> {
    if bores.len() > 16 {
        return Err(Error::Unsupported(
            "tilted plate supports at most sixteen bores",
        ));
    }
    let oriented = bores
        .iter()
        .map(|b| OrientedBore {
            radius: b.radius,
            tilt: b.tilt,
            azimuth: 0.,
            center: b.center,
        })
        .collect::<Vec<_>>();
    oriented_bores_demo_solid(radius, height, &oriented, tol)
}
/// Circular cylindrical tool with independent inclination and XY azimuth (radians).
#[derive(Clone, Copy, Debug)]
pub struct OrientedBore {
    pub radius: f64,
    pub tilt: f64,
    pub azimuth: f64,
    pub center: [f64; 2],
}
/// Exact restricted plate construction with independently oriented through bores.
/// Each pair must have certified separation throughout the full plate height.
pub fn oriented_bores_demo_solid(
    radius: f64,
    height: f64,
    bores: &[OrientedBore],
    tol: GeometryTolerance,
) -> Result<Solid> {
    if !radius.is_finite()
        || !height.is_finite()
        || radius <= 10. * tol.linear()
        || height <= 10. * tol.linear()
    {
        return Err(Error::InvalidInput(
            "tilted plate dimensions must be finite and resolved",
        ));
    }
    if bores.len() > 16 {
        return Err(Error::Unsupported(
            "tilted plate supports at most sixteen bores",
        ));
    }
    for bore in bores {
        if !bore.radius.is_finite()
            || bore.radius <= 10. * tol.linear()
            || !bore.tilt.is_finite()
            || !bore.azimuth.is_finite()
            || bore.tilt.abs() > PI / 3.
            || bore.center.iter().any(|x| !x.is_finite())
        {
            return Err(Error::InvalidInput(
                "tilted bore requires resolved dimensions, finite center and tilt in [-pi/3, pi/3]",
            ));
        }
        let clearance = radius
            - bore.center[0].hypot(bore.center[1])
            - height / 2. * bore.tilt.tan().abs()
            - bore.radius / bore.tilt.cos();
        if !clearance.is_finite() || clearance <= 20. * tol.linear() {
            return Err(Error::Unsupported(
                "tilted bore touches or leaves the circular plate",
            ));
        }
    }
    for (i, bore) in bores.iter().enumerate() {
        for other in &bores[..i] {
            if !separated_through_height(bore, other, height, tol.linear()) {
                return Err(Error::Unsupported(
                    "bores have no separating plane throughout plate height",
                ));
            }
        }
    }
    let mut solid = Solid {
        vertices: Vec::new(),
        edges: Vec::new(),
        shell: Shell { faces: Vec::new() },
    };
    let mut cap_wires = [Vec::new(), Vec::new()];
    for (ring, bore) in std::iter::once(OrientedBore {
        radius,
        tilt: 0.,
        azimuth: 0.,
        center: [0., 0.],
    })
    .chain(bores.iter().copied())
    .enumerate()
    {
        let r = bore.radius;
        let angle = bore.tilt;
        let (sin, cos) = angle.sin_cos();
        let (sp, cp) = bore.azimuth.sin_cos();
        let axis = Vec3::new(sin * cp, sin * sp, cos);
        let basis = Vec3::new(cos * cp, cos * sp, -sin);
        let perpendicular = Vec3::new(-sp, cp, 0.);
        let length = 2. * (height + r) / cos;
        let frame_origin = axis * (-length) + Vec3::new(bore.center[0], bore.center[1], 0.);
        let a = Vec3::new(r / cos * cp, r / cos * sp, 0.);
        let b = perpendicular * r;
        let centers = [
            Point3::new(
                bore.center[0] - height / 2. * angle.tan() * cp,
                bore.center[1] - height / 2. * angle.tan() * sp,
                -height / 2.,
            ),
            Point3::new(
                bore.center[0] + height / 2. * angle.tan() * cp,
                bore.center[1] + height / 2. * angle.tan() * sp,
                height / 2.,
            ),
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
                        center: [centers[level].x, centers[level].y],
                        cosine: [a.x * sign, a.y * sign],
                        sine: [b.x * sign, b.y * sign],
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
                [basis * sign, perpendicular * sign, axis],
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

/// Exact two-tilted-bore cap query demonstrating overlapping enclosing circles.
pub fn separated_tilted_bores_demo_json(offset: f64) -> Result<String> {
    if !offset.is_finite() {
        return Err(Error::InvalidInput("offset must be finite"));
    }
    let solid = tilted_bores_demo_solid(
        24.,
        8.,
        &[
            TiltedBore {
                radius: 3.,
                tilt: 1.,
                center: [0., -4.],
            },
            TiltedBore {
                radius: 3.,
                tilt: 1.,
                center: [0., 4.],
            },
        ],
        GeometryTolerance::default(),
    )?;
    let face = solid.shell.faces.len() - 1;
    crate::ellipse_planar::ellipse_planar_query_json(
        &solid,
        face,
        Point3::new(-34., offset, 4.),
        Vec3::new(2., 0., 0.),
    )
}

// Fixed XY support direction proves separation of every horizontal slice. Center
// projections vary affinely in Z; a same-sign gap at both endpoints proves
// separation at every intermediate height, without sampling the solid interior.
fn separated_through_height(a: &OrientedBore, b: &OrientedBore, height: f64, linear: f64) -> bool {
    let dx = b.center[0] - a.center[0];
    let dy = b.center[1] - a.center[1];
    let (sa, ca) = a.azimuth.sin_cos();
    let (sb, cb) = b.azimuth.sin_cos();
    let drift = [
        height / 2. * (b.tilt.tan() * cb - a.tilt.tan() * ca),
        height / 2. * (b.tilt.tan() * sb - a.tilt.tan() * sa),
    ];
    let axes = [a.radius / a.tilt.cos(), b.radius / b.tilt.cos()];
    let arithmetic = 4096.
        * f64::EPSILON
        * (a.center[0].hypot(a.center[1])
            + b.center[0].hypot(b.center[1])
            + drift[0].hypot(drift[1])
            + axes[0]
            + axes[1]
            + height);
    let initial = dy.atan2(dx);
    (0..64).any(|i| {
        let angle = initial + PI * i as f64 / 64.;
        let (ny, nx) = angle.sin_cos();
        let low = nx * (dx - drift[0]) + ny * (dy - drift[1]);
        let high = nx * (dx + drift[0]) + ny * (dy + drift[1]);
        let bound = (axes[0] * (nx * ca + ny * sa)).hypot(a.radius * (-nx * sa + ny * ca))
            + (axes[1] * (nx * cb + ny * sb)).hypot(b.radius * (-nx * sb + ny * cb));
        let gap = if low > 0. && high > 0. {
            low.min(high) - bound
        } else if low < 0. && high < 0. {
            (-low).min(-high) - bound
        } else {
            0.
        };
        gap.is_finite() && gap > 20. * linear + arithmetic
    })
}
/// Oppositely tilted exact cylindrical bores separated through the full height.
pub fn divergent_tilted_bores_demo_json(tilt: f64, offset: f64) -> Result<String> {
    if !offset.is_finite() {
        return Err(Error::InvalidInput("offset must be finite"));
    }
    let solid = tilted_bores_demo_solid(
        24.,
        8.,
        &[
            TiltedBore {
                radius: 3.,
                tilt,
                center: [0., -4.],
            },
            TiltedBore {
                radius: 3.,
                tilt: -tilt,
                center: [0., 4.],
            },
        ],
        GeometryTolerance::default(),
    )?;
    let face = solid.shell.faces.len() - 1;
    crate::ellipse_planar::ellipse_planar_query_json(
        &solid,
        face,
        Point3::new(-34., offset, 4.),
        Vec3::new(2., 0., 0.),
    )
}

/// Two arbitrary-azimuth bores; line direction rotates with the queried hole.
pub fn oriented_bores_demo_json(azimuth: f64, offset: f64) -> Result<String> {
    if !azimuth.is_finite() || !offset.is_finite() {
        return Err(Error::InvalidInput("azimuth and offset must be finite"));
    }
    let bores = [
        OrientedBore {
            radius: 3.,
            tilt: 0.7,
            azimuth,
            center: [0., -8.],
        },
        OrientedBore {
            radius: 3.,
            tilt: 0.5,
            azimuth: azimuth + PI / 2.,
            center: [0., 8.],
        },
    ];
    let solid = oriented_bores_demo_solid(28., 8., &bores, GeometryTolerance::default())?;
    let (s, c) = azimuth.sin_cos();
    let u = Vec3::new(c, s, 0.);
    let v = Vec3::new(-s, c, 0.);
    let anchor =
        Point3::new(4. * 0.7f64.tan() * c, -8. + 4. * 0.7f64.tan() * s, 4.) - u * 40. + v * offset;
    crate::ellipse_planar::ellipse_planar_query_json(
        &solid,
        solid.shell.faces.len() - 1,
        anchor,
        u * 2.,
    )
}
