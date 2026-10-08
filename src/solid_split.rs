//! Plane partition of planar straight-edge B-rep solids.
use crate::*;
#[derive(Clone, Debug)]
pub struct SolidPlaneSplit {
    /// Material on the negative side of the plane's u cross v normal.
    pub negative: Solid,
    pub positive: Solid,
    /// Exact section faces, oriented along the plane normal.
    pub section: Vec<PlanarFacePatch>,
}
fn cycles(mut edges: Vec<(usize, usize)>) -> Result<Vec<Vec<usize>>> {
    let mut outgoing = std::collections::BTreeMap::new();
    for (a, b) in edges.drain(..) {
        if outgoing.insert(a, b).is_some() {
            return Err(Error::Unsupported("section graph has ambiguous branches"));
        }
    }
    let mut rings = Vec::new();
    while let Some((&start, _)) = outgoing.first_key_value() {
        let mut ring = Vec::new();
        let mut vertex = start;
        loop {
            ring.push(vertex);
            vertex = outgoing
                .remove(&vertex)
                .ok_or(Error::InvalidTopology("section graph is open"))?;
            if vertex == start {
                break;
            }
        }
        rings.push(ring);
    }
    Ok(rings)
}
fn patches(
    surface: &Surface,
    orientation: i8,
    rings: Vec<Vec<usize>>,
    points: &[Point3],
) -> Result<Vec<PlanarFacePatch>> {
    let mut outer = Vec::new();
    let mut holes = Vec::new();
    for ring in rings {
        let uv: Vec<_> = ring
            .iter()
            .map(|&i| surface.parameters(points[i]))
            .collect();
        let area = crate::planar::polygon_area(&uv);
        if !area.is_finite() || area == 0.0 {
            return Err(Error::Unsupported("section has unresolved area"));
        }
        if area > 0.0 {
            outer.push((ring, uv));
        } else {
            holes.push((ring, uv));
        }
    }
    let mut result: Vec<_> = outer
        .iter()
        .map(|(ring, _)| PlanarFacePatch {
            surface: surface.clone(),
            orientation,
            rings: vec![ring.iter().map(|&i| points[i]).collect()],
        })
        .collect();
    for (ring, uv) in holes {
        let mut owner = None;
        for (i, (_, o)) in outer.iter().enumerate() {
            match locate_point_in_polygon(uv[0], o)? {
                PointLocation::Inside => {
                    if owner.replace(i).is_some() {
                        return Err(Error::Unsupported("section hole has multiple owners"));
                    }
                }
                PointLocation::Boundary => {
                    return Err(Error::Unsupported("section hole touches outer boundary"))
                }
                PointLocation::Outside => (),
            }
        }
        result[owner.ok_or(Error::InvalidTopology("section hole has no owner"))?]
            .rings
            .push(ring.iter().map(|&i| points[i]).collect());
    }
    Ok(result)
}
/// Split a validated planar straight-edge solid by a transverse plane.
/// All original vertices must clear the plane by ten local length budgets.
/// Both sides must form one connected closed solid. Contacts, coincident faces,
/// curved boundaries and disconnected results return explicit errors.
pub fn split_solid_by_plane(
    solid: &Solid,
    plane: &Surface,
    tol: GeometryTolerance,
) -> Result<SolidPlaneSplit> {
    let source = planar_face_patches(solid, tol.absolute())?;
    let Surface::Plane { origin, u, v } = *plane else {
        return Err(Error::Unsupported("solid partition requires a plane"));
    };
    Frame3::new(origin, [u, v, u.cross(v)], tol.absolute())?;
    let normal = u.cross(v);
    let volume = solid.volume()?;
    let budget = tol.length_at_scale((solid.bounds().max - solid.bounds().min).norm())?;
    let distance: Vec<_> = solid
        .vertices
        .iter()
        .map(|p| (p.point - origin).dot(normal))
        .collect();
    if distance
        .iter()
        .any(|d| !d.is_finite() || d.abs() <= 10.0 * budget)
    {
        return Err(Error::Unsupported(
            "cut plane touches or nearly touches an original vertex",
        ));
    }
    if !distance.iter().any(|&d| d < 0.) || !distance.iter().any(|&d| d > 0.) {
        return Err(Error::Unsupported("cut plane must cross the solid"));
    }
    let mut points: Vec<_> = solid.vertices.iter().map(|v| v.point).collect();
    let mut crossed = std::collections::BTreeMap::new();
    for (i, e) in solid.edges.iter().enumerate() {
        let [a, b] = e.vertices;
        if (distance[a] > 0.) != (distance[b] > 0.) {
            let da = distance[a];
            let db = distance[b];
            let scale = da.abs().max(db.abs());
            let t = (da / scale) / ((da / scale) - (db / scale));
            let p = points[a] + (points[b] - points[a]) * t;
            if !p.finite() || (p - origin).dot(normal).abs() > tol.linear() {
                return Err(Error::Unsupported("cut intersection loses plane precision"));
            }
            crossed.insert(i, points.len());
            points.push(p);
        }
    }
    let mut children: [Vec<PlanarFacePatch>; 2] = [Vec::new(), Vec::new()];
    let mut cap = Vec::new();
    for (fi, face) in solid.shell.faces.iter().enumerate() {
        let ids: Vec<_> = face
            .wires
            .iter()
            .flat_map(|w| &w.coedges)
            .flat_map(|c| solid.edges[c.edge].vertices)
            .collect();
        let has_positive = ids.iter().any(|&i| distance[i] > 0.);
        let has_negative = ids.iter().any(|&i| distance[i] < 0.);
        if !has_positive || !has_negative {
            children[usize::from(has_positive)].push(source[fi].clone());
            continue;
        }
        let PlanePlaneIntersection::Line(line) = intersect_plane_plane(&face.surface, plane, tol)?
        else {
            return Err(Error::Unsupported(
                "crossing face has unresolved plane intersection",
            ));
        };
        let clip = clip_line_to_planar_face(solid, fi, line.origin, line.direction, tol)?;
        let mut boundaries: [Vec<(usize, usize)>; 2] = [Vec::new(), Vec::new()];
        for w in &face.wires {
            for c in &w.coedges {
                let e = &solid.edges[c.edge];
                let a = e.vertices[usize::from(!c.forward)];
                let b = e.vertices[usize::from(c.forward)];
                if let Some(&hit) = crossed.get(&c.edge) {
                    boundaries[usize::from(distance[a] > 0.)].push((a, hit));
                    boundaries[usize::from(distance[b] > 0.)].push((hit, b));
                } else {
                    boundaries[usize::from(distance[a] > 0.)].push((a, b));
                }
            }
        }
        let Surface::Plane { u, v, .. } = face.surface else {
            unreachable!()
        };
        let left_positive = normal.dot(u.cross(v).cross(line.direction)) > 0.;
        for interval in clip.intervals {
            let a = *crossed
                .get(&interval.start.edge)
                .ok_or(Error::InvalidTopology("section event lacks shared vertex"))?;
            let b = *crossed
                .get(&interval.end.edge)
                .ok_or(Error::InvalidTopology("section event lacks shared vertex"))?;
            let positive = if left_positive { (a, b) } else { (b, a) };
            boundaries[1].push(positive);
            boundaries[0].push((positive.1, positive.0));
            cap.push(if face.orientation > 0 {
                positive
            } else {
                (positive.1, positive.0)
            });
        }
        for (side, boundary) in boundaries.into_iter().enumerate() {
            children[side].extend(patches(
                &face.surface,
                face.orientation,
                cycles(boundary)?,
                &points,
            )?);
        }
    }
    let section = patches(plane, 1, cycles(cap)?, &points)?;
    if section.is_empty() {
        return Err(Error::Unsupported("cut has no material section"));
    }
    for patch in &section {
        children[0].push(patch.clone());
        let mut positive = patch.clone();
        positive.orientation = -1;
        children[1].push(positive);
    }
    let negative = sew_planar_faces(&children[0], tol)?;
    let positive = sew_planar_faces(&children[1], tol)?;
    if (negative.volume()? + positive.volume()? - volume).abs() > volume.abs() * 1e-10 {
        return Err(Error::InvalidTopology(
            "solid partition does not conserve volume",
        ));
    }
    Ok(SolidPlaneSplit {
        negative,
        positive,
        section,
    })
}
/// Two exact, independently closed demo parts; mesh JSON is display-only.
pub fn solid_split_demo_json(offset: f64) -> Result<String> {
    let tol = GeometryTolerance::default();
    let s = make_box(
        BoxSpec {
            min: Point3::new(-40., -30., -12.),
            size: Vec3::new(80., 60., 24.),
        },
        tol.absolute(),
    )?;
    let plane = Surface::Plane {
        origin: Point3::new(0., 0., offset),
        u: Vec3::new(1., 0., 0.),
        v: Vec3::new(0., 1., 0.),
    };
    let r = split_solid_by_plane(&s, &plane, tol)?;
    Ok(format!(
        "{{\"negative\":{},\"positive\":{},\"section_faces\":{},\"original_volume\":{}}}",
        r.negative.mesh_json(0.05, tol.absolute())?,
        r.positive.mesh_json(0.05, tol.absolute())?,
        r.section.len(),
        s.volume()?
    ))
}
