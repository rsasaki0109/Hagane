//! Topological union of adjacent faces with certified identical planar support.
use crate::*;
/// Certify equal supports of two checked plane frames using exact dyadic
/// scalar triple products. Outward orientation is independent of this test.
pub fn same_plane_support(first: &Surface, second: &Surface) -> Result<bool> {
    let (
        Surface::Plane { origin: a, u, v },
        Surface::Plane {
            origin: b,
            u: s,
            v: t,
        },
    ) = (first, second)
    else {
        return Err(Error::Unsupported(
            "plane support identity requires two planes",
        ));
    };
    Frame3::new(*a, [*u, *v, u.cross(*v)], Tolerance::default())?;
    Frame3::new(*b, [*s, *t, s.cross(*t)], Tolerance::default())?;
    let coords = |p: Vec3| [p.x, p.y, p.z];
    Ok(crate::predicates::exact_plane_support(
        coords(*a),
        coords(*u),
        coords(*v),
        coords(*b),
        coords(*s),
        coords(*t),
    ))
}
fn same_support(a: &Face, b: &Face) -> Result<bool> {
    Ok(same_plane_support(&a.surface, &b.surface)?
        && a.surface.normal(0.).dot(b.surface.normal(0.))
            * (a.orientation as f64)
            * (b.orientation as f64)
            > 0.)
}
fn basis_coordinates(u: Vec3, v: Vec3, source: Vec3) -> [f64; 2] {
    if source == u {
        [1., 0.]
    } else if source == u * (-1.) {
        [-1., 0.]
    } else if source == v {
        [0., 1.]
    } else if source == v * (-1.) {
        [0., -1.]
    } else {
        [u.dot(source), v.dot(source)]
    }
}
/// Merge edge-connected planar straight-edge faces on identical supports.
/// Exact scalar triple products certify plane identity across different origins
/// and UV frames. Distinct supports and opposite orientations remain separate.
/// No tolerance-based plane snapping or collinear edge simplification is done.
pub fn merge_coplanar_faces(solid: &Solid, tol: GeometryTolerance) -> Result<Solid> {
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
            "face merging requires at most 512 faces and 4096 coedges",
        ));
    }
    let faces = &solid.shell.faces;
    let mut uses = vec![Vec::new(); solid.edges.len()];
    for (i, f) in faces.iter().enumerate() {
        for c in f.wires.iter().flat_map(|w| &w.coedges) {
            uses[c.edge].push(i);
        }
    }
    let mut adjacent = vec![Vec::new(); faces.len()];
    for edge in &uses {
        if edge.len() != 2 {
            return Err(Error::InvalidTopology(
                "face merging needs two incident faces per edge",
            ));
        }
        let (a, b) = (edge[0], edge[1]);
        if a != b && same_support(&faces[a], &faces[b])? {
            adjacent[a].push(b);
            adjacent[b].push(a);
        }
    }
    let mut owners = vec![usize::MAX; faces.len()];
    let mut groups = Vec::new();
    for start in 0..faces.len() {
        if owners[start] != usize::MAX {
            continue;
        }
        let id = groups.len();
        owners[start] = id;
        let mut group = vec![start];
        let mut cursor = 0;
        while cursor < group.len() {
            let f = group[cursor];
            cursor += 1;
            for &neighbor in &adjacent[f] {
                if owners[neighbor] == usize::MAX {
                    owners[neighbor] = id;
                    group.push(neighbor);
                }
            }
        }
        groups.push(group);
    }
    if groups.len() == faces.len() {
        return Ok(solid.clone());
    }
    let points: Vec<_> = solid.vertices.iter().map(|v| v.point).collect();
    let mut patches = Vec::new();
    for (id, group) in groups.iter().enumerate() {
        let representative = &faces[group[0]];
        let mut boundary = Vec::new();
        for &f in group {
            for c in faces[f].wires.iter().flat_map(|w| &w.coedges) {
                if uses[c.edge].iter().all(|&face| owners[face] == id) {
                    continue;
                }
                let [a, b] = solid.edges[c.edge].vertices;
                let direction = c.forward == (faces[f].orientation == representative.orientation);
                boundary.push(if direction { (a, b) } else { (b, a) });
            }
        }
        let rings = crate::solid_split::cycles(boundary)?;
        patches.extend(crate::solid_split::patches(
            &representative.surface,
            representative.orientation,
            rings,
            &points,
        )?);
    }
    let mut result = sew_planar_faces(&patches, tol)?;
    // Rigid placement preserves exact original UV trims. Reprojecting world
    // vertices can introduce spurious near-collinear UV knots; retain the
    // original affine pcurves, converting their local coordinates when needed.
    let vertex_ids: Vec<_> = result
        .vertices
        .iter()
        .map(|v| {
            solid
                .vertices
                .iter()
                .position(|old| old.point == v.point)
                .ok_or(Error::InvalidTopology(
                    "merged boundary introduces a vertex",
                ))
        })
        .collect::<Result<_>>()?;
    let original_edges: std::collections::BTreeMap<_, _> = solid
        .edges
        .iter()
        .enumerate()
        .map(|(i, e)| {
            (
                (
                    e.vertices[0].min(e.vertices[1]),
                    e.vertices[0].max(e.vertices[1]),
                ),
                i,
            )
        })
        .collect();
    for face in &mut result.shell.faces {
        let Surface::Plane { origin, u, v } = face.surface else {
            unreachable!()
        };
        let mut original_curves = std::collections::BTreeMap::new();
        for old in faces {
            if same_support(face, old)? {
                let Surface::Plane {
                    origin: o,
                    u: a,
                    v: b,
                } = old.surface
                else {
                    unreachable!()
                };
                let offset = if o == origin {
                    [0., 0.]
                } else {
                    face.surface.parameters(o)
                };
                let a = basis_coordinates(u, v, a);
                let b = basis_coordinates(u, v, b);
                let coefficients = [[a[0], b[0]], [a[1], b[1]]];
                let map = |p: [f64; 2]| coefficients.map(|row| row[0] * p[0] + row[1] * p[1]);
                for c in old.wires.iter().flat_map(|w| &w.coedges) {
                    let PCurve::Affine {
                        origin: p,
                        direction: d,
                    } = c.pcurve
                    else {
                        return Err(Error::Unsupported(
                            "plane frame conversion requires affine pcurves",
                        ));
                    };
                    let q = map(p);
                    original_curves.insert(
                        c.edge,
                        PCurve::Affine {
                            origin: [offset[0] + q[0], offset[1] + q[1]],
                            direction: map(d),
                        },
                    );
                }
            }
        }
        for c in face.wires.iter_mut().flat_map(|w| &mut w.coedges) {
            let e = &result.edges[c.edge];
            let a = vertex_ids[e.vertices[0]];
            let b = vertex_ids[e.vertices[1]];
            let old_edge = *original_edges
                .get(&(a.min(b), a.max(b)))
                .ok_or(Error::InvalidTopology("merged boundary introduces an edge"))?;
            if let Some(PCurve::Affine { origin, direction }) = original_curves.get(&old_edge) {
                c.pcurve = if solid.edges[old_edge].vertices[0] == a {
                    PCurve::Affine {
                        origin: *origin,
                        direction: *direction,
                    }
                } else {
                    PCurve::Affine {
                        origin: [origin[0] + direction[0], origin[1] + direction[1]],
                        direction: [-direction[0], -direction[1]],
                    }
                };
            }
        }
    }
    result.validate(tol.absolute())?;
    if result.shell.faces.len() > faces.len()
        || result.bounds() != solid.bounds()
        || (result.volume()? - solid.volume()?).abs() > solid.volume()?.abs() * 1e-10
    {
        return Err(Error::InvalidTopology(
            "coplanar face merging changes bounds, volume or increases face count",
        ));
    }
    Ok(result)
}
pub(crate) fn merged_contact_demo(offset: f64) -> Result<Solid> {
    merge_coplanar_faces(
        &crate::box_contact_demo(offset)?,
        GeometryTolerance::default(),
    )
}
pub fn merged_contact_demo_json(offset: f64) -> Result<String> {
    merged_contact_demo(offset)?.mesh_json(0.05, Tolerance::default())
}
pub(crate) fn reframed_merge_demo(offset: f64) -> Result<Solid> {
    let t = GeometryTolerance::default();
    let mut part = crate::booleans::convex_union_demo(offset)?.transformed(
        Transform::rotation(Vec3::new(1., 2., 3.), 0.7)?,
        t.absolute(),
    )?;
    for (i, f) in part.shell.faces.iter_mut().enumerate() {
        if i % 2 == 0 {
            continue;
        }
        let Surface::Plane { origin, u, v } = f.surface else {
            unreachable!()
        };
        f.surface = Surface::Plane {
            origin,
            u: v,
            v: u * (-1.),
        };
        for c in f.wires.iter_mut().flat_map(|w| &mut w.coedges) {
            let PCurve::Affine { origin, direction } = c.pcurve else {
                unreachable!()
            };
            c.pcurve = PCurve::Affine {
                origin: [origin[1], -origin[0]],
                direction: [direction[1], -direction[0]],
            };
        }
    }
    part.validate(t.absolute())?;
    merge_coplanar_faces(&part, t)
}
pub fn reframed_merge_demo_json(offset: f64) -> Result<String> {
    reframed_merge_demo(offset)?.mesh_json(0.05, Tolerance::default())
}
