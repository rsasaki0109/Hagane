//! Exact removal of redundant degree-two straight-boundary vertices.
use crate::*;
fn collinear(a: Point3, v: Point3, b: Point3) -> Result<bool> {
    Ok(
        orient2d([a.x, a.y], [v.x, v.y], [b.x, b.y])? == Orientation::Collinear
            && orient2d([a.x, a.z], [v.x, v.z], [b.x, b.z])? == Orientation::Collinear
            && orient2d([a.y, a.z], [v.y, v.z], [b.y, b.z])? == Orientation::Collinear,
    )
}
fn between(a: Point3, v: Point3, b: Point3) -> bool {
    let d = b - a;
    let axis = if d.x.abs() >= d.y.abs() && d.x.abs() >= d.z.abs() {
        0
    } else if d.y.abs() >= d.z.abs() {
        1
    } else {
        2
    };
    let a = [a.x, a.y, a.z][axis];
    let v = [v.x, v.y, v.z][axis];
    let b = [b.x, b.y, b.z][axis];
    v > a.min(b) && v < a.max(b)
}
fn uv_start(c: &Coedge) -> [f64; 2] {
    c.pcurve.evaluate(if c.forward { 0. } else { 1. })
}
fn uv_end(c: &Coedge) -> [f64; 2] {
    c.pcurve.evaluate(if c.forward { 1. } else { 0. })
}
/// Remove degree-two vertices that are exactly collinear in 3D and both incident
/// face UV boundaries. Planar straight-edge inputs only. Corners, branches and
/// nearly collinear knots remain; no snapping or healing is performed.
pub fn simplify_straight_edges(solid: &Solid, tol: GeometryTolerance) -> Result<Solid> {
    planar_face_patches(solid, tol.absolute())?;
    if solid.shell.faces.len() > 512
        || solid
            .shell
            .faces
            .iter()
            .flat_map(|f| &f.wires)
            .map(|w| w.coedges.len())
            .sum::<usize>()
            > 4096
    {
        return Err(Error::Unsupported(
            "straight edge simplification requires at most 512 faces and 4096 coedges",
        ));
    }
    let mut incident = vec![Vec::new(); solid.vertices.len()];
    let mut faces = vec![Vec::new(); solid.edges.len()];
    for (i, e) in solid.edges.iter().enumerate() {
        for &v in &e.vertices {
            incident[v].push(i);
        }
    }
    for (i, f) in solid.shell.faces.iter().enumerate() {
        for c in f.wires.iter().flat_map(|w| &w.coedges) {
            faces[c.edge].push(i);
        }
    }
    let mut remove = vec![false; solid.vertices.len()];
    for (v, edges) in incident.iter().enumerate() {
        if edges.len() != 2 || faces[edges[0]] != faces[edges[1]] {
            continue;
        }
        let other = |e: usize| solid.edges[e].vertices.iter().copied().find(|&i| i != v);
        let (Some(a), Some(b)) = (other(edges[0]), other(edges[1])) else {
            continue;
        };
        let p = solid.vertices[v].point;
        if a == b
            || !collinear(solid.vertices[a].point, p, solid.vertices[b].point)?
            || !between(solid.vertices[a].point, p, solid.vertices[b].point)
        {
            continue;
        }
        let mut uses = 0;
        let mut exact = true;
        for &f in &faces[edges[0]] {
            for w in &solid.shell.faces[f].wires {
                for (i, c) in w.coedges.iter().enumerate() {
                    if solid.edges[c.edge].vertices[usize::from(!c.forward)] != v {
                        continue;
                    }
                    let previous = &w.coedges[(i + w.coedges.len() - 1) % w.coedges.len()];
                    uses += 1;
                    let a = uv_start(previous);
                    let p = uv_start(c);
                    let b = uv_end(c);
                    exact &= uv_end(previous) == p
                        && p != a
                        && p != b
                        && (0..2).all(|axis| {
                            p[axis] >= a[axis].min(b[axis]) && p[axis] <= a[axis].max(b[axis])
                        })
                        && orient2d(a, p, b)? == Orientation::Collinear;
                }
            }
        }
        remove[v] = exact && uses == 2;
    }
    if !remove.iter().any(|&v| v) {
        return Ok(solid.clone());
    }
    let mut result = Solid {
        vertices: vec![],
        edges: vec![],
        shell: Shell { faces: vec![] },
    };
    let mut vertex_ids = vec![usize::MAX; solid.vertices.len()];
    for (i, v) in solid.vertices.iter().enumerate() {
        if !remove[i] {
            vertex_ids[i] = result.vertices.len();
            result.vertices.push(v.clone());
        }
    }
    let mut edge_ids = std::collections::BTreeMap::new();
    for f in &solid.shell.faces {
        let mut wires = Vec::new();
        for w in &f.wires {
            let starts: Vec<_> = w
                .coedges
                .iter()
                .map(|c| solid.edges[c.edge].vertices[usize::from(!c.forward)])
                .collect();
            let retained: Vec<_> = (0..starts.len()).filter(|&i| !remove[starts[i]]).collect();
            if retained.len() < 3 {
                return Err(Error::InvalidTopology("simplification collapses a wire"));
            }
            let mut coedges = Vec::new();
            for (i, &start) in retained.iter().enumerate() {
                let end = retained[(i + 1) % retained.len()];
                let last = (end + starts.len() - 1) % starts.len();
                let a = vertex_ids[starts[start]];
                let b = vertex_ids[starts[end]];
                let key = (a.min(b), a.max(b));
                let single = (start + 1) % starts.len() == end;
                let id = *edge_ids.entry(key).or_insert_with(|| {
                    let id = result.edges.len();
                    let edge = if single {
                        let e = &solid.edges[w.coedges[start].edge];
                        Edge {
                            vertices: e.vertices.map(|v| vertex_ids[v]),
                            curve: e.curve.clone(),
                        }
                    } else {
                        Edge {
                            vertices: [a, b],
                            curve: Curve::Line {
                                a: result.vertices[a].point,
                                b: result.vertices[b].point,
                            },
                        }
                    };
                    result.edges.push(edge);
                    id
                });
                let forward = result.edges[id].vertices[0] == a;
                let pcurve = if single && forward == w.coedges[start].forward {
                    w.coedges[start].pcurve.clone()
                } else {
                    let p = uv_start(&w.coedges[start]);
                    let q = uv_end(&w.coedges[last]);
                    let (p, q) = if forward { (p, q) } else { (q, p) };
                    PCurve::Affine {
                        origin: p,
                        direction: [q[0] - p[0], q[1] - p[1]],
                    }
                };
                coedges.push(Coedge {
                    edge: id,
                    forward,
                    pcurve,
                });
            }
            wires.push(Wire { coedges });
        }
        result.shell.faces.push(Face {
            surface: f.surface.clone(),
            orientation: f.orientation,
            wires,
        });
    }
    result.validate(tol.absolute())?;
    if result.bounds() != solid.bounds()
        || (result.volume()? - solid.volume()?).abs() > solid.volume()?.abs() * 1e-10
        || result.edges.len() > solid.edges.len()
    {
        return Err(Error::InvalidTopology(
            "edge simplification changes bounds or volume or adds edges",
        ));
    }
    Ok(result)
}
pub(crate) fn simplified_contact_demo(offset: f64) -> Result<Solid> {
    simplify_straight_edges(
        &crate::face_merge::merged_contact_demo(offset)?,
        GeometryTolerance::default(),
    )
}
pub fn simplified_contact_demo_json(offset: f64) -> Result<String> {
    simplified_contact_demo(offset)?.mesh_json(0.05, Tolerance::default())
}
