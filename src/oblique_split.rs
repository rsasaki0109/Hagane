//! Exact transverse plane subdivision of bounded circular extrusion walls.
use crate::*;

#[derive(Clone, Debug)]
pub struct PlaneBoundarySubdivision {
    pub solid: Solid,
    /// Original face slot and appended child slot for every crossed face.
    pub split_faces: Vec<[usize; 2]>,
    /// Shared exact line/ellipse edges on the cutting plane; no interior cap.
    pub section_edges: Vec<usize>,
}
#[derive(Clone, Copy)]
struct EdgeSplit {
    vertex: usize,
    second: usize,
    fraction: f64,
}

fn affine(edge: usize, forward: bool, origin: [f64; 2], end: [f64; 2]) -> Coedge {
    Coedge {
        edge,
        forward,
        pcurve: PCurve::Affine {
            origin,
            direction: [end[0] - origin[0], end[1] - origin[1]],
        },
    }
}
fn expanded(c: &Coedge, split: Option<EdgeSplit>) -> Result<Vec<Coedge>> {
    let Some(split) = split else {
        return Ok(vec![c.clone()]);
    };
    let PCurve::Affine { origin, direction } = c.pcurve else {
        return Err(Error::Unsupported(
            "crossed straight edges require affine pcurves",
        ));
    };
    let middle = [
        origin[0] + direction[0] * split.fraction,
        origin[1] + direction[1] * split.fraction,
    ];
    let end = [origin[0] + direction[0], origin[1] + direction[1]];
    let a = affine(c.edge, c.forward, origin, middle);
    let b = affine(split.second, c.forward, middle, end);
    Ok(if c.forward { vec![a, b] } else { vec![b, a] })
}

/// Subdivide the boundary along a transverse plane, retaining one closed solid.
/// Bounded circular walls must be crossed strictly between both complete rims.
/// Crossed planar neighbors require quadrilateral straight-edge trims. The plane
/// may be oblique to the extrusion; its circular-wall sections are exact ellipses.
/// Rim contacts, partial wall crossings, periodic circles and repeated graph-band
/// cuts are unsupported. No material is removed and no mesh performs the cut.
pub fn subdivide_extrusion_boundary_by_plane(
    solid: &Solid,
    origin: Point3,
    normal: Vec3,
    tol: GeometryTolerance,
) -> Result<PlaneBoundarySubdivision> {
    solid.validate(tol.absolute())?;
    if !origin.finite() {
        return Err(Error::InvalidInput("cut plane requires a finite origin"));
    }
    let normal = normal.normalized()?;
    let bounds = solid.bounds();
    let scale = (bounds.max - bounds.min).norm();
    let budget = tol.length_at_scale(scale)?;
    let world_scale = [
        bounds.min.x,
        bounds.min.y,
        bounds.min.z,
        bounds.max.x,
        bounds.max.y,
        bounds.max.z,
        origin.x,
        origin.y,
        origin.z,
    ]
    .into_iter()
    .map(f64::abs)
    .fold(scale, f64::max);
    if !world_scale.is_finite() || 128. * f64::EPSILON * world_scale >= budget {
        return Err(Error::Unsupported(
            "cut plane has insufficient world-coordinate precision",
        ));
    }
    let mut bands = vec![None; solid.shell.faces.len()];
    let mut circular = 0;
    for (fi, face) in solid.shell.faces.iter().enumerate() {
        if matches!(face.surface, Surface::Plane { .. }) {
            continue;
        }
        circular += 1;
        let span = face.cylinder_span()?;
        let (frame, radius, height, drift) = crate::circular_trims::surface_data(&face.surface)?;
        let c = &face.wires[0].coedges;
        if !matches!(solid.edges[c[0].edge].curve, Curve::Arc { .. })
            || !matches!(solid.edges[c[2].edge].curve, Curve::Arc { .. })
        {
            return Err(Error::Unsupported(
                "oblique boundary subdivision requires bounded arc rims",
            ));
        }
        let g = frame.vector(Vec3::new(drift[0], drift[1], 1.));
        let denominator = normal.dot(g);
        if !denominator.is_finite() || denominator.abs() <= tol.angular().sin() * g.norm() {
            return Err(Error::Unsupported(
                "cut plane is parallel or unresolved along circular generators",
            ));
        }
        let [x, y, _] = frame.axes();
        let coefficients = [
            normal.dot(origin - frame.origin()) / denominator,
            -radius * normal.dot(x) / denominator,
            -radius * normal.dot(y) / denominator,
        ];
        let (lo, hi) = crate::circular_trims::extrema(coefficients, span)?;
        let guard = 10. * budget / denominator.abs();
        if !guard.is_finite() || lo <= guard || height - hi <= guard {
            return Err(Error::Unsupported(
                "cut plane touches or crosses a circular rim",
            ));
        }
        bands[fi] = Some(coefficients);
    }
    if circular == 0 {
        return Err(Error::Unsupported(
            "this boundary operation requires bounded circular walls",
        ));
    }
    let mut splits = vec![None; solid.edges.len()];
    let mut output = solid.clone();
    let volume = solid.volume()?;
    for (index, edge) in solid.edges.iter().enumerate() {
        let Curve::Line { a, b } = edge.curve else {
            continue;
        };
        let pa = normal.dot(a - origin);
        let pb = normal.dot(b - origin);
        if !pa.is_finite() || !pb.is_finite() {
            return Err(Error::InvalidInput(
                "cut signed distances exceed finite range",
            ));
        }
        if pa.abs().min(pb.abs()) <= 10. * budget {
            return Err(Error::Unsupported(
                "cut plane touches an existing boundary vertex",
            ));
        }
        if pa.is_sign_positive() == pb.is_sign_positive() {
            continue;
        }
        let magnitude = pa.abs().max(pb.abs());
        let fraction = (pa.abs() / magnitude) / (pa.abs() / magnitude + pb.abs() / magnitude);
        let point = a + (b - a) * fraction;
        if !point.finite()
            || normal.dot(point - origin).abs() > budget
            || (point - a).norm() <= 10. * budget
            || (point - b).norm() <= 10. * budget
        {
            return Err(Error::Unsupported(
                "cut boundary intersection is unresolved",
            ));
        }
        let vertex = output.vertices.len();
        output.vertices.push(Vertex { point });
        let second = output.edges.len();
        output.edges.push(Edge {
            vertices: [vertex, edge.vertices[1]],
            curve: Curve::Line { a: point, b },
        });
        output.edges[index] = Edge {
            vertices: [edge.vertices[0], vertex],
            curve: Curve::Line { a, b: point },
        };
        splits[index] = Some(EdgeSplit {
            vertex,
            second,
            fraction,
        });
    }
    let mut split_faces = Vec::new();
    let mut section_edges = Vec::new();
    for (fi, face) in solid.shell.faces.iter().enumerate() {
        let crossings: Vec<_> = face
            .wires
            .iter()
            .flat_map(|w| &w.coedges)
            .filter_map(|c| splits[c.edge].map(|s| s.vertex))
            .collect();
        if crossings.is_empty() {
            continue;
        }
        if crossings.len() != 2
            || crossings[0] == crossings[1]
            || face.wires.len() != 1
            || face.wires[0].coedges.len() != 4
        {
            return Err(Error::Unsupported(
                "crossed faces require two opposite generator intersections and one four-edge wire",
            ));
        }
        let section = output.edges.len();
        let children = if let Some(coeff) = bands[fi] {
            let span = face.circular_span()?;
            let (frame, radius, height, drift) =
                crate::circular_trims::surface_data(&face.surface)?;
            let c = &face.wires[0].coedges;
            let right = splits[c[1].edge]
                .ok_or(Error::InvalidTopology("cut misses the right generator"))?;
            let left =
                splits[c[3].edge].ok_or(Error::InvalidTopology("cut misses the left generator"))?;
            let a = crate::circular_trims::value(coeff, 0.);
            let b = crate::circular_trims::value(coeff, span);
            let curve = Curve::EllipseArc {
                center: frame.point(Vec3::new(
                    drift[0] * coeff[0],
                    drift[1] * coeff[0],
                    coeff[0],
                )),
                cosine: frame.vector(Vec3::new(
                    radius + drift[0] * coeff[1],
                    drift[1] * coeff[1],
                    coeff[1],
                )),
                sine: frame.vector(Vec3::new(
                    drift[0] * coeff[2],
                    radius + drift[1] * coeff[2],
                    coeff[2],
                )),
                sweep: span,
            };
            output.edges.push(Edge {
                vertices: [left.vertex, right.vertex],
                curve,
            });
            let graph = PCurve::HeightGraph {
                offset: coeff[0],
                cosine: coeff[1],
                sine: coeff[2],
                sweep: span,
            };
            let lower = vec![
                c[0].clone(),
                affine(c[1].edge, true, [span, 0.], [span, b]),
                Coedge {
                    edge: section,
                    forward: false,
                    pcurve: graph.clone(),
                },
                affine(c[3].edge, false, [0., 0.], [0., a]),
            ];
            let upper = vec![
                Coedge {
                    edge: section,
                    forward: true,
                    pcurve: graph,
                },
                affine(right.second, true, [span, b], [span, height]),
                c[2].clone(),
                affine(left.second, false, [0., a], [0., height]),
            ];
            [lower, upper]
        } else {
            if !matches!(face.surface, Surface::Plane { .. })
                || face.wires[0]
                    .coedges
                    .iter()
                    .any(|c| !matches!(solid.edges[c.edge].curve, Curve::Line { .. }))
            {
                return Err(Error::Unsupported(
                    "crossed planar neighbors require straight quadrilaterals",
                ));
            }
            let mut wire = Vec::new();
            for c in &face.wires[0].coedges {
                wire.extend(expanded(c, splits[c.edge])?);
            }
            let start = |c: &Coedge| output.edges[c.edge].vertices[usize::from(!c.forward)];
            let va = crossings[0];
            let vb = crossings[1];
            let ia = wire
                .iter()
                .position(|c| start(c) == va)
                .ok_or(Error::InvalidTopology(
                    "first cut vertex absent from planar wire",
                ))?;
            let ib = wire
                .iter()
                .position(|c| start(c) == vb)
                .ok_or(Error::InvalidTopology(
                    "second cut vertex absent from planar wire",
                ))?;
            let path = |from: usize, to: usize, forward: bool| {
                let mut result = Vec::new();
                let mut i = from;
                while i != to {
                    result.push(wire[i].clone());
                    i = (i + 1) % wire.len();
                }
                let a = face.surface.parameters(output.vertices[va].point);
                let b = face.surface.parameters(output.vertices[vb].point);
                result.push(affine(section, forward, a, b));
                result
            };
            let children = [path(ia, ib, false), path(ib, ia, true)];
            output.edges.push(Edge {
                vertices: [va, vb],
                curve: Curve::Line {
                    a: output.vertices[va].point,
                    b: output.vertices[vb].point,
                },
            });
            children
        };
        output.shell.faces[fi] = Face {
            surface: face.surface.clone(),
            orientation: face.orientation,
            wires: vec![Wire {
                coedges: children[0].clone(),
            }],
        };
        let second = output.shell.faces.len();
        output.shell.faces.push(Face {
            surface: face.surface.clone(),
            orientation: face.orientation,
            wires: vec![Wire {
                coedges: children[1].clone(),
            }],
        });
        split_faces.push([fi, second]);
        section_edges.push(section);
    }
    if split_faces.is_empty() {
        return Err(Error::Unsupported(
            "plane does not subdivide the extrusion boundary",
        ));
    }
    output.validate(tol.absolute())?;
    if (output.volume()? - volume).abs() > volume.abs() * 1e-10 {
        return Err(Error::InvalidTopology(
            "oblique boundary subdivision changes analytic volume",
        ));
    }
    Ok(PlaneBoundarySubdivision {
        solid: output,
        split_faces,
        section_edges,
    })
}

fn demo_fixture(slope: f64) -> Result<PlaneBoundarySubdivision> {
    if !slope.is_finite() {
        return Err(Error::InvalidInput("oblique demo requires finite slope"));
    }
    let source = crate::classification::curved_classification_solid(5)?;
    subdivide_extrusion_boundary_by_plane(
        &source,
        Point3::new(6., -3., 0.),
        Vec3::new(slope, -0.08, 1.),
        GeometryTolerance::default(),
    )
}
pub fn oblique_boundary_demo(slope: f64, placement: f64) -> Result<Solid> {
    let source = demo_fixture(slope)?.solid;
    source.transformed(
        Transform::rotation(Vec3::new(1., 2., 3.), placement)?,
        Tolerance::default(),
    )
}
pub fn oblique_boundary_demo_json(slope: f64, placement: f64) -> Result<String> {
    oblique_boundary_mesh_json(slope, placement, 0.05)
}
pub(crate) fn oblique_boundary_mesh_json(slope: f64, placement: f64, error: f64) -> Result<String> {
    let source = demo_fixture(slope)?;
    let solid = source.solid.transformed(
        Transform::rotation(Vec3::new(1., 2., 3.), placement)?,
        Tolerance::default(),
    )?;
    let mut json = solid.mesh_json(error, Tolerance::default())?;
    let counts = crate::mesh::shared_edge_counts(&solid, error)?;
    let mut segments = Vec::new();
    for index in source.section_edges {
        let curve = &solid.edges[index].curve;
        let count = counts[index];
        let end = curve.range()[1];
        for i in 0..count {
            let a = curve.evaluate(end * i as f64 / count as f64);
            let b = curve.evaluate(end * (i + 1) as f64 / count as f64);
            segments.push(format!(
                "[[{},{},{}],[{},{},{}]]",
                a.x, a.y, a.z, b.x, b.y, b.z
            ));
        }
    }
    json.pop();
    json.push_str(&format!(",\"section_segments\":[{}]}}", segments.join(",")));
    Ok(json)
}
