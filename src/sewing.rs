//! Exact planar face sewing with shared straight boundaries.
use crate::*;
/// Independent face boundary points. Rings omit the repeated closing point.
/// Outer rings are CCW and holes CW in surface UV, independent of orientation.
#[derive(Clone, Debug)]
pub struct PlanarFacePatch {
    pub surface: Surface,
    pub orientation: i8,
    pub rings: Vec<Vec<Point3>>,
}
/// Extract independent patches from a validated all-planar, straight-edge solid.
pub fn planar_face_patches(solid: &Solid, tol: Tolerance) -> Result<Vec<PlanarFacePatch>> {
    solid.validate(tol)?;
    solid
        .shell
        .faces
        .iter()
        .map(|f| {
            if !matches!(f.surface, Surface::Plane { .. }) {
                return Err(Error::Unsupported("patch extraction requires planar faces"));
            }
            let rings = f
                .wires
                .iter()
                .map(|w| {
                    w.coedges
                        .iter()
                        .map(|c| {
                            let e = &solid.edges[c.edge];
                            if !matches!(e.curve, Curve::Line { .. }) {
                                return Err(Error::Unsupported(
                                    "patch extraction requires straight edges",
                                ));
                            }
                            Ok(solid.vertices[e.vertices[usize::from(!c.forward)]].point)
                        })
                        .collect()
                })
                .collect::<Result<_>>()?;
            Ok(PlanarFacePatch {
                surface: f.surface.clone(),
                orientation: f.orientation,
                rings,
            })
        })
        .collect()
}
fn exact_collinear(a: Point3, b: Point3, p: Point3) -> Result<bool> {
    Ok(
        orient2d([a.x, a.y], [b.x, b.y], [p.x, p.y])? == Orientation::Collinear
            && orient2d([a.x, a.z], [b.x, b.z], [p.x, p.z])? == Orientation::Collinear
            && orient2d([a.y, a.z], [b.y, b.z], [p.y, p.z])? == Orientation::Collinear,
    )
}
/// Sew coincident independent planar boundaries into one closed manifold solid.
/// Exact coincident vertices merge; exact collinear subdivision vertices split
/// all incident edges. Near-but-distinct vertices/T-junctions are rejected,
/// rather than snapped. Open, inconsistent or nonmanifold shells fail validation.
/// Input patch interiors must not intersect; arbitrary surface sewing/healing
/// and geometric self-intersection detection are outside this API's domain.
pub fn sew_planar_faces(patches: &[PlanarFacePatch], tol: GeometryTolerance) -> Result<Solid> {
    if patches.is_empty()
        || patches.len() > 512
        || patches
            .iter()
            .flat_map(|p| &p.rings)
            .map(Vec::len)
            .sum::<usize>()
            > 4096
    {
        return Err(Error::Unsupported(
            "sewing requires 1..512 patches and at most 4096 corners",
        ));
    }
    let mut s = Solid {
        vertices: vec![],
        edges: vec![],
        shell: Shell { faces: vec![] },
    };
    let mut rings = Vec::new();
    for patch in patches {
        let Surface::Plane { origin, u, v } = patch.surface else {
            return Err(Error::Unsupported("sewing requires planar patches"));
        };
        Frame3::new(origin, [u, v, u.cross(v)], tol.absolute())?;
        if patch.rings.is_empty() || ![-1, 1].contains(&patch.orientation) {
            return Err(Error::InvalidInput(
                "patch requires rings and signed orientation",
            ));
        }
        let mut pr = Vec::new();
        for ring in &patch.rings {
            if ring.len() < 3 {
                return Err(Error::InvalidInput(
                    "patch ring needs at least three corners",
                ));
            }
            let mut ids = Vec::new();
            for &p in ring {
                if !p.finite() {
                    return Err(Error::InvalidInput("nonfinite patch vertex"));
                }
                let uv = patch.surface.parameters(p);
                if !tol
                    .absolute()
                    .coincident(p, patch.surface.evaluate(uv[0], uv[1]))
                {
                    return Err(Error::InvalidInput("patch vertex is off its plane"));
                }
                let mut id = None;
                for (i, vertex) in s.vertices.iter().enumerate() {
                    if vertex.point == p {
                        id = Some(i);
                        break;
                    }
                    if tol.absolute().coincident(vertex.point, p) {
                        return Err(Error::Unsupported(
                            "near-coincident sewing vertices require explicit healing",
                        ));
                    }
                }
                let i = id.unwrap_or_else(|| {
                    let i = s.vertices.len();
                    s.vertices.push(Vertex { point: p });
                    i
                });
                ids.push(i);
            }
            pr.push(ids);
        }
        rings.push(pr);
    }
    let mut edges = std::collections::BTreeMap::new();
    for (patch, pr) in patches.iter().zip(rings) {
        let mut wires = Vec::new();
        for ring in pr {
            let mut coedges = Vec::new();
            for i in 0..ring.len() {
                let a = ring[i];
                let b = ring[(i + 1) % ring.len()];
                let pa = s.vertices[a].point;
                let pb = s.vertices[b].point;
                let d = pb - pa;
                let length = d.norm();
                if !length.is_finite() || length <= 10.0 * tol.linear() {
                    return Err(Error::InvalidInput(
                        "sewing edges must exceed ten linear tolerances",
                    ));
                }
                let axis = if d.x.abs() >= d.y.abs() && d.x.abs() >= d.z.abs() {
                    0
                } else if d.y.abs() >= d.z.abs() {
                    1
                } else {
                    2
                };
                let component = |p: Vec3| [p.x, p.y, p.z][axis];
                let mut points = vec![(0.0, a), (1.0, b)];
                for (j, vertex) in s.vertices.iter().enumerate() {
                    if j == a || j == b {
                        continue;
                    }
                    let t = component(vertex.point - pa) / component(d);
                    if t <= 0.0 || t >= 1.0 {
                        continue;
                    }
                    if exact_collinear(pa, pb, vertex.point)? {
                        points.push((t, j));
                    } else if (vertex.point - (pa + d * t)).norm() <= tol.linear() {
                        return Err(Error::Unsupported(
                            "near-collinear sewing junction requires explicit healing",
                        ));
                    }
                }
                points.sort_by(|a, b| a.0.total_cmp(&b.0));
                for pair in points.windows(2) {
                    let a = pair[0].1;
                    let b = pair[1].1;
                    if (s.vertices[a].point - s.vertices[b].point).norm() <= 10.0 * tol.linear() {
                        return Err(Error::Unsupported(
                            "sewn subedges must exceed ten linear tolerances",
                        ));
                    }
                    let (first, last) = if a < b { (a, b) } else { (b, a) };
                    let edge = *edges.entry((first, last)).or_insert_with(|| {
                        let index = s.edges.len();
                        s.edges.push(Edge {
                            vertices: [first, last],
                            curve: Curve::Line {
                                a: s.vertices[first].point,
                                b: s.vertices[last].point,
                            },
                        });
                        index
                    });
                    let uv = patch.surface.parameters(s.vertices[first].point);
                    let end = patch.surface.parameters(s.vertices[last].point);
                    coedges.push(Coedge {
                        edge,
                        forward: a == first,
                        pcurve: PCurve::Affine {
                            origin: uv,
                            direction: [end[0] - uv[0], end[1] - uv[1]],
                        },
                    });
                }
            }
            wires.push(Wire { coedges });
        }
        s.shell.faces.push(Face {
            surface: patch.surface.clone(),
            orientation: patch.orientation,
            wires,
        });
    }
    s.validate(tol.absolute())?;
    Ok(s)
}
