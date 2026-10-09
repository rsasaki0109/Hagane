use crate::*;
use std::f64::consts::TAU;
#[derive(Clone, Debug, Default)]
pub struct Mesh {
    pub positions: Vec<Point3>,
    pub normals: Vec<Vec3>,
    pub triangles: Vec<[usize; 3]>,
    pub face_ids: Vec<usize>,
}
impl Mesh {
    /// Signed enclosed volume of a closed, oriented mesh; closure is not checked.
    /// Recentered tetrahedra and compensated summation avoid world-origin cancellation.
    pub fn signed_volume(&self) -> f64 {
        let Some(first) = self.triangles.first() else {
            return 0.;
        };
        let reference = self.positions[first[0]];
        let mut sum: f64 = 0.;
        let mut correction = 0.;
        for t in &self.triangles {
            let a = self.positions[t[0]] - reference;
            let b = self.positions[t[1]] - reference;
            let c = self.positions[t[2]] - reference;
            let term = a.dot(b.cross(c)) / 6.;
            let next = sum + term;
            correction += if sum.abs() >= term.abs() {
                (sum - next) + term
            } else {
                (term - next) + sum
            };
            sum = next;
        }
        sum + correction
    }
    fn triangle(&mut self, mut p: [Point3; 3], mut n: [Vec3; 3], face: usize, orientation: i8) {
        if orientation < 0 {
            p.swap(1, 2);
            n.swap(1, 2);
        }
        let start = self.positions.len();
        self.positions.extend(p);
        self.normals.extend(n.map(|x| x * orientation as f64));
        self.triangles.push([start, start + 1, start + 2]);
        self.face_ids.push(face);
    }
}
/// A conservative chordal segment count. Error is the circle sagitta, in length units.
pub fn circle_segments(radius: f64, chord_error: f64) -> Result<usize> {
    if !radius.is_finite() || radius <= 0.0 || !chord_error.is_finite() || chord_error <= 0.0 {
        return Err(Error::InvalidInput(
            "positive finite radius and chord error required",
        ));
    }
    let angle = 4.0 * (chord_error / (2.0 * radius)).min(0.5).sqrt().asin();
    let count = (TAU / angle).ceil().max(12.0);
    if !count.is_finite() || count > 65536.0 {
        return Err(Error::Tessellation(
            "requested accuracy exceeds 65536 segments",
        ));
    }
    Ok(count as usize)
}
/// Sagitta-bounded sampling of a positive angular span, in radians.
pub fn arc_segments(radius: f64, sweep: f64, chord_error: f64) -> Result<usize> {
    if !sweep.is_finite() || sweep <= 0.0 || sweep > TAU {
        return Err(Error::InvalidInput("arc sweep must be in (0, 2pi]"));
    }
    Ok((circle_segments(radius, chord_error)? as f64 * sweep / TAU)
        .ceil()
        .max(1.0) as usize)
}

fn absolute(p: Vec3) -> Vec3 {
    Vec3::new(p.x.abs(), p.y.abs(), p.z.abs())
}
// Absolute coefficient envelopes cover intermediate arithmetic, not only the
// final bounding box. Thus large cancelling plane origins are not overlooked.
fn coordinate_roundoff_budget(solid: &Solid) -> Result<f64> {
    let mut scale: f64 = 0.;
    let mut observe = |p: Vec3| -> Result<()> {
        if !p.finite() {
            return Err(Error::Tessellation(
                "coordinate evaluation envelope overflows",
            ));
        }
        scale = scale.max(p.x.abs()).max(p.y.abs()).max(p.z.abs());
        Ok(())
    };
    for v in &solid.vertices {
        observe(v.point)?;
    }
    for edge in &solid.edges {
        match edge.curve {
            Curve::Nurbs(_) => {
                return Err(Error::Unsupported(
                    "NURBS solid tessellation is not implemented",
                ))
            }
            Curve::Line { a, b } => {
                observe(a)?;
                observe(b)?;
            }
            Curve::Circle { center, radius } => {
                observe(absolute(center) + Vec3::new(radius, radius, 0.))?;
            }
            Curve::FramedCircle { frame, radius } | Curve::Arc { frame, radius, .. } => {
                let axes = frame.axes();
                observe(
                    absolute(frame.origin()) + (absolute(axes[0]) + absolute(axes[1])) * radius,
                )?;
            }
            Curve::EllipseArc {
                center,
                cosine,
                sine,
                ..
            } => {
                observe(absolute(center) + absolute(cosine) + absolute(sine))?;
            }
        }
    }
    for face in &solid.shell.faces {
        if let Surface::Plane { origin, u, v } = face.surface {
            for c in face.wires.iter().flat_map(|w| &w.coedges) {
                let range = solid.edges[c.edge].curve.range();
                let parameter = range[0].abs().max(range[1].abs());
                let uv = match c.pcurve {
                    PCurve::Nurbs(_) => {
                        return Err(Error::Unsupported(
                            "rational pcurve analysis is not implemented",
                        ))
                    }
                    PCurve::Affine { origin, direction } => {
                        std::array::from_fn(|i| origin[i].abs() + direction[i].abs() * parameter)
                    }
                    PCurve::Circle { center, radius } | PCurve::Arc { center, radius, .. } => {
                        center.map(|x| x.abs() + radius)
                    }
                    PCurve::EllipseArc {
                        center,
                        cosine,
                        sine,
                        ..
                    } => std::array::from_fn(|i| center[i].abs() + cosine[i].abs() + sine[i].abs()),
                    PCurve::HeightGraph {
                        offset,
                        cosine,
                        sine,
                        ..
                    } => [parameter, offset.abs() + cosine.abs() + sine.abs()],
                };
                observe(absolute(origin) + absolute(u) * uv[0] + absolute(v) * uv[1])?;
            }
        } else {
            let (frame, radius, _, drift) = crate::circular_trims::surface_data(&face.surface)?;
            let axial = face
                .circular_bands()?
                .iter()
                .map(|b| b[0].abs() + b[1].abs() + b[2].abs())
                .fold(0., f64::max);
            let axes = frame.axes();
            observe(
                absolute(frame.origin())
                    + (absolute(axes[0]) + absolute(axes[1])) * radius
                    + (absolute(axes[2])
                        + absolute(axes[0]) * drift[0].abs()
                        + absolute(axes[1]) * drift[1].abs())
                        * axial,
            )?;
        }
    }
    Ok(32. * f64::EPSILON * scale)
}
// Opposite rims use the same angular grid. Union propagation keeps cap/wall
// and ellipse/wall samples conforming even after oblique cuts increase accuracy.
pub(crate) fn shared_edge_counts(solid: &Solid, error: f64) -> Result<Vec<usize>> {
    let mut parent: Vec<_> = (0..solid.edges.len()).collect();
    fn root(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    for face in &solid.shell.faces {
        if !matches!(face.surface, Surface::Plane { .. }) {
            let c = &face.wires[0].coedges;
            let a = root(&mut parent, c[0].edge);
            let b = root(&mut parent, c[2].edge);
            parent[a] = b;
        }
    }
    let mut maxima = vec![0; solid.edges.len()];
    for (i, edge) in solid.edges.iter().enumerate() {
        let count = match edge.curve {
            Curve::Nurbs(_) => {
                return Err(Error::Unsupported(
                    "NURBS solid tessellation is not implemented",
                ))
            }
            Curve::Line { .. } => 1,
            Curve::Circle { radius, .. } | Curve::FramedCircle { radius, .. } => {
                circle_segments(radius, error)?
            }
            Curve::Arc { radius, sweep, .. } => arc_segments(radius, sweep, error)?,
            Curve::EllipseArc {
                cosine,
                sine,
                sweep,
                ..
            } => {
                let curvature = cosine.norm() + sine.norm();
                let count = (sweep * (curvature / (8. * error)).sqrt()).ceil().max(1.);
                if !curvature.is_finite() || !count.is_finite() || count > 65536. {
                    return Err(Error::Tessellation(
                        "ellipse accuracy exceeds 65536 segments",
                    ));
                }
                count as usize
            }
        };
        let r = root(&mut parent, i);
        maxima[r] = maxima[r].max(count);
    }
    Ok((0..solid.edges.len())
        .map(|i| maxima[root(&mut parent, i)])
        .collect())
}
impl Solid {
    /// Conservative allowance for floating coordinate evaluation during meshing.
    /// It includes placement and trim coefficients, including cancelling origins.
    /// This is an arithmetic guard, not an interval proof for arbitrary libm.
    pub fn tessellation_roundoff_budget(&self, tol: Tolerance) -> Result<f64> {
        self.validate(tol)?;
        coordinate_roundoff_budget(self)
    }
    /// Meshes the exact supported B-rep surfaces and trims; never performs mesh CSG.
    /// Coordinate roundoff is reserved from the chord error. Unresolved requests fail.
    pub fn tessellate(&self, chord_error: f64, tol: Tolerance) -> Result<Mesh> {
        self.validate(tol)?;
        if !chord_error.is_finite() || chord_error <= 0.0 {
            return Err(Error::InvalidInput("positive finite chord error required"));
        }
        let roundoff = coordinate_roundoff_budget(self)?;
        if roundoff >= chord_error * 0.25 {
            return Err(Error::Tessellation(
                "coordinate precision cannot resolve requested chord error; use a local frame or coarser display tolerance",
            ));
        }
        let chord_error = chord_error - roundoff;
        let counts = shared_edge_counts(self, chord_error)?;
        let mut mesh = Mesh::default();
        for (fi, f) in self.shell.faces.iter().enumerate() {
            match f.surface {
                Surface::Nurbs(_) => {
                    return Err(Error::Unsupported(
                        "NURBS solid tessellation is not implemented",
                    ))
                }
                Surface::Plane { .. } => {
                    let mut coords = Vec::new();
                    let mut holes = Vec::new();
                    let mut line_vertices = Vec::new();
                    let mixed = f.wires.iter().flat_map(|w| &w.coedges).any(|c| {
                        matches!(c.pcurve, PCurve::Arc { .. } | PCurve::EllipseArc { .. })
                    });
                    for (wi, w) in f.wires.iter().enumerate() {
                        if wi > 0 {
                            holes.push(coords.len() / 2);
                        }
                        for c in &w.coedges {
                            let edge = &self.edges[c.edge];
                            if matches!(edge.curve, Curve::Line { .. }) {
                                line_vertices.push(coords.len() / 2);
                            }
                            let count = counts[c.edge];
                            if mixed && coords.len() / 2 + count > 4096 {
                                return Err(Error::Tessellation("mixed planar display trim checking is limited to 4096 total samples per face"));
                            }
                            let range = edge.curve.range();
                            for k in 0..count {
                                let frac = k as f64 / count as f64;
                                let t = if c.forward {
                                    range[0] + (range[1] - range[0]) * frac
                                } else {
                                    range[1] - (range[1] - range[0]) * frac
                                };
                                coords.extend(c.pcurve.evaluate(t));
                            }
                        }
                    }
                    if mixed {
                        let boundaries: Vec<_> = std::iter::once(0)
                            .chain(holes.iter().copied())
                            .zip(
                                holes
                                    .iter()
                                    .copied()
                                    .chain(std::iter::once(coords.len() / 2)),
                            )
                            .map(|(lo, hi)| {
                                (lo..hi)
                                    .map(|i| [coords[2 * i], coords[2 * i + 1]])
                                    .collect()
                            })
                            .collect();
                        crate::planar::validate_sampled_region(&boundaries,tol).map_err(|_|Error::Tessellation("sampled curved trims are unresolved or intersecting; adjust chord error or model tolerance"))?;
                    }
                    let triangles = earcutr::earcut(&coords, &holes, 2)
                        .map_err(|_| Error::Tessellation("planar trim triangulation failed"))?;
                    if triangles.is_empty() {
                        return Err(Error::Tessellation("no planar triangles"));
                    }
                    let candidates = if mixed {
                        (0..coords.len() / 2).collect::<Vec<_>>()
                    } else {
                        line_vertices
                    };
                    let triangles = if candidates.is_empty() {
                        triangles
                    } else {
                        conforming_triangles(&coords, &triangles, &candidates)?
                    };
                    for tri in triangles.as_chunks::<3>().0 {
                        let p = [tri[0], tri[1], tri[2]]
                            .map(|i| f.surface.evaluate(coords[i * 2], coords[i * 2 + 1]));
                        let normal = f.surface.normal(0.0);
                        let mut p = p;
                        if (p[1] - p[0]).cross(p[2] - p[0]).dot(normal) < 0.0 {
                            p.swap(1, 2);
                        }
                        mesh.triangle(p, [normal; 3], fi, f.orientation);
                    }
                }
                Surface::Cylinder { .. }
                | Surface::FramedCylinder { .. }
                | Surface::ExtrudedCircle { .. } => {
                    let span = f.circular_span()?;
                    let bands = f.circular_bands()?;
                    let n = counts[f.wires[0].coedges[0].edge];
                    for i in 0..n {
                        let a = span * i as f64 / n as f64;
                        let b = span * (i + 1) as f64 / n as f64;
                        let p = [
                            f.surface
                                .evaluate(a, crate::circular_trims::value(bands[0], a)),
                            f.surface
                                .evaluate(b, crate::circular_trims::value(bands[0], b)),
                            f.surface
                                .evaluate(b, crate::circular_trims::value(bands[1], b)),
                            f.surface
                                .evaluate(a, crate::circular_trims::value(bands[1], a)),
                        ];
                        let na = f.surface.normal(a);
                        let nb = f.surface.normal(b);
                        mesh.triangle([p[0], p[1], p[2]], [na, nb, nb], fi, f.orientation);
                        mesh.triangle([p[0], p[2], p[3]], [na, nb, na], fi, f.orientation);
                    }
                }
            }
        }
        Ok(mesh)
    }
}
/// Browser/native single-bore fixture. Kept as a compatible shorthand.
pub fn demo_json(radius: f64, chord_error: f64) -> Result<String> {
    demo_preset_json(0, radius, chord_error)
}
/// Select actual kernel operations: 0 single bore, 1 four bores, 2 concave
/// polygon extrusion with a polygon hole, 3 coaxial tube, 4 rigidly placed
/// four-bore part, 5 rounded-rectangle line/arc extrusion, 6 concave arc-notch with rounded hole, 7 exact planar face split, 8 arc-rim and cylinder-wall refinement.
/// Unknown IDs fail.
pub fn demo_preset_json(preset: u32, radius: f64, chord_error: f64) -> Result<String> {
    if preset == 24 {
        return crate::oblique_split::oblique_boundary_mesh_json(
            (radius - 16.) / 48.,
            0.,
            chord_error,
        );
    }
    let tol = Tolerance::default();
    let b = BoxSpec {
        min: Point3::new(-40.0, -30.0, -12.0),
        size: Vec3::new(80.0, 60.0, 24.0),
    };
    let tool = |x, y, radius| CylinderSpec {
        base: Point3::new(x, y, -20.0),
        radius,
        height: 40.0,
    };
    let s = match preset {
        0 => subtract_through_cylinder(b, tool(0.0, 0.0, radius), tol)?,
        // Half the radius slider: four independent bores at fixed centers.
        1 => subtract_through_cylinders(
            b,
            &[
                tool(-20.0, -14.0, radius * 0.5),
                tool(20.0, -14.0, radius * 0.5),
                tool(20.0, 14.0, radius * 0.5),
                tool(-20.0, 14.0, radius * 0.5),
            ],
            tol,
        )?,
        2 => extrude_polygon(
            &PolygonProfile {
                origin: Point3::new(0.0, 0.0, -12.0),
                outer: vec![
                    [-40.0, -30.0],
                    [40.0, -30.0],
                    [40.0, -5.0],
                    [-5.0, -5.0],
                    [-5.0, 30.0],
                    [-40.0, 30.0],
                ],
                holes: vec![vec![
                    [-32.0, -16.0],
                    [-20.0, -16.0],
                    [-20.0, 12.0],
                    [-32.0, 12.0],
                ]],
            },
            Vec3::new(6.0, 3.0, 24.0),
            tol,
        )?,
        4 => subtract_through_cylinders(
            b,
            &[
                tool(-20.0, -14.0, radius * 0.5),
                tool(20.0, -14.0, radius * 0.5),
                tool(20.0, 14.0, radius * 0.5),
                tool(-20.0, 14.0, radius * 0.5),
            ],
            tol,
        )?
        .transformed(
            Transform::translation(Vec3::new(8.0, -4.0, 6.0))?
                .compose(Transform::rotation(Vec3::new(1.0, 2.0, 0.5), 0.8)?)?,
            tol,
        )?,
        30 => box_face_blind_bore_demo_solid(BoxFace::MinY, radius * 0.5, 20.)?,
        29 => subtract_blind_cylinder(
            b,
            CylinderSpec {
                base: Point3::new(0., 0., -4.),
                radius,
                height: 20.,
            },
            tol,
        )?,
        28 => {
            if !radius.is_finite() || !(8.0..=24.0).contains(&radius) {
                return Err(Error::Unsupported("oriented bore control must be in 8..24"));
            }
            crate::oriented_bores_demo_solid(
                28.,
                8.,
                &[
                    crate::OrientedBore {
                        radius: radius / 6.,
                        tilt: 0.7,
                        azimuth: 0.4,
                        center: [0., -8.],
                    },
                    crate::OrientedBore {
                        radius: radius / 6.,
                        tilt: 0.5,
                        azimuth: 0.4 + std::f64::consts::PI / 2.,
                        center: [0., 8.],
                    },
                ],
                crate::GeometryTolerance::default(),
            )?
        }
        27 => {
            if !radius.is_finite() || !(8.0..=24.0).contains(&radius) {
                return Err(Error::Unsupported(
                    "divergent bore control must be in 8..24",
                ));
            }
            crate::tilted_bores_demo_solid(
                24.,
                8.,
                &[-1., 1.].map(|sign| crate::TiltedBore {
                    radius: radius / 6.,
                    tilt: sign * 0.7,
                    center: [0., sign * 5.],
                }),
                crate::GeometryTolerance::default(),
            )?
        }
        26 => {
            if !radius.is_finite() || !(8.0..=24.0).contains(&radius) {
                return Err(Error::Unsupported("parallel bore control must be in 8..24"));
            }
            crate::tilted_bores_demo_solid(
                24.,
                8.,
                &[-5., 5.].map(|y| crate::TiltedBore {
                    radius: radius / 6.,
                    tilt: 1.,
                    center: [0., y],
                }),
                crate::GeometryTolerance::default(),
            )?
        }
        25 => {
            if !radius.is_finite() || !(8.0..=24.0).contains(&radius) {
                return Err(Error::Unsupported("tilted bore control must be in 8..24"));
            }
            crate::tilted_bore_demo_solid(
                24.,
                radius * 0.5,
                16.,
                0.5,
                crate::GeometryTolerance::default(),
            )?
        }
        23 => crate::skew_face_subdivision_demo(radius / 32., 0.)?,
        22 => crate::skew_arc_extrusion_demo(14., radius, -24.)?,
        21 => crate::framed_arc_extrusion_demo(radius)?,
        20 => crate::simplified_contact_demo(radius - 16.0)?,
        19 => crate::reframed_merge_demo(radius - 16.0)?,
        18 => crate::merged_contact_demo(radius - 16.0)?,
        17 => crate::box_contact_demo(radius - 16.0)?,
        16 => crate::booleans::convex_union_demo(radius - 16.0)?,
        15 => match crate::booleans::convex_difference_demo(radius - 16.0)? {
            SolidDifference::Solid(s) => s,
            SolidDifference::Empty => return Err(Error::Unsupported("demo difference is empty")),
        },
        14 => match crate::booleans::convex_intersection_demo(radius - 16.0)? {
            SolidIntersection::Solid(s) => s,
            SolidIntersection::Empty => {
                return Err(Error::Unsupported("demo intersection is empty"))
            }
        },
        13 => {
            split_solid_by_plane(
                &make_box(b, tol)?,
                &Surface::Plane {
                    origin: Point3::new(0.0, 0.0, radius - 16.0),
                    u: Vec3::new(1.0, 0.0, 0.0),
                    v: Vec3::new(0.0, 1.0, 0.0),
                },
                GeometryTolerance::default(),
            )?
            .positive
        }
        12 => {
            if !radius.is_finite() || !(8.0..=24.0).contains(&radius) {
                return Err(Error::InvalidInput(
                    "sewing subdivision control must be in 8..24",
                ));
            }
            let source = make_box(b, tol)?;
            let mut patches = planar_face_patches(&source, tol)?;
            let ring = &mut patches[1].rings[0];
            ring.insert(1, ring[0] + (ring[1] - ring[0]) * (radius / 32.0));
            patches.reverse();
            sew_planar_faces(&patches, GeometryTolerance::default())?
        }
        11 => {
            subdivide_planar_face(
                &subtract_through_cylinder(b, tool(0.0, 0.0, 12.0), tol)?,
                1,
                Point3::new(0.0, radius - 16.0, 12.0),
                Vec3::new(1.0, 0.0, 0.0),
                GeometryTolerance::default(),
            )?
            .solid
        }
        10 => {
            let ring = |radius| {
                [0.0, std::f64::consts::PI]
                    .map(|start_angle| PlanarSegment::Arc {
                        center: [0.0, 0.0],
                        radius,
                        start_angle,
                        sweep: std::f64::consts::PI,
                    })
                    .to_vec()
            };
            subdivide_planar_face(
                &extrude_arc_line_region(
                    &ArcLineRegion {
                        origin: Point3::new(0.0, 0.0, -12.0),
                        outer: ring(30.0),
                        holes: vec![ring(26.0)],
                    },
                    24.0,
                    tol,
                )?,
                1,
                Point3::new(0.0, radius, 12.0),
                Vec3::new(1.0, 0.0, 0.0),
                GeometryTolerance::default(),
            )?
            .solid
        }
        9 => {
            subdivide_planar_face(
                &extrude_polygon(
                    &PolygonProfile {
                        origin: Point3::new(0.0, 0.0, -12.0),
                        outer: vec![[-40.0, -30.0], [40.0, -30.0], [40.0, 30.0], [-40.0, 30.0]],
                        holes: vec![
                            vec![[-24.0, -10.0], [-12.0, -10.0], [-12.0, 10.0], [-24.0, 10.0]],
                            vec![[12.0, -10.0], [24.0, -10.0], [24.0, 10.0], [12.0, 10.0]],
                        ],
                    },
                    Vec3::new(0.0, 0.0, 24.0),
                    tol,
                )?,
                1,
                Point3::new(0.0, radius - 16.0, 12.0),
                Vec3::new(1.0, 0.0, 0.0),
                GeometryTolerance::default(),
            )?
            .solid
        }
        8 => {
            split_planar_face(
                &extrude_arc_line(
                    &rounded_rectangle_profile(
                        Point3::new(0.0, 0.0, -12.0),
                        80.0,
                        60.0,
                        24.0,
                        tol,
                    )?,
                    24.0,
                    tol,
                )?,
                1,
                Point3::new(0.0, radius, 12.0),
                Vec3::new(1.0, 0.0, 0.0),
                GeometryTolerance::default(),
            )?
            .solid
        }
        7 => {
            split_planar_face(
                &subtract_through_cylinder(b, tool(0.0, 0.0, 7.0), tol)?,
                1,
                Point3::new(0.0, radius, 12.0),
                Vec3::new(1.0, 0.0, 0.0),
                GeometryTolerance::default(),
            )?
            .solid
        }
        6 => extrude_arc_line_region(&crate::mixed::notched_demo_profile(radius, tol)?, 24.0, tol)?,
        5 => extrude_arc_line(
            &rounded_rectangle_profile(Point3::new(0.0, 0.0, -12.0), 80.0, 60.0, radius, tol)?,
            24.0,
            tol,
        )?,
        3 => make_tube(
            TubeSpec {
                base: Point3::new(0.0, 0.0, -12.0),
                outer_radius: 30.0,
                inner_radius: radius,
                height: 24.0,
            },
            tol,
        )?,
        _ => return Err(Error::Unsupported("unknown demo preset")),
    };
    s.mesh_json(chord_error, tol)
}
impl Solid {
    /// Display-only JSON transport for the WebGL demo. Not a B-rep interchange format.
    pub fn mesh_json(&self, chord_error: f64, tol: Tolerance) -> Result<String> {
        let m = self.tessellate(chord_error, tol)?;
        let mut out = format!(
            "{{\"volume\":{},\"faces\":{},\"edges\":{},\"positions\":[",
            self.volume()?,
            self.shell.faces.len(),
            self.edges.len()
        );
        for (i, p) in m.positions.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str(&format!("{},{},{}", p.x, p.y, p.z));
        }
        out.push_str("],\"normals\":[");
        for (i, p) in m.normals.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str(&format!("{},{},{}", p.x, p.y, p.z));
        }
        out.push_str("]}");
        Ok(out)
    }
}

// Earcut can collapse collinear hole-bridge vertices. Restore them on every
// triangle edge so caps share exactly the same boundary segments as the walls.
fn conforming_triangles(
    coords: &[f64],
    indices: &[usize],
    candidates: &[usize],
) -> Result<Vec<usize>> {
    use crate::predicates::{orient2d, Orientation};
    let point = |i: usize| [coords[2 * i], coords[2 * i + 1]];
    let mut pending: Vec<[usize; 3]> = indices.as_chunks::<3>().0.to_vec();
    let mut result = Vec::new();
    while let Some(tri) = pending.pop() {
        let mut split = None;
        'edges: for e in 0..3 {
            let a = point(tri[e]);
            let b = point(tri[(e + 1) % 3]);
            for &i in candidates {
                if tri.contains(&i) {
                    continue;
                }
                let p = point(i);
                if p[0] < a[0].min(b[0])
                    || p[0] > a[0].max(b[0])
                    || p[1] < a[1].min(b[1])
                    || p[1] > a[1].max(b[1])
                    || p == a
                    || p == b
                {
                    continue;
                }
                if orient2d(a, b, p)? == Orientation::Collinear {
                    split = Some((e, i));
                    break 'edges;
                }
            }
        }
        if let Some((e, i)) = split {
            pending.push([tri[e], i, tri[(e + 2) % 3]]);
            pending.push([i, tri[(e + 1) % 3], tri[(e + 2) % 3]]);
        } else {
            result.extend(tri);
        }
    }
    Ok(result)
}
