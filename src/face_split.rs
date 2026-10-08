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
    let (frame, radius, height) = match wall.surface {
        Surface::FramedCylinder {
            frame,
            radius,
            height,
        } => (frame, radius, height),
        _ => {
            return Err(Error::Unsupported(
                "bounded rim subdivision requires a framed cylinder",
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
    let rectangle = |bottom, top, right, left, span| Wire {
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
    };
    s.shell.faces[fi] = Face {
        surface: Surface::FramedCylinder {
            frame,
            radius,
            height,
        },
        orientation: wall.orientation,
        wires: vec![rectangle(bottom, top, generator, left, t)],
    };
    s.shell.faces.push(Face {
        surface: Surface::FramedCylinder {
            frame: rotate_arc_frame(frame, t, tol)?,
            radius,
            height,
        },
        orientation: wall.orientation,
        wires: vec![rectangle(bnew, tnew, right, generator, span - t)],
    });
    Ok(if index == bottom { bv } else { tv })
}
