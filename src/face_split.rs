//! Scoped planar face subdivision, preserving a closed shared-edge B-rep.
use crate::*;
#[derive(Clone, Debug)]
pub struct PlanarFaceSplit {
    pub solid: Solid,
    /// First child replaces the input face; second child is appended.
    pub faces: [usize; 2],
    pub cut_edge: usize,
    pub cut_vertices: [usize; 2],
}
fn start_vertex(s: &Solid, c: &Coedge) -> usize {
    s.edges[c.edge].vertices[usize::from(!c.forward)]
}
fn split_line_edge(
    s: &mut Solid,
    index: usize,
    t: f64,
    point: Point3,
    tol: Tolerance,
) -> Result<usize> {
    let old = s.edges[index].clone();
    let Curve::Line { a, b } = old.curve else {
        return Err(Error::Unsupported(
            "face splitting requires straight boundary edges",
        ));
    };
    if (point - a).norm() <= 10.0 * tol.linear || (point - b).norm() <= 10.0 * tol.linear {
        return Err(Error::Unsupported(
            "split boundary subedges must exceed ten tolerances",
        ));
    }
    for f in &s.shell.faces {
        for w in &f.wires {
            for c in &w.coedges {
                if c.edge == index
                    && (!matches!(f.surface, Surface::Plane { .. })
                        || !matches!(c.pcurve, PCurve::Affine { .. }))
                {
                    return Err(Error::Unsupported(
                        "straight split edge neighbors require planar affine trims",
                    ));
                }
            }
        }
    }
    let vertex = s.vertices.len();
    s.vertices.push(Vertex { point });
    let new = s.edges.len();
    s.edges.push(Edge {
        vertices: [vertex, old.vertices[1]],
        curve: Curve::Line { a: point, b },
    });
    s.edges[index] = Edge {
        vertices: [old.vertices[0], vertex],
        curve: Curve::Line { a, b: point },
    };
    for f in &mut s.shell.faces {
        for w in &mut f.wires {
            let mut coedges = Vec::new();
            for c in &w.coedges {
                if c.edge != index {
                    coedges.push(c.clone());
                    continue;
                }
                let PCurve::Affine { origin, direction } = c.pcurve else {
                    unreachable!()
                };
                let first = Coedge {
                    edge: index,
                    forward: c.forward,
                    pcurve: PCurve::Affine {
                        origin,
                        direction: direction.map(|d| d * t),
                    },
                };
                let second = Coedge {
                    edge: new,
                    forward: c.forward,
                    pcurve: PCurve::Affine {
                        origin: [origin[0] + direction[0] * t, origin[1] + direction[1] * t],
                        direction: direction.map(|d| d * (1.0 - t)),
                    },
                };
                if c.forward {
                    coedges.extend([first, second]);
                } else {
                    coedges.extend([second, first]);
                }
            }
            w.coedges = coedges;
        }
    }
    Ok(vertex)
}
/// Splits a planar line/arc face along one proper interior interval. Exactly two
/// crossings on distinct outer edges are required. Holes cannot be crossed.
/// Arc cuts refine their rectangular cylinder walls and opposite rims. Edge uses are split
/// and reparameterized atomically on a clone; unsupported inputs return errors.
pub fn split_planar_face(
    solid: &Solid,
    face_index: usize,
    anchor: Point3,
    direction: Vec3,
    tol: GeometryTolerance,
) -> Result<PlanarFaceSplit> {
    let clip = clip_line_to_planar_face(solid, face_index, anchor, direction, tol)?;
    let face = &solid.shell.faces[face_index];
    if face.wires[0].coedges.iter().any(|c| {
        matches!(
            solid.edges[c.edge].curve,
            Curve::Circle { .. } | Curve::FramedCircle { .. }
        )
    }) {
        return Err(Error::Unsupported(
            "split circular outer wires into bounded arcs before face subdivision",
        ));
    }
    if clip.events.len() != 2
        || clip.intervals.len() != 1
        || clip.events.iter().any(|e| e.wire != 0)
        || clip.events[0].edge == clip.events[1].edge
    {
        return Err(Error::Unsupported(
            "face split requires one interior interval with two distinct outer-edge crossings",
        ));
    }
    let original_volume = solid.volume()?;
    let mut s = solid.clone();
    let va = split_boundary_edge(
        &mut s,
        clip.events[0].edge,
        clip.events[0].edge_parameter,
        clip.events[0].point,
        tol.absolute(),
    )?;
    let vb = split_boundary_edge(
        &mut s,
        clip.events[1].edge,
        clip.events[1].edge_parameter,
        clip.events[1].point,
        tol.absolute(),
    )?;
    let face = s.shell.faces[face_index].clone();
    let outer = &face.wires[0].coedges;
    let find = |vertex| {
        outer
            .iter()
            .position(|c| start_vertex(&s, c) == vertex)
            .ok_or(Error::InvalidTopology("split vertex not on outer wire"))
    };
    let ia = find(va)?;
    let ib = find(vb)?;
    let path = |start: usize, end: usize| {
        let mut result = Vec::new();
        let mut i = start;
        while i != end {
            result.push(outer[i].clone());
            i = (i + 1) % outer.len();
        }
        result
    };
    let pa = s.vertices[va].point;
    let pb = s.vertices[vb].point;
    let uv_a = face.surface.parameters(pa);
    let uv_b = face.surface.parameters(pb);
    let cut = s.edges.len();
    s.edges.push(Edge {
        vertices: [va, vb],
        curve: Curve::Line { a: pa, b: pb },
    });
    let chord = |forward| Coedge {
        edge: cut,
        forward,
        pcurve: PCurve::Affine {
            origin: uv_a,
            direction: [uv_b[0] - uv_a[0], uv_b[1] - uv_a[1]],
        },
    };
    let mut a = path(ia, ib);
    a.push(chord(false));
    let mut b = path(ib, ia);
    b.push(chord(true));
    let analytic_ring = |coedges: &[Coedge]| {
        crate::face_intersections::rings(&Face {
            surface: face.surface.clone(),
            orientation: face.orientation,
            wires: vec![Wire {
                coedges: coedges.to_vec(),
            }],
        })
        .remove(0)
    };
    let ap = analytic_ring(&a);
    let bp = analytic_ring(&b);
    let mut aw = vec![Wire { coedges: a }];
    let mut bw = vec![Wire { coedges: b }];
    for hole in &face.wires[1..] {
        let c = &hole.coedges[0];
        let range = s.edges[c.edge].curve.range();
        let witness = c.pcurve.evaluate(range[usize::from(!c.forward)]);
        match (
            crate::mixed::point_location(witness, &ap, tol.absolute())?,
            crate::mixed::point_location(witness, &bp, tol.absolute())?,
        ) {
            (PointLocation::Inside, PointLocation::Outside) => aw.push(hole.clone()),
            (PointLocation::Outside, PointLocation::Inside) => bw.push(hole.clone()),
            _ => return Err(Error::Unsupported("split hole ownership is unresolved")),
        }
    }
    s.shell.faces[face_index] = Face {
        surface: face.surface.clone(),
        orientation: face.orientation,
        wires: aw,
    };
    let second = s.shell.faces.len();
    s.shell.faces.push(Face {
        surface: face.surface,
        orientation: face.orientation,
        wires: bw,
    });
    s.validate(tol.absolute())?;
    if (s.volume()? - original_volume).abs() > original_volume.abs() * 1e-10 {
        return Err(Error::InvalidTopology(
            "face subdivision changes analytic volume",
        ));
    }
    Ok(PlanarFaceSplit {
        solid: s,
        faces: [face_index, second],
        cut_edge: cut,
        cut_vertices: [va, vb],
    })
}

fn rotate_arc_frame(frame: Frame3, t: f64, tol: Tolerance) -> Result<Frame3> {
    let [u, v, w] = frame.axes();
    Frame3::new(
        frame.origin(),
        [u * t.cos() + v * t.sin(), v * t.cos() - u * t.sin(), w],
        tol,
    )
}

// Rebase angular coordinates while preserving the physical translation vector.
fn rotate_circular_surface(surface: Surface, angle: f64, tol: Tolerance) -> Result<Surface> {
    Ok(match surface {
        Surface::FramedCylinder {
            frame,
            radius,
            height,
        } => Surface::FramedCylinder {
            frame: rotate_arc_frame(frame, angle, tol)?,
            radius,
            height,
        },
        Surface::ExtrudedCircle {
            frame,
            radius,
            height,
            drift,
        } => {
            let rotated = rotate_arc_frame(frame, angle, tol)?;
            let world = frame.vector(Vec3::new(drift[0], drift[1], 0.));
            let local = rotated.local_vector(world);
            Surface::ExtrudedCircle {
                frame: rotated,
                radius,
                height,
                drift: [local.x, local.y],
            }
        }
        _ => {
            return Err(Error::Unsupported(
                "angular rebasing requires a framed circular wall",
            ))
        }
    })
}
/// Two rectangular wall children sharing an original-geometry generator edge.
#[derive(Clone, Debug)]
pub struct CircularFaceSubdivision {
    pub solid: Solid,
    pub faces: [usize; 2],
    pub generator_edge: usize,
    pub rim_vertices: [usize; 2],
}
/// Split a bounded circular wall at its local angle, preserving the closed solid.
/// Supports rectangular framed cylinders and skew circular translation walls
/// with matching bounded arc rims. Both cap wires are refined atomically.
/// Full-periodic rims, near-endpoint cuts and general trim loops are unsupported.
pub fn subdivide_circular_face(
    solid: &Solid,
    face_index: usize,
    angle: f64,
    tol: GeometryTolerance,
) -> Result<CircularFaceSubdivision> {
    solid.validate(tol.absolute())?;
    let wall = solid
        .shell
        .faces
        .get(face_index)
        .ok_or(Error::InvalidInput(
            "circular subdivision face index is out of range",
        ))?;
    let (radius, height, drift) = match wall.surface {
        Surface::FramedCylinder { radius, height, .. } => (radius, height, [0., 0.]),
        Surface::ExtrudedCircle {
            radius,
            height,
            drift,
            ..
        } => (radius, height, drift),
        _ => {
            return Err(Error::Unsupported(
                "circular subdivision requires bounded framed circular walls",
            ))
        }
    };
    let span = wall.cylinder_span()?;
    if !angle.is_finite() || angle <= 0. || angle >= span {
        return Err(Error::InvalidInput(
            "subdivision angle must be finite and strictly inside the face",
        ));
    }
    let budget = tol.length_at_scale(radius.max(height * drift[0].hypot(drift[1]).hypot(1.)))?;
    let guard = 10. * budget * (1. + drift[0].hypot(drift[1]));
    let chord = |u: f64| 2. * radius * (u / 2.).sin().abs();
    if !guard.is_finite() || chord(angle).min(chord(span - angle)) <= guard {
        return Err(Error::Unsupported(
            "circular subdivision is unresolved near an angular endpoint",
        ));
    }
    let volume = solid.volume()?;
    let bottom = wall.wires[0].coedges[0].edge;
    if !matches!(solid.edges[bottom].curve, Curve::Arc { .. })
        || !matches!(
            solid.edges[wall.wires[0].coedges[2].edge].curve,
            Curve::Arc { .. }
        )
    {
        return Err(Error::Unsupported(
            "circular subdivision requires matching bounded arc rims",
        ));
    }
    let mut output = solid.clone();
    let second = output.shell.faces.len();
    let bv = split_boundary_edge(
        &mut output,
        bottom,
        angle,
        solid.edges[bottom].curve.evaluate(angle),
        tol.absolute(),
    )?;
    let generator_edge = output.shell.faces[face_index].wires[0].coedges[1].edge;
    let rim_vertices = output.edges[generator_edge].vertices;
    if rim_vertices[0] != bv {
        return Err(Error::InvalidTopology(
            "circular subdivision lost its generator provenance",
        ));
    }
    output.validate(tol.absolute())?;
    if (output.volume()? - volume).abs() > volume.abs() * 1e-10 {
        return Err(Error::InvalidTopology(
            "circular subdivision changes analytic volume",
        ));
    }
    Ok(CircularFaceSubdivision {
        solid: output,
        faces: [face_index, second],
        generator_edge,
        rim_vertices,
    })
}

// Rim subdivision updates planar cap uses. Cylinder uses are replaced by two
// checked rectangles below, so their unwrapped UV coordinates never get reset
// accidentally while the 3D arc's angular parameter restarts at zero.
fn split_arc_rim(s: &mut Solid, index: usize, t: f64, tol: Tolerance) -> Result<(usize, usize)> {
    let old = s.edges[index].clone();
    let Curve::Arc {
        frame,
        radius,
        sweep,
    } = old.curve
    else {
        return Err(Error::Unsupported(
            "cylinder rim refinement requires bounded circular arcs",
        ));
    };
    let point = old.curve.evaluate(t);
    if t <= 0.0
        || t >= sweep
        || radius * t <= 10.0 * tol.linear
        || radius * (sweep - t) <= 10.0 * tol.linear
        || (point - old.curve.evaluate(0.0)).norm() <= 10.0 * tol.linear
        || (point - old.curve.evaluate(sweep)).norm() <= 10.0 * tol.linear
    {
        return Err(Error::Unsupported(
            "arc subedges are unresolved at model tolerance",
        ));
    }
    let vertex = s.vertices.len();
    s.vertices.push(Vertex { point });
    let new = s.edges.len();
    s.edges[index] = Edge {
        vertices: [old.vertices[0], vertex],
        curve: Curve::Arc {
            frame,
            radius,
            sweep: t,
        },
    };
    s.edges.push(Edge {
        vertices: [vertex, old.vertices[1]],
        curve: Curve::Arc {
            frame: rotate_arc_frame(frame, t, tol)?,
            radius,
            sweep: sweep - t,
        },
    });
    for f in &mut s.shell.faces {
        if !matches!(f.surface, Surface::Plane { .. }) {
            continue;
        }
        for w in &mut f.wires {
            let mut coedges = Vec::new();
            for c in &w.coedges {
                if c.edge != index {
                    coedges.push(c.clone());
                    continue;
                }
                let PCurve::Arc {
                    center,
                    radius,
                    start_angle,
                    sweep,
                } = c.pcurve
                else {
                    return Err(Error::Unsupported(
                        "planar arc split requires angular arc pcurves",
                    ));
                };
                let a = Coedge {
                    edge: index,
                    forward: c.forward,
                    pcurve: PCurve::Arc {
                        center,
                        radius,
                        start_angle,
                        sweep: t,
                    },
                };
                let b = Coedge {
                    edge: new,
                    forward: c.forward,
                    pcurve: PCurve::Arc {
                        center,
                        radius,
                        start_angle: (start_angle + t).rem_euclid(std::f64::consts::TAU),
                        sweep: sweep - t,
                    },
                };
                if c.forward {
                    coedges.extend([a, b]);
                } else {
                    coedges.extend([b, a]);
                }
            }
            w.coedges = coedges;
        }
    }
    Ok((vertex, new))
}
fn split_boundary_edge(
    s: &mut Solid,
    index: usize,
    t: f64,
    point: Point3,
    tol: Tolerance,
) -> Result<usize> {
    if matches!(s.edges[index].curve, Curve::Line { .. }) {
        return split_line_edge(s, index, t, point, tol);
    }
    if !matches!(s.edges[index].curve, Curve::Arc { .. }) {
        return Err(Error::Unsupported(
            "boundary subdivision requires lines or bounded arcs",
        ));
    }
    let walls: Vec<_> = s
        .shell
        .faces
        .iter()
        .enumerate()
        .filter(|(_, f)| {
            !matches!(f.surface, Surface::Plane { .. })
                && f.wires
                    .iter()
                    .flat_map(|w| &w.coedges)
                    .any(|c| c.edge == index)
        })
        .map(|(i, _)| i)
        .collect();
    if walls.len() != 1 {
        return Err(Error::Unsupported(
            "arc boundary requires one rectangular cylindrical neighbor",
        ));
    }
    let fi = walls[0];
    let wall = s.shell.faces[fi].clone();
    let span = wall.cylinder_span()?;
    let height = match wall.surface {
        Surface::FramedCylinder {
            frame: _,
            radius: _,
            height,
        }
        | Surface::ExtrudedCircle {
            radius: _, height, ..
        } => height,
        _ => {
            return Err(Error::Unsupported(
                "bounded rim subdivision requires a framed circular wall",
            ))
        }
    };
    let c = &wall.wires[0].coedges;
    let bottom = c[0].edge;
    let right = c[1].edge;
    let top = c[2].edge;
    let left = c[3].edge;
    if index != bottom && index != top {
        return Err(Error::Unsupported("only cylinder rim arcs can be refined"));
    }
    if !matches!(s.edges[bottom].curve, Curve::Arc { .. })
        || !matches!(s.edges[top].curve, Curve::Arc { .. })
    {
        return Err(Error::Unsupported(
            "both cylinder rims must use bounded arcs",
        ));
    }
    for index in [bottom, top] {
        let Curve::Arc { sweep, .. } = s.edges[index].curve else {
            unreachable!()
        };
        if sweep != span {
            return Err(Error::Unsupported(
                "cylinder refinement requires identical angular domains on both rim edges",
            ));
        }
    }
    let (bv, bnew) = split_arc_rim(s, bottom, t, tol)?;
    let (tv, tnew) = split_arc_rim(s, top, t, tol)?;
    let generator = s.edges.len();
    s.edges.push(Edge {
        vertices: [bv, tv],
        curve: Curve::Line {
            a: s.vertices[bv].point,
            b: s.vertices[tv].point,
        },
    });
    s.shell.faces[fi] = Face {
        surface: wall.surface.clone(),
        orientation: wall.orientation,
        wires: vec![cylinder_rectangle(bottom, top, generator, left, t, height)],
    };
    s.shell.faces.push(Face {
        surface: rotate_circular_surface(wall.surface, t, tol)?,
        orientation: wall.orientation,
        wires: vec![cylinder_rectangle(
            bnew,
            tnew,
            right,
            generator,
            span - t,
            height,
        )],
    });
    Ok(if index == bottom { bv } else { tv })
}

/// A cut graph can create several children and shared material-interval edges.
#[derive(Clone, Debug)]
pub struct PlanarFaceSubdivision {
    pub solid: Solid,
    pub faces: Vec<usize>,
    pub cut_edges: Vec<usize>,
}
/// Subdivide all transverse material intervals, including polygon/arc holes.
/// Repeated crossings on bounded arcs are refined in original-parameter order.
/// Full periodic circle rims are refined with seams away from cut events.
/// Contacts and nonrectangular cylindrical neighbors are unsupported.
pub fn subdivide_planar_face(
    solid: &Solid,
    face_index: usize,
    anchor: Point3,
    direction: Vec3,
    tol: GeometryTolerance,
) -> Result<PlanarFaceSubdivision> {
    let clip = clip_line_to_planar_face(solid, face_index, anchor, direction, tol)?;
    if clip.intervals.is_empty() {
        return Err(Error::Unsupported(
            "subdivision requires material intervals",
        ));
    }
    let periodic: std::collections::BTreeSet<_> = clip
        .events
        .iter()
        .filter(|e| {
            matches!(
                solid.edges[e.edge].curve,
                Curve::Circle { .. } | Curve::FramedCircle { .. }
            )
        })
        .map(|e| e.edge)
        .collect();
    if !periodic.is_empty() {
        let mut normalized = solid.clone();
        for edge in periodic {
            let hits: Vec<_> = clip
                .events
                .iter()
                .filter(|e| e.edge == edge)
                .map(|e| e.edge_parameter)
                .collect();
            let radius = match solid.edges[edge].curve {
                Curve::Circle { radius, .. } | Curve::FramedCircle { radius, .. } => radius,
                _ => unreachable!(),
            };
            let theta = periodic_seam(&hits, radius, tol.absolute())?;
            refine_periodic_wall(&mut normalized, edge, theta, tol.absolute())?;
        }
        normalized.validate(tol.absolute())?;
        let result = subdivide_planar_face(&normalized, face_index, anchor, direction, tol)?;
        if (result.solid.volume()? - solid.volume()?).abs() > solid.volume()?.abs() * 1e-10 {
            return Err(Error::InvalidTopology("periodic refinement changes volume"));
        }
        return Ok(result);
    }
    for e in &clip.events {
        if !matches!(
            solid.edges[e.edge].curve,
            Curve::Line { .. } | Curve::Arc { .. }
        ) {
            return Err(Error::Unsupported(
                "cut graph requires bounded boundary edges",
            ));
        }
    }
    let volume = solid.volume()?;
    let mut s = solid.clone();
    let mut vertices = std::collections::BTreeMap::new();
    // Descending original parameters keep every unprocessed hit on the first
    // subedge at the original index. Arc parameters stay angular; a straight
    // subedge restarts its normalized [0,1] range, so scale by its original end.
    let mut events: Vec<_> = clip.events.iter().collect();
    events.sort_by(|a, b| {
        a.edge
            .cmp(&b.edge)
            .then_with(|| b.edge_parameter.total_cmp(&a.edge_parameter))
    });
    let mut last_edge = None;
    let mut upper = 1.0;
    for e in events {
        if last_edge != Some(e.edge) {
            upper = 1.0;
            last_edge = Some(e.edge);
        }
        let parameter = if matches!(s.edges[e.edge].curve, Curve::Line { .. }) {
            e.edge_parameter / upper
        } else {
            e.edge_parameter
        };
        let v = split_boundary_edge(&mut s, e.edge, parameter, e.point, tol.absolute())?;
        vertices.insert((e.edge, e.edge_parameter.to_bits()), v);
        upper = e.edge_parameter;
    }
    let face = s.shell.faces[face_index].clone();
    let origin = face.surface.parameters(anchor);
    let unit = direction.normalized()?;
    let Surface::Plane { u, v, .. } = face.surface else {
        unreachable!()
    };
    let d = [unit.dot(u), unit.dot(v)];
    let mut halves: [Vec<Coedge>; 2] = [Vec::new(), Vec::new()];
    for w in &face.wires {
        for c in &w.coedges {
            let range = s.edges[c.edge].curve.range();
            let p = c.pcurve.evaluate((range[0] + range[1]) * 0.5);
            let side = d[0] * (p[1] - origin[1]) - d[1] * (p[0] - origin[0]);
            if side.abs() <= tol.linear() {
                return Err(Error::Unsupported(
                    "boundary side is unresolved at cut tolerance",
                ));
            }
            halves[usize::from(side < 0.0)].push(c.clone());
        }
    }
    let mut cut_edges = Vec::new();
    for interval in &clip.intervals {
        let a = vertices[&(interval.start.edge, interval.start.edge_parameter.to_bits())];
        let b = vertices[&(interval.end.edge, interval.end.edge_parameter.to_bits())];
        let pa = s.vertices[a].point;
        let pb = s.vertices[b].point;
        if (pb - pa).norm() <= 10.0 * tol.absolute().linear {
            return Err(Error::Unsupported("cut interval is too small"));
        }
        let edge = s.edges.len();
        s.edges.push(Edge {
            vertices: [a, b],
            curve: Curve::Line { a: pa, b: pb },
        });
        cut_edges.push(edge);
        let uv = face.surface.parameters(pa);
        let end = face.surface.parameters(pb);
        for (i, half) in halves.iter_mut().enumerate() {
            half.push(Coedge {
                edge,
                forward: i == 0,
                pcurve: PCurve::Affine {
                    origin: uv,
                    direction: [end[0] - uv[0], end[1] - uv[1]],
                },
            });
        }
    }
    let mut children = Vec::new();
    for half in halves {
        let mut outgoing = std::collections::BTreeMap::new();
        for c in half {
            if outgoing.insert(start_vertex(&s, &c), c).is_some() {
                return Err(Error::Unsupported("cut graph has ambiguous outgoing edges"));
            }
        }
        let mut loops = Vec::new();
        while let Some((&first, _)) = outgoing.first_key_value() {
            let mut vertex = first;
            let mut coedges = Vec::new();
            loop {
                let c = outgoing
                    .remove(&vertex)
                    .ok_or(Error::InvalidTopology("cut graph is open"))?;
                vertex = s.edges[c.edge].vertices[usize::from(c.forward)];
                coedges.push(c);
                if vertex == first {
                    break;
                }
            }
            loops.push(Wire { coedges });
        }
        let (outer, holes): (Vec<_>, Vec<_>) = loops
            .into_iter()
            .partition(|w| crate::topology::wire_area(w) > 0.0);
        let mut faces: Vec<_> = outer
            .into_iter()
            .map(|w| Face {
                surface: face.surface.clone(),
                orientation: face.orientation,
                wires: vec![w],
            })
            .collect();
        for hole in holes {
            let c = &hole.coedges[0];
            let range = s.edges[c.edge].curve.range();
            let p = c.pcurve.evaluate(range[usize::from(!c.forward)]);
            let mut owner = None;
            for (i, f) in faces.iter().enumerate() {
                let ring = crate::face_intersections::rings(f).remove(0);
                match crate::mixed::point_location(p, &ring, tol.absolute())? {
                    PointLocation::Inside => {
                        if owner.replace(i).is_some() {
                            return Err(Error::Unsupported("hole has multiple owners"));
                        }
                    }
                    PointLocation::Outside => (),
                    PointLocation::Boundary => {
                        return Err(Error::Unsupported("hole ownership touches boundary"))
                    }
                }
            }
            faces[owner.ok_or(Error::InvalidTopology("cut hole has no owner"))?]
                .wires
                .push(hole);
        }
        children.extend(faces);
    }
    if children.len() < 2 {
        return Err(Error::Unsupported("cut does not subdivide the face"));
    }
    let mut faces = vec![face_index];
    s.shell.faces[face_index] = children.remove(0);
    for child in children {
        faces.push(s.shell.faces.len());
        s.shell.faces.push(child);
    }
    s.validate(tol.absolute())?;
    if (s.volume()? - volume).abs() > volume.abs() * 1e-10 {
        return Err(Error::InvalidTopology("cut graph changes volume"));
    }
    Ok(PlanarFaceSubdivision {
        solid: s,
        faces,
        cut_edges,
    })
}

fn cylinder_rectangle(
    bottom: usize,
    top: usize,
    right: usize,
    left: usize,
    span: f64,
    height: f64,
) -> Wire {
    Wire {
        coedges: vec![
            Coedge {
                edge: bottom,
                forward: true,
                pcurve: PCurve::Affine {
                    origin: [0.0, 0.0],
                    direction: [1.0, 0.0],
                },
            },
            Coedge {
                edge: right,
                forward: true,
                pcurve: PCurve::Affine {
                    origin: [span, 0.0],
                    direction: [0.0, height],
                },
            },
            Coedge {
                edge: top,
                forward: false,
                pcurve: PCurve::Affine {
                    origin: [0.0, height],
                    direction: [1.0, 0.0],
                },
            },
            Coedge {
                edge: left,
                forward: false,
                pcurve: PCurve::Affine {
                    origin: [0.0, 0.0],
                    direction: [0.0, height],
                },
            },
        ],
    }
}

// Choose a canonical quarter partition whose vertices avoid all proper hits.
// Original periodic seams are artificial, so relocation preserves the surface.
fn periodic_seam(hits: &[f64], radius: f64, tol: Tolerance) -> Result<f64> {
    use std::f64::consts::{FRAC_PI_2, TAU};
    let mut best = (0.0, 0.0);
    for i in 0..64 {
        let theta = (i as f64 + 0.5) * FRAC_PI_2 / 64.0;
        let clearance = hits
            .iter()
            .flat_map(|t| {
                (0..4).map(move |j| {
                    let gap = (t - theta - j as f64 * FRAC_PI_2).rem_euclid(TAU);
                    gap.min(TAU - gap)
                })
            })
            .fold(f64::INFINITY, f64::min);
        // Keep the first candidate on floating-point ties for native/WASM parity.
        if clearance > best.1 + 128.0 * f64::EPSILON {
            best = (theta, clearance);
        }
    }
    if 2.0 * radius * (best.1 * 0.5).sin() <= 10.0 * tol.linear {
        return Err(Error::Unsupported("no resolved periodic seam partition"));
    }
    Ok(best.0)
}

fn refine_periodic_wall(s: &mut Solid, edge: usize, theta: f64, tol: Tolerance) -> Result<()> {
    use std::f64::consts::{FRAC_PI_2, TAU};
    let walls: Vec<_> = s
        .shell
        .faces
        .iter()
        .enumerate()
        .filter(|(_, f)| {
            !matches!(f.surface, Surface::Plane { .. })
                && f.wires
                    .iter()
                    .flat_map(|w| &w.coedges)
                    .any(|c| c.edge == edge)
        })
        .map(|(i, _)| i)
        .collect();
    if walls.len() != 1 {
        return Err(Error::Unsupported(
            "periodic rim requires one cylindrical neighbor",
        ));
    }
    let fi = walls[0];
    let wall = s.shell.faces[fi].clone();
    if wall.cylinder_span()? != TAU {
        return Err(Error::Unsupported(
            "periodic refinement requires a full cylinder rectangle",
        ));
    }
    let (frame, radius, height) = match wall.surface {
        Surface::Cylinder {
            center,
            radius,
            height,
        } => (Frame3::translation(center)?, radius, height),
        Surface::FramedCylinder {
            frame,
            radius,
            height,
        } => (frame, radius, height),
        _ => return Err(Error::Unsupported("periodic rim requires a cylinder")),
    };
    let c = &wall.wires[0].coedges;
    let bottom = c[0].edge;
    let top = c[2].edge;
    let seam = c[1].edge;
    if (edge != bottom && edge != top) || c[3].edge != seam {
        return Err(Error::Unsupported(
            "periodic cylinder must share its seam edge",
        ));
    }
    let mut rims = Vec::new();
    let mut rim_vertices = Vec::new();
    for index in [bottom, top] {
        let old = s.edges[index].clone();
        let (rim_frame, r) = match old.curve {
            Curve::Circle { center, radius } => (Frame3::translation(center)?, radius),
            Curve::FramedCircle { frame, radius } => (frame, radius),
            _ => {
                return Err(Error::Unsupported(
                    "both periodic rims require full circles",
                ))
            }
        };
        if r != radius || old.vertices[0] != old.vertices[1] {
            return Err(Error::Unsupported(
                "periodic rim radius or seam is inconsistent",
            ));
        }
        let mut vertices = vec![old.vertices[0]];
        for j in 1..4 {
            let vertex = s.vertices.len();
            vertices.push(vertex);
            s.vertices.push(Vertex {
                point: old.curve.evaluate(theta + j as f64 * FRAC_PI_2),
            });
        }
        s.vertices[vertices[0]].point = old.curve.evaluate(theta);
        let mut edges = vec![index];
        for j in 0..4 {
            let arc = Edge {
                vertices: [vertices[j], vertices[(j + 1) % 4]],
                curve: Curve::Arc {
                    frame: rotate_arc_frame(rim_frame, theta + j as f64 * FRAC_PI_2, tol)?,
                    radius,
                    sweep: FRAC_PI_2,
                },
            };
            if j == 0 {
                s.edges[index] = arc;
            } else {
                edges.push(s.edges.len());
                s.edges.push(arc);
            }
        }
        for f in &mut s.shell.faces {
            if !matches!(f.surface, Surface::Plane { .. }) {
                continue;
            }
            for w in &mut f.wires {
                let mut uses = Vec::new();
                for c in &w.coedges {
                    if c.edge != index {
                        uses.push(c.clone());
                        continue;
                    }
                    let PCurve::Circle { center, radius } = c.pcurve else {
                        return Err(Error::Unsupported(
                            "periodic planar rim requires a full-circle pcurve",
                        ));
                    };
                    let mut arcs: Vec<_> = edges
                        .iter()
                        .enumerate()
                        .map(|(j, &edge)| Coedge {
                            edge,
                            forward: c.forward,
                            pcurve: PCurve::Arc {
                                center,
                                radius,
                                start_angle: (theta + j as f64 * FRAC_PI_2).rem_euclid(TAU),
                                sweep: FRAC_PI_2,
                            },
                        })
                        .collect();
                    if !c.forward {
                        arcs.reverse();
                    }
                    uses.extend(arcs);
                }
                w.coedges = uses;
            }
        }
        rims.push(edges);
        rim_vertices.push(vertices);
    }
    let mut generators = vec![seam];
    for (j, (&a, &b)) in rim_vertices[0].iter().zip(&rim_vertices[1]).enumerate() {
        let line = Edge {
            vertices: [a, b],
            curve: Curve::Line {
                a: s.vertices[a].point,
                b: s.vertices[b].point,
            },
        };
        if j == 0 {
            s.edges[seam] = line;
        } else {
            generators.push(s.edges.len());
            s.edges.push(line);
        }
    }
    for j in 0..4 {
        let face = Face {
            surface: Surface::FramedCylinder {
                frame: rotate_arc_frame(frame, theta + j as f64 * FRAC_PI_2, tol)?,
                radius,
                height,
            },
            orientation: wall.orientation,
            wires: vec![cylinder_rectangle(
                rims[0][j],
                rims[1][j],
                generators[(j + 1) % 4],
                generators[j],
                FRAC_PI_2,
                height,
            )],
        };
        if j == 0 {
            s.shell.faces[fi] = face;
        } else {
            s.shell.faces.push(face);
        }
    }
    Ok(())
}

/// Demo: split a skew rounded plate cap, then split a remaining circular wall.
pub fn skew_face_subdivision_demo(fraction: f64, placement: f64) -> Result<Solid> {
    if !fraction.is_finite() || fraction <= 0. || fraction >= 1. || !placement.is_finite() {
        return Err(Error::InvalidInput(
            "expected an interior angular fraction and finite placement",
        ));
    }
    let tol = GeometryTolerance::default();
    let solid = crate::classification::curved_classification_solid(5)?;
    let split = split_planar_face(
        &solid,
        0,
        Point3::new(0., 22., -12.),
        Vec3::new(1., 0., 0.),
        tol,
    )?
    .solid;
    let fi = split
        .shell
        .faces
        .iter()
        .position(|f| matches!(f.surface, Surface::ExtrudedCircle { .. }))
        .ok_or(Error::InvalidTopology(
            "skew subdivision fixture has no circular wall",
        ))?;
    let angle = split.shell.faces[fi].cylinder_span()? * fraction;
    let result = subdivide_circular_face(&split, fi, angle, tol)?.solid;
    result.transformed(
        Transform::rotation(Vec3::new(1., 2., 3.), placement)?,
        tol.absolute(),
    )
}
/// Exact closed B-rep-derived display mesh, shared by native and WASM demos.
pub fn skew_face_subdivision_demo_json(fraction: f64, placement: f64) -> Result<String> {
    skew_face_subdivision_demo(fraction, placement)?.mesh_json(0.05, Tolerance::default())
}
