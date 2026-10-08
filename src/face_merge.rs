//! Topological union of adjacent faces with certified identical planar support.
use crate::*;
fn same_support(a: &Face, b: &Face) -> bool {
    let (
        Surface::Plane {
            origin: ao,
            u: au,
            v: av,
        },
        Surface::Plane {
            origin: bo,
            u: bu,
            v: bv,
        },
    ) = (&a.surface, &b.surface)
    else {
        return false;
    };
    if ao == bo && au == bu && av == bv && a.orientation == b.orientation {
        return true;
    }
    let an = au.cross(*av) * (a.orientation as f64);
    let bn = bu.cross(*bv) * (b.orientation as f64);
    if an != bn {
        return false;
    }
    let normals = [
        Vec3::new(1., 0., 0.),
        Vec3::new(0., 1., 0.),
        Vec3::new(0., 0., 1.),
    ];
    for (axis, n) in normals.into_iter().enumerate() {
        if an == n || an == n * (-1.) {
            return [ao.x, ao.y, ao.z][axis] == [bo.x, bo.y, bo.z][axis];
        }
    }
    false
}
/// Merge edge-connected planar straight-edge faces on identical supports.
/// Supports must have identical origin/UV frame and orientation, or exact equal
/// axis-aligned planes with equal outward normals. Other planes remain separate.
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
        if a != b && same_support(&faces[a], &faces[b]) {
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
    // original affine pcurves whenever the merged face uses that same frame.
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
            if matches!(old.surface,Surface::Plane{origin:o,u:a,v:b} if o==origin && a==u && b==v) {
                for c in old.wires.iter().flat_map(|w| &w.coedges) {
                    original_curves.insert(c.edge, &c.pcurve);
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
