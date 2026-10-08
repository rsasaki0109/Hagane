//! Checked line intersections with rectangular and harmonic circular face trims.
use crate::*;
use std::f64::consts::TAU;
/// Boundary provenance follows the owning 3D edge parameter, independently of
/// coedge traversal and face orientation. Periodic seams retain both coedge uses.
#[derive(Clone, Debug, PartialEq)]
pub struct CircularFaceBoundary {
    pub wire: usize,
    pub coedge: usize,
    pub edge: usize,
    pub edge_parameter: f64,
}
#[derive(Clone, Debug, PartialEq)]
pub struct CircularFacePoint {
    pub point: Point3,
    pub parameter: f64,
    pub uv: [f64; 2],
    /// Unit normal including the owning face's orientation (including hole walls).
    pub normal: Vec3,
    /// Empty for interior hits. Nonperiodic corners have two uses; a full-periodic
    /// seam has two uses of its shared generator edge.
    pub boundaries: Vec<CircularFaceBoundary>,
}
#[derive(Clone, Debug, PartialEq)]
pub enum CircularFaceLineIntersection {
    Empty,
    /// Sorted by increasing parameter on the caller's original line direction.
    Points {
        points: Vec<CircularFacePoint>,
        contact: IntersectionContact,
    },
    /// Continuous generator overlap, with bounded endpoints and provenance.
    /// Start precedes end in line parameter, even for reversed generators.
    Coincident {
        start: CircularFacePoint,
        end: CircularFacePoint,
    },
}
/// Intersect a line with one validated circular face, including harmonic height bands.
/// Supports normal cylinders and skew circular translations. Angular trimming
/// preserves original boundary parameters. Uncertified near angular/elliptical
/// boundaries return errors; coplanar shared-ellipse incidence is certified exactly.
/// Full-periodic seams do not remove or duplicate surface hits. No mesh is used.
pub fn intersect_line_circular_face(
    solid: &Solid,
    face_index: usize,
    anchor: Point3,
    direction: Vec3,
    tol: GeometryTolerance,
) -> Result<CircularFaceLineIntersection> {
    solid.validate(tol.absolute())?;
    let face = solid
        .shell
        .faces
        .get(face_index)
        .ok_or(Error::InvalidInput("circular face index is out of range"))?;
    let (frame, radius, height, drift) = match face.surface {
        Surface::Cylinder {
            center,
            radius,
            height,
        } => (Frame3::translation(center)?, radius, height, [0., 0.]),
        Surface::FramedCylinder {
            frame,
            radius,
            height,
        } => (frame, radius, height, [0., 0.]),
        Surface::ExtrudedCircle {
            frame,
            radius,
            height,
            drift,
        } => (frame, radius, height, drift),
        _ => {
            return Err(Error::Unsupported(
                "circular face intersection requires a circular wall",
            ))
        }
    };
    let skew = drift[0].hypot(drift[1]);
    let extent = radius.max(height * skew.hypot(1.));
    let budget = tol.length_at_scale(extent)?;
    let angular_guard = budget * (1. + skew);
    if !angular_guard.is_finite() {
        return Err(Error::InvalidInput(
            "circular trim guard exceeds finite range",
        ));
    }
    let span = face.circular_span()?;
    let bands = face.circular_bands()?;
    let levels = |u: f64| {
        [
            crate::circular_trims::value(bands[0], u),
            crate::circular_trims::value(bands[1], u),
        ]
    };
    let graph_guards: Vec<_> = bands
        .iter()
        .map(|b| {
            let normal = Vec3::new(
                -b[1] / radius,
                -b[2] / radius,
                1. + (b[1] / radius) * drift[0] + (b[2] / radius) * drift[1],
            );
            let guard = budget * normal.norm();
            let roundoff = 256. * f64::EPSILON * (b[0].abs() + b[1].hypot(b[2]));
            if !guard.is_finite() || guard <= 0. || !roundoff.is_finite() || roundoff >= guard {
                Err(Error::Unsupported(
                    "harmonic trim height is unresolved at this precision",
                ))
            } else {
                Ok(guard)
            }
        })
        .collect::<Result<_>>()?;
    let surface_tol = GeometryTolerance::new(budget, tol.angular(), 0.)?;
    let accepted = |angle: f64| -> Result<bool> {
        if span == TAU || angle == 0. || angle == span {
            return Ok(true);
        }
        let gap = (2. * radius * (angle / 2.).sin().abs())
            .min(2. * radius * ((angle - span) / 2.).sin().abs());
        if gap <= angular_guard {
            return Err(Error::Unsupported(
                "circular face hit is unresolved near an angular trim boundary",
            ));
        }
        Ok(angle < span)
    };
    let location = |point: Point3, parameter: f64, uv: [f64; 2]| -> Result<CircularFacePoint> {
        let local = frame.local_point(point);
        let expected = Vec3::new(
            radius * uv[0].cos() + drift[0] * uv[1],
            radius * uv[0].sin() + drift[1] * uv[1],
            uv[1],
        );
        if !point.finite()
            || !parameter.is_finite()
            || !local.finite()
            || !expected.finite()
            || (local - expected).norm() > budget
        {
            return Err(Error::InvalidInput(
                "circular face hit loses local dimensions at world coordinate magnitude",
            ));
        }
        let mut boundaries = Vec::new();
        let mut indices = Vec::new();
        let [lower, upper] = levels(uv[0]);
        if uv[1] == lower {
            indices.push(0);
        }
        if uv[0] == span || (span == TAU && uv[0] == 0.) {
            indices.push(1);
        }
        if uv[1] == upper {
            indices.push(2);
        }
        if uv[0] == 0. {
            indices.push(3);
        }
        for index in indices {
            let c = &face.wires[0].coedges[index];
            let boundary_uv = if index == 1 && span == TAU {
                [span, uv[1]]
            } else {
                uv
            };
            let t = match c.pcurve {
                PCurve::Affine {
                    origin,
                    direction: pc_direction,
                } => {
                    let coordinate = usize::from(pc_direction[1].abs() > pc_direction[0].abs());
                    (boundary_uv[coordinate] - origin[coordinate]) / pc_direction[coordinate]
                }
                PCurve::HeightGraph { .. } if index == 0 || index == 2 => boundary_uv[0],
                _ => {
                    return Err(Error::Unsupported(
                        "circular boundary pcurve is unsupported",
                    ))
                }
            };
            let edge = &solid.edges[c.edge];
            let range = edge.curve.range();
            let pc_uv = c.pcurve.evaluate(t);
            if !t.is_finite()
                || t < range[0]
                || t > range[1]
                || !tol.coincident(point, edge.curve.evaluate(t), extent)?
                || !tol.coincident(point, face.surface.evaluate(pc_uv[0], pc_uv[1]), extent)?
            {
                return Err(Error::InvalidInput(
                    "circular face boundary loses original edge parameter agreement",
                ));
            }
            boundaries.push(CircularFaceBoundary {
                wire: 0,
                coedge: index,
                edge: c.edge,
                edge_parameter: t,
            });
        }
        Ok(CircularFacePoint {
            point,
            parameter,
            uv,
            normal: face.surface.normal(uv[0]) * face.orientation as f64,
            boundaries,
        })
    };
    let unit = direction.normalized()?;
    let coords = |p: Vec3| [p.x, p.y, p.z];
    let relation =
        |index: usize| -> Result<(Point3, Point3, crate::predicates::ExactLineRelation)> {
            let edge = &solid.edges[face.wires[0].coedges[index].edge];
            let Curve::Line { a, b } = edge.curve else {
                return Err(Error::InvalidTopology(
                    "circular generator boundary must be straight",
                ));
            };
            Ok((
                a,
                b,
                crate::predicates::exact_line_relation(
                    coords(anchor),
                    coords(direction),
                    coords(a),
                    coords(b),
                )?,
            ))
        };
    // A boundary generator can be certified using actual shared straight edges,
    // even when trig/frame roundoff makes the underlying surface solver ambiguous.
    for index in [1, 3] {
        let (a, b, r) = relation(index)?;
        if r == crate::predicates::ExactLineRelation::Collinear {
            let c = &face.wires[0].coedges[index];
            let mut ends = Vec::new();
            for (point, t) in [(a, 0.), (b, 1.)] {
                let travel = (point - anchor).dot(unit);
                let parameter = crate::intersections::line_parameter(travel, direction)?;
                if !tol.coincident(
                    point,
                    anchor + direction * parameter,
                    extent.max(travel.abs()),
                )? {
                    return Err(Error::InvalidInput(
                        "certified generator loses line parameter agreement",
                    ));
                }
                let mut uv = c.pcurve.evaluate(t);
                if span == TAU && uv[0] == TAU {
                    uv[0] = 0.;
                }
                uv[1] = levels(uv[0])[usize::from(t == 1.)];
                ends.push(location(point, parameter, uv)?);
            }
            ends.sort_by(|a, b| a.parameter.total_cmp(&b.parameter));
            if ends[1].parameter <= ends[0].parameter {
                return Err(Error::InvalidInput(
                    "certified generator interval is unrepresentable",
                ));
            }
            return Ok(CircularFaceLineIntersection::Coincident {
                end: ends.pop().unwrap(),
                start: ends.pop().unwrap(),
            });
        }
    }
    let boundary_hit = |candidate: Point3| -> Result<Option<CircularFacePoint>> {
        for index in [1, 3] {
            let (a, b, r) = relation(index)?;
            if r != crate::predicates::ExactLineRelation::Coplanar {
                continue;
            }
            let edge_direction = b - a;
            let length = edge_direction.norm();
            if !length.is_finite() {
                return Err(Error::InvalidInput(
                    "boundary generator length exceeds finite range",
                ));
            }
            let generator = edge_direction.normalized()?;
            let n = unit.cross(generator);
            let sine = n.norm();
            if sine <= tol.angular().sin() {
                return Err(Error::Unsupported(
                    "near-parallel circular boundary crossing is unresolved",
                ));
            }
            let offset = a - anchor;
            let denominator = n.dot(n);
            let travel = offset.cross(generator).dot(n) / denominator;
            let edge_travel = offset.cross(unit).dot(n) / denominator;
            let t = edge_travel / length;
            if !travel.is_finite()
                || !t.is_finite()
                || (edge_travel != 0. && t == 0.)
                || (edge_travel != length && t == 1.)
            {
                return Err(Error::InvalidInput(
                    "certified boundary crossing exceeds finite range",
                ));
            }
            if !(0. ..=1.).contains(&t) {
                continue;
            }
            let point = a + edge_direction * t;
            if (point - candidate).norm() > budget {
                continue;
            }
            let parameter = crate::intersections::line_parameter(travel, direction)?;
            if !tol.coincident(
                point,
                anchor + direction * parameter,
                extent.max(travel.abs()),
            )? {
                return Err(Error::InvalidInput(
                    "certified boundary crossing loses line agreement",
                ));
            }
            let mut uv = face.wires[0].coedges[index].pcurve.evaluate(t);
            if span == TAU && uv[0] == TAU {
                uv[0] = 0.;
            }
            if t == 0. || t == 1. {
                uv[1] = levels(uv[0])[usize::from(t == 1.)];
            }
            return Ok(Some(location(point, parameter, uv)?));
        }
        Ok(None)
    };
    let graph_hit = |candidate: Point3, uv: [f64; 2]| -> Result<Option<CircularFacePoint>> {
        for index in [0, 2] {
            let c = &face.wires[0].coedges[index];
            if !matches!(c.pcurve, PCurve::HeightGraph { .. }) {
                continue;
            }
            let edge = &solid.edges[c.edge];
            let Curve::EllipseArc {
                center,
                cosine,
                sine,
                sweep,
            } = edge.curve
            else {
                return Err(Error::InvalidTopology(
                    "harmonic boundary requires a stored ellipse edge",
                ));
            };
            if !crate::predicates::exact_line_in_ellipse_plane(
                coords(anchor),
                coords(direction),
                coords(center),
                coords(cosine),
                coords(sine),
            )? {
                continue;
            }
            let u = uv[0];
            if u < 0. || u > sweep {
                continue;
            }
            let point = edge.curve.evaluate(u);
            if (point - candidate).norm() > budget {
                continue;
            }
            let travel = (point - anchor).dot(unit);
            let parameter = crate::intersections::line_parameter(travel, direction)?;
            if !tol.coincident(
                point,
                anchor + direction * parameter,
                extent.max(travel.abs()),
            )? {
                return Err(Error::Unsupported(
                    "certified ellipse hit loses original line agreement",
                ));
            }
            return Ok(Some(location(point, parameter, c.pcurve.evaluate(u))?));
        }
        Ok(None)
    };
    let clip_height =
        |point: Point3, parameter: f64, uv: [f64; 2]| -> Result<Option<CircularFacePoint>> {
            let [lower, upper] = levels(uv[0]);
            let near_lower = (uv[1] - lower).abs() <= graph_guards[0];
            let near_upper = (uv[1] - upper).abs() <= graph_guards[1];
            let harmonic_near = (near_lower
                && matches!(face.wires[0].coedges[0].pcurve, PCurve::HeightGraph { .. }))
                || (near_upper
                    && matches!(face.wires[0].coedges[2].pcurve, PCurve::HeightGraph { .. }));
            if harmonic_near {
                if near_lower && near_upper {
                    return Err(Error::Unsupported(
                        "harmonic boundary tolerance bands overlap at the hit",
                    ));
                }
                return graph_hit(point, uv)?.map(Some).ok_or(Error::Unsupported(
                    "circular hit is unresolved near an ellipse boundary",
                ));
            }
            if uv[1] < lower || uv[1] > upper {
                return Ok(None);
            }
            Ok(Some(location(point, parameter, uv)?))
        };
    let result = if matches!(face.surface, Surface::ExtrudedCircle { .. }) {
        intersect_line_extruded_circle(anchor, direction, &face.surface, surface_tol)?
    } else {
        intersect_line_cylinder(anchor, direction, &face.surface, surface_tol)?
    };
    Ok(match result {
        LineCylinderIntersection::Empty => CircularFaceLineIntersection::Empty,
        LineCylinderIntersection::Points(points) => {
            let contact = points
                .first()
                .ok_or(Error::InvalidInput("surface returned an empty point list"))?
                .contact;
            let mut retained = Vec::new();
            for hit in points {
                match accepted(hit.uv[0]) {
                    Ok(true) => {
                        if let Some(point) = clip_height(hit.point, hit.parameter, hit.uv)? {
                            retained.push(point);
                        }
                    }
                    Ok(false) => (),
                    Err(error) => match boundary_hit(hit.point)? {
                        Some(point) => retained.push(point),
                        None => return Err(error),
                    },
                }
            }
            if retained.is_empty() {
                CircularFaceLineIntersection::Empty
            } else {
                CircularFaceLineIntersection::Points {
                    points: retained,
                    contact,
                }
            }
        }
        LineCylinderIntersection::Coincident {
            parameter_range,
            angle,
        } => {
            if !accepted(angle)? {
                return Ok(CircularFaceLineIntersection::Empty);
            }
            if bands == [[0., 0., 0.], [height, 0., 0.]] {
                let first = anchor + direction * parameter_range[0];
                let last = anchor + direction * parameter_range[1];
                let first_v = face.surface.parameters(first)[1];
                let last_v = face.surface.parameters(last)[1];
                let levels = if first_v < last_v {
                    [0., height]
                } else {
                    [height, 0.]
                };
                CircularFaceLineIntersection::Coincident {
                    start: location(first, parameter_range[0], [angle, levels[0]])?,
                    end: location(last, parameter_range[1], [angle, levels[1]])?,
                }
            } else {
                let mut ends = Vec::new();
                for v in levels(angle) {
                    let point = face.surface.evaluate(angle, v);
                    let travel = (point - anchor).dot(unit);
                    let parameter = crate::intersections::line_parameter(travel, direction)?;
                    if !tol.coincident(
                        point,
                        anchor + direction * parameter,
                        extent.max(travel.abs()),
                    )? {
                        return Err(Error::Unsupported(
                            "trimmed generator endpoint loses line agreement",
                        ));
                    }
                    ends.push(location(point, parameter, [angle, v])?);
                }
                ends.sort_by(|a, b| a.parameter.total_cmp(&b.parameter));
                if ends[0].parameter >= ends[1].parameter {
                    return Err(Error::Unsupported(
                        "trimmed generator interval is unrepresentable",
                    ));
                }
                CircularFaceLineIntersection::Coincident {
                    end: ends.pop().unwrap(),
                    start: ends.pop().unwrap(),
                }
            }
        }
    })
}
/// Native/WASM fixture: select either actual semicircular wall of the skew solid.
/// Selection 0/1 uses the first/second wall, modes match the surface query demo.
pub fn circular_face_intersections_demo_json(
    selection: u32,
    mode: u32,
    offset: f64,
    placement: f64,
) -> Result<String> {
    if selection > 1 {
        return Err(Error::InvalidInput(
            "circular face demo selection must be 0 or 1",
        ));
    }
    let (solid, a, d, _) =
        crate::extrusion_intersections::extrusion_intersections_fixture(mode, offset, placement)?;
    let face = 2 + selection as usize;
    circular_face_query_json(&solid, face, a, d)
}
pub(crate) fn circular_face_query_json(
    solid: &Solid,
    face: usize,
    a: Point3,
    d: Vec3,
) -> Result<String> {
    let t = GeometryTolerance::default();
    let xyz = |p: Vec3| format!("[{},{},{}]", p.x, p.y, p.z);
    let point_json = |p: &CircularFacePoint, contact: Option<IntersectionContact>| {
        let boundaries: Vec<_> = p
            .boundaries
            .iter()
            .map(|b| {
                format!(
                    "{{\"wire\":{},\"coedge\":{},\"edge\":{},\"edge_parameter\":{}}}",
                    b.wire, b.coedge, b.edge, b.edge_parameter
                )
            })
            .collect();
        let contact = contact
            .map(|c| {
                format!(
                    ",\"contact\":\"{}\"",
                    if c == IntersectionContact::Tangent {
                        "tangent"
                    } else {
                        "crossing"
                    }
                )
            })
            .unwrap_or_default();
        format!("{{\"point\":{},\"normal\":{},\"parameter\":{},\"uv\":[{},{}],\"boundaries\":[{}]{contact}}}",xyz(p.point),xyz(p.normal),p.parameter,p.uv[0],p.uv[1],boundaries.join(","))
    };
    let result = match intersect_line_circular_face(solid, face, a, d, t)? {
        CircularFaceLineIntersection::Empty => "{\"kind\":\"empty\",\"hits\":[]}".into(),
        CircularFaceLineIntersection::Points { points, contact } => format!(
            "{{\"kind\":\"points\",\"hits\":[{}]}}",
            points
                .iter()
                .map(|p| point_json(p, Some(contact)))
                .collect::<Vec<_>>()
                .join(",")
        ),
        CircularFaceLineIntersection::Coincident { start, end } => format!(
            "{{\"kind\":\"coincident\",\"hits\":[],\"range\":[{},{}],\"endpoints\":[{},{}]}}",
            start.parameter,
            end.parameter,
            point_json(&start, None),
            point_json(&end, None)
        ),
    };
    let mesh = solid.tessellate(0.05, t.absolute())?;
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    for (i, tri) in mesh.triangles.iter().enumerate() {
        if mesh.face_ids[i] == face {
            for vertex in tri {
                positions.push(xyz(mesh.positions[*vertex]));
                normals.push(xyz(mesh.normals[*vertex]));
            }
        }
    }
    // Flatten arrays for the existing display transport; this is only a selected
    // face's rendering, while volume/topology continue belonging to the solid.
    let flatten = |values: Vec<String>| {
        values
            .into_iter()
            .map(|s| s[1..s.len() - 1].to_string())
            .collect::<Vec<_>>()
            .join(",")
    };
    Ok(format!("{{\"intersection\":{result},\"line\":{{\"anchor\":{},\"direction\":{}}},\"face\":{face},\"display_mesh\":{{\"positions\":[{}],\"normals\":[{}]}},\"mesh\":{}}}",xyz(a),xyz(d),flatten(positions),flatten(normals),solid.mesh_json(0.05,t.absolute())?))
}

/// Query either height band of a plane-subdivided skew circular B-rep wall.
/// Modes 0..2 are transverse/forward/reverse generators; mode 3 lies in the
/// actual shared ellipse plane. Offset in mode 3 translates along world Z.
pub fn harmonic_face_intersections_demo_json(
    selection: u32,
    mode: u32,
    offset: f64,
    placement: f64,
) -> Result<String> {
    if selection > 1 || mode > 3 || !offset.is_finite() || !placement.is_finite() {
        return Err(Error::InvalidInput(
            "harmonic face demo requires selection 0..1, mode 0..3 and finite values",
        ));
    }
    let (source, a, d, _) =
        crate::extrusion_intersections::extrusion_intersections_fixture(mode.min(2), offset, 0.)?;
    let cut = subdivide_extrusion_boundary_by_plane(
        &source,
        Point3::new(6., -3., 0.),
        Vec3::new(0.25, 0., 1.),
        GeometryTolerance::default(),
    )?;
    let children =
        cut.split_faces
            .iter()
            .find(|pair| pair[0] == 2)
            .ok_or(Error::InvalidTopology(
                "demo circular face was not subdivided",
            ))?;
    let face = children[selection as usize];
    let transform = Transform::rotation(Vec3::new(1., 2., 3.), placement)?;
    let solid = cut
        .solid
        .transformed(transform, GeometryTolerance::default().absolute())?;
    let (anchor, direction) = if mode == 3 {
        let coedge = &solid.shell.faces[face].wires[0].coedges[if selection == 0 { 2 } else { 0 }];
        let Curve::EllipseArc { center, sine, .. } = solid.edges[coedge.edge].curve else {
            return Err(Error::InvalidTopology("demo section must be an ellipse"));
        };
        (center + Vec3::new(0., 0., offset), sine)
    } else {
        (transform.point(a), transform.vector(d))
    };
    circular_face_query_json(&solid, face, anchor, direction)
}
