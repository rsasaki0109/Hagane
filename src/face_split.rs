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
fn split_edge(s: &mut Solid, index: usize, t: f64, point: Point3, tol: Tolerance) -> Result<usize> {
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
                        || !matches!(c.pcurve, PCurve::Affine { .. })
                        || f.wires
                            .iter()
                            .flat_map(|w| &w.coedges)
                            .any(|c| matches!(c.pcurve, PCurve::Arc { .. })))
                {
                    return Err(Error::Unsupported(
                        "split edge neighbors require planar affine/circular trims",
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
/// Splits a planar polygon face along one proper interior interval. Exactly two
/// crossings on distinct outer straight edges are required. Holes may be full
/// circles or polygons but cannot be crossed. Neighboring edge uses are split
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
    if face
        .wires
        .iter()
        .flat_map(|w| &w.coedges)
        .any(|c| matches!(c.pcurve, PCurve::Arc { .. }))
        || face.wires[0]
            .coedges
            .iter()
            .any(|c| !matches!(solid.edges[c.edge].curve, Curve::Line { .. }))
    {
        return Err(Error::Unsupported(
            "face splitting supports polygon outer trims and polygon/full-circle holes",
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
    let va = split_edge(
        &mut s,
        clip.events[0].edge,
        clip.events[0].edge_parameter,
        clip.events[0].point,
        tol.absolute(),
    )?;
    let vb = split_edge(
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
    let polygon = |coedges: &[Coedge]| {
        coedges
            .iter()
            .map(|c| c.pcurve.evaluate(if c.forward { 0.0 } else { 1.0 }))
            .collect::<Vec<_>>()
    };
    let ap = polygon(&a);
    let bp = polygon(&b);
    let mut aw = vec![Wire { coedges: a }];
    let mut bw = vec![Wire { coedges: b }];
    for hole in &face.wires[1..] {
        let c = &hole.coedges[0];
        let range = s.edges[c.edge].curve.range();
        let witness = c.pcurve.evaluate(range[usize::from(!c.forward)]);
        match (
            locate_point_in_polygon(witness, &ap)?,
            locate_point_in_polygon(witness, &bp)?,
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
