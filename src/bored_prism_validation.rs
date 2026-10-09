//! Geometric certificate for polygon prisms with separated normal through/blind bores.
use crate::*;
use std::collections::{BTreeMap, BTreeSet};
/// Verified geometric structure; it does not recover original modeling history.
#[derive(Debug, Clone, PartialEq)]
pub struct BoredPrismCertificate {
    /// Original face indices, lower then upper in the common positive UV frame.
    pub stock_caps: [usize; 2],
    pub stock_translation: Vec3,
    pub bores: Vec<CircularPrismCertificate>,
    /// Axial cut intervals measured from the lower stock cap, in length units.
    /// Geometry is retained; these intervals are not recovered operation history.
    pub bore_intervals: Vec<[f64; 2]>,
    /// Conservative physical lower bound over side/opening/pair separation.
    pub minimum_clearance: f64,
}
const DOMAIN: Error = Error::Unsupported(
    "solid is not a certified polygon prism with separated normal through/blind bores",
);
// Copy an exact subset for proof only, retaining geometry and oriented pcurves.
// No primitive reconstruction, coordinate welding, or mesh operation occurs.
fn subset(source: &Solid, mut faces: Vec<Face>) -> Solid {
    let used_edges: BTreeSet<_> = faces
        .iter()
        .flat_map(|f| &f.wires)
        .flat_map(|w| &w.coedges)
        .map(|c| c.edge)
        .collect();
    let used_vertices: BTreeSet<_> = used_edges
        .iter()
        .flat_map(|&e| source.edges[e].vertices)
        .collect();
    let vertex_map: BTreeMap<_, _> = used_vertices
        .iter()
        .enumerate()
        .map(|(i, &v)| (v, i))
        .collect();
    let vertices = used_vertices
        .iter()
        .map(|&v| source.vertices[v].clone())
        .collect();
    let edge_map: BTreeMap<_, _> = used_edges
        .iter()
        .enumerate()
        .map(|(i, &e)| (e, i))
        .collect();
    let edges = used_edges
        .iter()
        .map(|&e| {
            let mut edge = source.edges[e].clone();
            edge.vertices = edge.vertices.map(|v| vertex_map[&v]);
            edge
        })
        .collect();
    for face in &mut faces {
        for wire in &mut face.wires {
            for c in &mut wire.coedges {
                c.edge = edge_map[&c.edge];
            }
        }
    }
    Solid {
        vertices,
        edges,
        shell: Shell { faces },
    }
}
/// Validate polygon-prism stock, complete periodic inward walls, exact cap/rim
/// ownership, resolved blind floors, and depth-bounded swept-footprint clearance.
/// Outward curved stock, internal cavities, oblique bores and intersecting tools
/// are unsupported.
pub fn certify_bored_prism(s: &Solid, t: Tolerance) -> Result<BoredPrismCertificate> {
    s.validate(t)?;
    certify_validated_bored_prism(s, t)
}
pub(crate) fn certify_validated_bored_prism(
    s: &Solid,
    t: Tolerance,
) -> Result<BoredPrismCertificate> {
    if s.shell.faces.len() > 128 {
        return Err(DOMAIN);
    }
    let walls: Vec<_> = s
        .shell
        .faces
        .iter()
        .enumerate()
        .filter(|(_, f)| !matches!(f.surface, Surface::Plane { .. }))
        .collect();
    if walls.is_empty() || walls.len() > 64 {
        return Err(DOMAIN);
    }
    let diagonal = (s.bounds().max - s.bounds().min).norm();
    let budget = (64. * f64::EPSILON * diagonal).min(t.linear / 1024.);
    if !diagonal.is_finite() || budget <= 0. || !s.volume()?.is_finite() {
        return Err(DOMAIN);
    }
    let mut removed_rims = BTreeSet::new();
    let mut removed_edges = BTreeSet::new();
    let mut bore_caps = Vec::new();
    let mut floors = BTreeSet::new();
    let mut entries = BTreeSet::new();
    let mut bores = Vec::new();
    for (_, wall) in &walls {
        if wall.orientation != -1
            || !matches!(
                wall.surface,
                Surface::Cylinder { .. } | Surface::FramedCylinder { .. }
            )
            || wall.cylinder_span()? != std::f64::consts::TAU
        {
            return Err(DOMAIN);
        }
        let c = &wall.wires[0].coedges;
        if c[1].edge != c[3].edge {
            return Err(DOMAIN);
        }
        let mut caps = Vec::new();
        let mut indices = Vec::new();
        for rim in [c[0].edge, c[2].edge] {
            if !removed_rims.insert(rim) {
                return Err(DOMAIN);
            }
            let mut owner = None;
            for (fi, face) in s.shell.faces.iter().enumerate() {
                if !matches!(face.surface, Surface::Plane { .. }) {
                    continue;
                }
                for (wi, wire) in face.wires.iter().enumerate() {
                    if wire.coedges.iter().any(|c| c.edge == rim) {
                        if wire.coedges.len() != 1
                            || owner.is_some()
                            || (wi == 0 && face.wires.len() != 1)
                        {
                            return Err(DOMAIN);
                        }
                        owner = Some((fi, wi == 0, wire));
                    }
                }
            }
            let (fi, floor, wire) = owner.ok_or(DOMAIN)?;
            if floor {
                if !floors.insert(fi) {
                    return Err(DOMAIN);
                }
            } else {
                entries.insert(fi);
            }
            let mut wire = wire.clone();
            wire.coedges[0].forward = true;
            caps.push(Face {
                surface: s.shell.faces[fi].surface.clone(),
                wires: vec![wire],
                orientation: s.shell.faces[fi].orientation * if floor { -1 } else { 1 },
            });
            indices.push(fi);
        }
        if indices[0] == indices[1] || indices.iter().all(|i| floors.contains(i)) {
            return Err(DOMAIN);
        }
        for edge in [c[0].edge, c[1].edge, c[2].edge] {
            if !removed_edges.insert(edge) {
                return Err(DOMAIN);
            }
        }
        let mut positive_wall = (*wall).clone();
        positive_wall.orientation = 1;
        caps.push(positive_wall);
        let cylinder = subset(s, caps);
        bores.push(certify_circular_prism(&cylinder, t)?);
        bore_caps.push([indices[0], indices[1]]);
    }
    if !(1..=2).contains(&entries.len()) {
        return Err(DOMAIN);
    }
    // Entry owners determine the stock axis even for a box with reordered faces.
    let order = entries
        .iter()
        .copied()
        .chain((0..s.shell.faces.len()).filter(|i| !entries.contains(i)));
    let mut stock_faces = Vec::new();
    let mut stock_to_original = Vec::new();
    for fi in order {
        let face = &s.shell.faces[fi];
        if !matches!(face.surface, Surface::Plane { .. }) || floors.contains(&fi) {
            continue;
        }
        let mut face = face.clone();
        face.wires
            .retain(|w| !w.coedges.iter().any(|c| removed_rims.contains(&c.edge)));
        if face.wires.is_empty()
            || face
                .wires
                .iter()
                .flat_map(|w| &w.coedges)
                .any(|c| removed_edges.contains(&c.edge))
        {
            return Err(DOMAIN);
        }
        stock_faces.push(face);
        stock_to_original.push(fi);
    }
    let stock = subset(s, stock_faces);
    if stock.edges.len() + removed_edges.len() != s.edges.len()
        || stock.vertices.len() + 2 * bores.len() != s.vertices.len()
    {
        return Err(DOMAIN);
    }
    let certificate = certify_planar_prism(&stock, t)?;
    for edge in &stock.edges {
        let Curve::Line { a, b } = edge.curve else {
            return Err(DOMAIN);
        };
        if (a - stock.vertices[edge.vertices[0]].point).norm() > budget
            || (b - stock.vertices[edge.vertices[1]].point).norm() > budget
        {
            return Err(DOMAIN);
        }
    }
    let [a, b] = certificate.cap_faces;
    let (lower, upper, translation) =
        if stock.shell.faces[a].orientation == -1 && stock.shell.faces[b].orientation == 1 {
            (a, b, certificate.translation)
        } else if stock.shell.faces[b].orientation == -1 && stock.shell.faces[a].orientation == 1 {
            (b, a, certificate.translation * (-1.))
        } else {
            return Err(DOMAIN);
        };
    let cap = &stock.shell.faces[lower];
    let Surface::Plane { u, v, .. } = cap.surface else {
        return Err(DOMAIN);
    };
    let normal = u.cross(v).normalized()?;
    let first = &cap.wires[0].coedges[0];
    let anchor =
        stock.vertices[stock.edges[first.edge].vertices[if first.forward { 0 } else { 1 }]].point;
    let frame = Frame3::new(anchor, [u, v, normal], t)?;
    let height = translation.dot(normal);
    let offset = [translation.dot(u), translation.dot(v)];
    if !height.is_finite() || height <= 10. * t.linear || offset.iter().any(|x| !x.is_finite()) {
        return Err(DOMAIN);
    }
    let rings: Vec<Vec<_>> = cap
        .wires
        .iter()
        .map(|w| {
            w.coedges
                .iter()
                .map(|c| {
                    let edge = &stock.edges[c.edge];
                    let local = frame.local_point(
                        stock.vertices[edge.vertices[if c.forward { 0 } else { 1 }]].point,
                    );
                    [local.x, local.y]
                })
                .collect()
        })
        .collect();
    let stock_caps = [stock_to_original[lower], stock_to_original[upper]];
    // Cover projection/distance/frame roundoff before claiming separation,
    // including callers whose modeling tolerance is below local arithmetic noise.
    let separation_roundoff = 256. * f64::EPSILON * diagonal;
    let mut minimum_clearance = f64::INFINITY;
    let mut centers = Vec::new();
    let mut bore_intervals = Vec::new();
    for (bore, caps) in bores.iter().zip(&bore_caps) {
        let local = frame.local_point(bore.axis.origin());
        if !local.finite()
            || bore
                .axis
                .axes()
                .iter()
                .zip(frame.axes())
                .any(|(a, b)| (*a - b).norm() > 64. * f64::EPSILON)
        {
            return Err(DOMAIN);
        }
        let endpoints = [local.z, local.z + bore.height];
        if endpoints.iter().any(|z| !z.is_finite()) {
            return Err(DOMAIN);
        }
        let mut interval = endpoints;
        for k in 0..2 {
            if floors.contains(&caps[k]) {
                let web = if k == 0 {
                    endpoints[k]
                } else {
                    height - endpoints[k]
                };
                if endpoints[k] <= 0.
                    || endpoints[k] >= height
                    || web - separation_roundoff <= 10. * t.linear
                {
                    return Err(Error::Unsupported(
                        "blind bore lacks a resolved stock floor",
                    ));
                }
                minimum_clearance = minimum_clearance.min(web - separation_roundoff);
            } else {
                let z = if k == 0 { 0. } else { height };
                if caps[k] != stock_caps[k] || (endpoints[k] - z).abs() > budget {
                    return Err(DOMAIN);
                }
                // Only the proof interval uses the certified stock-cap value.
                // Original vertices, surfaces and curves remain unchanged.
                interval[k] = z;
            }
        }
        bore_intervals.push(interval);
        let center = [local.x, local.y];
        let clearance = crate::operations::swept_polygon_region_bore_clearance(
            &rings[0],
            &rings[1..],
            center,
            bore.outer_radius,
            offset,
            height,
            [interval[0] / height, interval[1] / height],
        )?
        .0 - separation_roundoff;
        if !clearance.is_finite() || clearance <= 10. * t.linear {
            return Err(Error::Unsupported(
                "bore crosses or nearly touches a swept stock boundary",
            ));
        }
        minimum_clearance = minimum_clearance.min(clearance);
        centers.push(center);
    }
    for i in 0..bores.len() {
        for j in i + 1..bores.len() {
            let radial = (centers[i][0] - centers[j][0]).hypot(centers[i][1] - centers[j][1])
                - bores[i].outer_radius
                - bores[j].outer_radius;
            let axial = (bore_intervals[i][0] - bore_intervals[j][1])
                .max(bore_intervals[j][0] - bore_intervals[i][1]);
            let clearance = radial.max(0.).hypot(axial.max(0.)) - separation_roundoff;
            if !clearance.is_finite() || clearance <= 10. * t.linear {
                return Err(Error::Unsupported(
                    "bores intersect, touch or lack resolved separation",
                ));
            }
            minimum_clearance = minimum_clearance.min(clearance);
        }
    }
    Ok(BoredPrismCertificate {
        stock_caps,
        stock_translation: translation,
        bores,
        bore_intervals,
        minimum_clearance,
    })
}
