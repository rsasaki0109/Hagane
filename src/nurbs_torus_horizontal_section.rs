//! Reusable exact horizontal section loops with retained plane/source pcurves.
//! Both loops traverse increasing major U; this is not an annular face or solid.
//! The initial scope inherits the checked horizontal split's precision and
//! finite-volume admission, including its literal-zero and resolved-tip guards.
use crate::*;
#[derive(Clone, Debug)]
pub struct NurbsTorusSectionFaceUse {
    pub edge: usize,
    pub face: usize,
    pub pcurve: PCurve,
}
#[derive(Clone, Debug)]
pub struct NurbsTorusHorizontalSection {
    source: NurbsTorusSolid,
    cut: f64,
    plane: Surface,
    vertices: [Vertex; 8],
    edges: [Edge; 8],
    loops: [Wire; 2],
    uses: Vec<NurbsTorusSectionFaceUse>,
}
fn unresolved() -> Error {
    Error::Unsupported(
        "horizontal torus section provenance or same-parameter precision is unresolved",
    )
}
fn norm(v: Vec3) -> f64 {
    v.x.hypot(v.y).hypot(v.z)
}
fn same_pcurve(a: &PCurve, b: &PCurve) -> bool {
    match (a, b) {
        (
            PCurve::Affine {
                origin: a,
                direction: b,
            },
            PCurve::Affine {
                origin: c,
                direction: d,
            },
        ) => a == c && b == d,
        (PCurve::Nurbs(a), PCurve::Nurbs(b)) => {
            a.degree() == b.degree()
                && a.knots() == b.knots()
                && a.weights() == b.weights()
                && a.control_points() == b.control_points()
        }
        _ => false,
    }
}
fn extract(
    source: &NurbsTorusSolid,
    cut: f64,
    policy: GeometryTolerance,
) -> Result<NurbsTorusHorizontalSection> {
    let split = source.split_by_local_z(cut, policy)?;
    let lower = split.lower;
    let strips = lower.strip_ranges();
    let n = strips.len();
    let solid = lower.solid();
    let cap = &solid.shell.faces[4 * n];
    let mut vertices = Vec::new();
    let mut edges = Vec::new();
    let mut loops = Vec::new();
    let mut uses = Vec::new();
    for ring in 0..2 {
        let j = if ring == 0 { n } else { 0 };
        let (q, v) = if ring == 0 {
            let (q, [_, v]) = strips[n - 1];
            (q, v)
        } else {
            let (q, [v, _]) = strips[0];
            (q, v)
        };
        let mut coedges = Vec::new();
        for i in 0..4 {
            let old = i * (n + 1) + j;
            let index = ring * 4 + i;
            let edge = &solid.edges[old];
            vertices.push(solid.vertices[edge.vertices[0]].clone());
            let mut edge = edge.clone();
            edge.vertices = [index, ring * 4 + (i + 1) % 4];
            edges.push(edge);
            let cap_use = cap
                .wires
                .iter()
                .flat_map(|w| &w.coedges)
                .find(|c| c.edge == old)
                .ok_or_else(unresolved)?;
            coedges.push(Coedge {
                edge: index,
                forward: true,
                pcurve: cap_use.pcurve.clone(),
            });
            uses.push(NurbsTorusSectionFaceUse {
                edge: index,
                face: i * 4 + q,
                pcurve: PCurve::Affine {
                    origin: [0., v],
                    direction: [1., 0.],
                },
            });
            if cut == 0. {
                let (q, v) = if v == 0. {
                    ((q + 3) % 4, 1.)
                } else {
                    ((q + 1) % 4, 0.)
                };
                uses.push(NurbsTorusSectionFaceUse {
                    edge: index,
                    face: i * 4 + q,
                    pcurve: PCurve::Affine {
                        origin: [0., v],
                        direction: [1., 0.],
                    },
                });
            }
        }
        loops.push(Wire { coedges });
    }
    Ok(NurbsTorusHorizontalSection {
        source: source.clone(),
        cut,
        plane: cap.surface.clone(),
        vertices: vertices.try_into().map_err(|_| unresolved())?,
        edges: edges.try_into().map_err(|_| unresolved())?,
        loops: loops.try_into().map_err(|_| unresolved())?,
        uses,
    })
}
impl NurbsTorusSolid {
    /// Two closed ring wires, with complete original-face attachment provenance.
    /// A zero-height rim has two adjacent original source face uses per edge.
    pub fn section_by_local_z(
        &self,
        cut: f64,
        policy: GeometryTolerance,
    ) -> Result<NurbsTorusHorizontalSection> {
        let section = extract(self, cut, policy)?;
        section.validate(policy)?;
        Ok(section)
    }
}
impl NurbsTorusHorizontalSection {
    pub fn source(&self) -> &NurbsTorusSolid {
        &self.source
    }
    pub fn cut_height(&self) -> f64 {
        self.cut
    }
    pub fn plane(&self) -> &Surface {
        &self.plane
    }
    pub fn vertices(&self) -> &[Vertex; 8] {
        &self.vertices
    }
    pub fn edges(&self) -> &[Edge; 8] {
        &self.edges
    }
    /// Outer then inner loop; BOTH traverse increasing major U, not a face boundary.
    pub fn loops(&self) -> &[Wire; 2] {
        &self.loops
    }
    pub fn source_uses(&self) -> &[NurbsTorusSectionFaceUse] {
        &self.uses
    }
    pub fn validate(&self, policy: GeometryTolerance) -> Result<()> {
        self.source.validate(policy)?;
        let expected = extract(&self.source, self.cut, policy)?;
        let same_plane = matches!((&self.plane,&expected.plane),(Surface::Plane{origin:a,u:b,v:c},Surface::Plane{origin:d,u:e,v:f}) if a==d&&b==e&&c==f);
        let equal = same_plane
            && self
                .vertices
                .iter()
                .zip(&expected.vertices)
                .all(|(a, b)| a.point == b.point)
            && self.edges.iter().zip(&expected.edges).all(|(a, b)| {
                a.vertices == b.vertices
                    && crate::nurbs_graph_step_import::same_curve(&a.curve, &b.curve)
            })
            && self.loops.iter().zip(&expected.loops).all(|(a, b)| {
                a.coedges.len() == b.coedges.len()
                    && a.coedges.iter().zip(&b.coedges).all(|(a, b)| {
                        a.edge == b.edge
                            && a.forward == b.forward
                            && same_pcurve(&a.pcurve, &b.pcurve)
                    })
            })
            && self.uses.len() == expected.uses.len()
            && self.uses.iter().zip(&expected.uses).all(|(a, b)| {
                a.edge == b.edge && a.face == b.face && same_pcurve(&a.pcurve, &b.pcurve)
            });
        if !equal {
            return Err(Error::InvalidTopology(
                "torus section differs from complete actual split/source-attachment certificate",
            ));
        }
        let [major, minor] = self.source.radii();
        let o = self.source.frame().origin();
        let error = 65536.
            * f64::EPSILON
            * (2. * (major + minor).hypot(minor) + o.x.abs().max(o.y.abs()).max(o.z.abs()));
        for wire in &self.loops {
            for (i, c) in wire.coedges.iter().enumerate() {
                let next = &wire.coedges[(i + 1) % 4];
                let edge = &self.edges[c.edge];
                if !c.forward || edge.vertices[1] != self.edges[next.edge].vertices[0] {
                    return Err(Error::InvalidTopology(
                        "torus section loop is not closed in increasing major U",
                    ));
                }
                let (Curve::Nurbs(curve), PCurve::Nurbs(uv)) = (&edge.curve, &c.pcurve) else {
                    return Err(unresolved());
                };
                if curve.weights() != uv.weights() || curve.knots() != uv.knots() {
                    return Err(unresolved());
                }
                for (point, uv) in curve.control_points().iter().zip(uv.control_points()) {
                    if norm(self.plane.try_evaluate(uv.x, uv.y)? - *point) + error
                        >= policy.linear() / 8.
                    {
                        return Err(unresolved());
                    }
                }
            }
        }
        for use_ in &self.uses {
            for t in [0., 0.25, 0.5, 0.75, 1.] {
                let uv = use_.pcurve.try_evaluate(t)?;
                let p = self.source.solid().shell.faces[use_.face]
                    .surface
                    .try_evaluate(uv[0], uv[1])?;
                let curve = self.edges[use_.edge].curve.try_evaluate(t)?;
                if norm(p - curve) + error >= policy.linear() / 4. {
                    return Err(unresolved());
                }
            }
        }
        Ok(())
    }
}
