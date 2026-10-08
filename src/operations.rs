use crate::*;
use std::f64::consts::TAU;
#[derive(Clone, Copy, Debug)]
pub struct BoxSpec {
    pub min: Point3,
    pub size: Vec3,
}
#[derive(Clone, Copy, Debug)]
pub struct CylinderSpec {
    pub base: Point3,
    pub radius: f64,
    pub height: f64,
}
fn positive(v: f64, t: Tolerance) -> bool {
    v.is_finite() && v > t.linear * 10.0
}
fn check_box(b: BoxSpec, t: Tolerance) -> Result<()> {
    Tolerance::new(t.linear)?;
    if !b.min.finite()
        || !positive(b.size.x, t)
        || !positive(b.size.y, t)
        || !positive(b.size.z, t)
        || !(b.min + b.size).finite()
    {
        return Err(Error::InvalidInput(
            "box dimensions must exceed ten linear tolerances",
        ));
    }
    Ok(())
}
fn check_cylinder(c: CylinderSpec, t: Tolerance) -> Result<()> {
    Tolerance::new(t.linear)?;
    if !c.base.finite()
        || !positive(c.radius, t)
        || !positive(c.height, t)
        || !(c.base + Vec3::new(c.radius, c.radius, c.height)).finite()
    {
        return Err(Error::InvalidInput(
            "cylinder dimensions must exceed ten linear tolerances",
        ));
    }
    Ok(())
}
fn plane_wire(s: &Solid, ids: &[usize], surface: &Surface) -> Wire {
    let coedges = (0..ids.len())
        .map(|i| {
            let a = ids[i];
            let b = ids[(i + 1) % ids.len()];
            let (edge, e) = s
                .edges
                .iter()
                .enumerate()
                .find(|(_, e)| e.vertices == [a, b] || e.vertices == [b, a])
                .expect("builder edge");
            let x = surface.parameters(s.vertices[e.vertices[0]].point);
            let y = surface.parameters(s.vertices[e.vertices[1]].point);
            Coedge {
                edge,
                forward: e.vertices == [a, b],
                pcurve: PCurve::Affine {
                    origin: x,
                    direction: [y[0] - x[0], y[1] - x[1]],
                },
            }
        })
        .collect();
    Wire { coedges }
}
pub fn make_box(b: BoxSpec, t: Tolerance) -> Result<Solid> {
    check_box(b, t)?;
    let x = b.size.x;
    let y = b.size.y;
    let z = b.size.z;
    let mut s = Solid {
        vertices: vec![],
        edges: vec![],
        shell: Shell { faces: vec![] },
    };
    for p in [
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(x, 0.0, 0.0),
        Vec3::new(x, y, 0.0),
        Vec3::new(0.0, y, 0.0),
        Vec3::new(0.0, 0.0, z),
        Vec3::new(x, 0.0, z),
        Vec3::new(x, y, z),
        Vec3::new(0.0, y, z),
    ] {
        s.vertices.push(Vertex { point: b.min + p });
    }
    for [a, b] in [
        [0, 1],
        [1, 2],
        [2, 3],
        [3, 0],
        [4, 5],
        [5, 6],
        [6, 7],
        [7, 4],
        [0, 4],
        [1, 5],
        [2, 6],
        [3, 7],
    ] {
        s.edges.push(Edge {
            vertices: [a, b],
            curve: Curve::Line {
                a: s.vertices[a].point,
                b: s.vertices[b].point,
            },
        });
    }
    for (ids, orientation) in [
        ([0, 1, 2, 3], -1),
        ([4, 5, 6, 7], 1),
        ([0, 1, 5, 4], 1),
        ([1, 2, 6, 5], 1),
        ([2, 3, 7, 6], 1),
        ([3, 0, 4, 7], 1),
    ] {
        let o = s.vertices[ids[0]].point;
        let u = (s.vertices[ids[1]].point - o).normalized()?;
        let v = (s.vertices[ids[3]].point - o).normalized()?;
        let surface = Surface::Plane { origin: o, u, v };
        let wire = plane_wire(&s, &ids, &surface);
        s.shell.faces.push(Face {
            surface,
            wires: vec![wire],
            orientation,
        });
    }
    s.validate(t)?;
    Ok(s)
}
fn circle_edge(s: &mut Solid, curve: Curve) -> usize {
    let v = s.vertices.len();
    s.vertices.push(Vertex {
        point: curve.evaluate(0.0),
    });
    let e = s.edges.len();
    s.edges.push(Edge {
        vertices: [v, v],
        curve,
    });
    e
}
fn ring(s: &mut Solid, center: Point3, radius: f64) -> usize {
    circle_edge(s, Curve::Circle { center, radius })
}
fn cap_ring(s: &Solid, e: usize, surface: &Surface, forward: bool) -> Wire {
    let Curve::Circle { center, radius } = s.edges[e].curve else {
        unreachable!()
    };
    Wire {
        coedges: vec![Coedge {
            edge: e,
            forward,
            pcurve: PCurve::Circle {
                center: surface.parameters(center),
                radius,
            },
        }],
    }
}
fn cylindrical_face(s: &mut Solid, bottom: usize, top: usize, c: CylinderSpec, orientation: i8) {
    let a = s.edges[bottom].vertices[0];
    let b = s.edges[top].vertices[0];
    let seam = s.edges.len();
    s.edges.push(Edge {
        vertices: [a, b],
        curve: Curve::Line {
            a: s.vertices[a].point,
            b: s.vertices[b].point,
        },
    });
    let cedge = |edge, forward, origin, direction| Coedge {
        edge,
        forward,
        pcurve: PCurve::Affine { origin, direction },
    };
    let wire = Wire {
        coedges: vec![
            cedge(bottom, true, [0.0, 0.0], [1.0, 0.0]),
            cedge(seam, true, [TAU, 0.0], [0.0, c.height]),
            cedge(top, false, [0.0, c.height], [1.0, 0.0]),
            cedge(seam, false, [0.0, 0.0], [0.0, c.height]),
        ],
    };
    s.shell.faces.push(Face {
        surface: Surface::Cylinder {
            center: c.base,
            radius: c.radius,
            height: c.height,
        },
        wires: vec![wire],
        orientation,
    });
}
pub fn make_cylinder(c: CylinderSpec, t: Tolerance) -> Result<Solid> {
    check_cylinder(c, t)?;
    let mut s = Solid {
        vertices: vec![],
        edges: vec![],
        shell: Shell { faces: vec![] },
    };
    let bottom = ring(&mut s, c.base, c.radius);
    let top = ring(&mut s, c.base + Vec3::new(0.0, 0.0, c.height), c.radius);
    for (e, orientation, z) in [(bottom, -1, 0.0), (top, 1, c.height)] {
        let surface = Surface::Plane {
            origin: c.base + Vec3::new(0.0, 0.0, z),
            u: Vec3::new(1.0, 0.0, 0.0),
            v: Vec3::new(0.0, 1.0, 0.0),
        };
        let wire = cap_ring(&s, e, &surface, true);
        s.shell.faces.push(Face {
            surface,
            wires: vec![wire],
            orientation,
        });
    }
    cylindrical_face(&mut s, bottom, top, c, 1);
    s.validate(t)?;
    Ok(s)
}
/// Narrow-phase primitive difference, returning a genuine shared-edge B-rep.
/// Only Z-aligned, strictly interior, strictly overhanging through cylinders.
/// Specs are immutable operands; no claim of general Solid/Solid CSG is made.
pub fn subtract_through_cylinder(b: BoxSpec, c: CylinderSpec, t: Tolerance) -> Result<Solid> {
    check_box(b, t)?;
    check_cylinder(c, t)?;
    let max = b.min + b.size;
    if c.base.z >= b.min.z - t.linear || c.base.z + c.height <= max.z + t.linear {
        return Err(Error::Unsupported(
            "cylinder must strictly overhang both box caps",
        ));
    }
    if c.base.x - c.radius <= b.min.x + t.linear
        || c.base.y - c.radius <= b.min.y + t.linear
        || c.base.x + c.radius >= max.x - t.linear
        || c.base.y + c.radius >= max.y - t.linear
    {
        return Err(Error::Unsupported(
            "hole must be strictly inside box sides; tangencies and intersections are unsupported",
        ));
    }
    let mut s = make_box(b, t)?;
    let base = Point3::new(c.base.x, c.base.y, b.min.z);
    let cutter_surface = Surface::Cylinder {
        center: c.base,
        radius: c.radius,
        height: c.height,
    };
    let lower = cylinder_plane(&cutter_surface, &s.shell.faces[0].surface, t)?;
    let upper = cylinder_plane(&cutter_surface, &s.shell.faces[1].surface, t)?;
    let bottom = circle_edge(&mut s, lower);
    let top = circle_edge(&mut s, upper);
    for (fi, e) in [(0, bottom), (1, top)] {
        let w = cap_ring(&s, e, &s.shell.faces[fi].surface, false);
        s.shell.faces[fi].wires.push(w);
    }
    cylindrical_face(
        &mut s,
        bottom,
        top,
        CylinderSpec {
            base,
            radius: c.radius,
            height: b.size.z,
        },
        -1,
    );
    s.validate(t)?;
    Ok(s)
}
#[derive(Clone, Copy, Debug)]
pub enum Profile {
    Rectangle {
        origin: Point3,
        width: f64,
        depth: f64,
    },
    Disk {
        center: Point3,
        radius: f64,
    },
}
/// Exact positive-Z extrusion of the two supported closed analytic profiles.
pub fn extrude(profile: Profile, height: f64, t: Tolerance) -> Result<Solid> {
    match profile {
        Profile::Rectangle {
            origin,
            width,
            depth,
        } => make_box(
            BoxSpec {
                min: origin,
                size: Vec3::new(width, depth, height),
            },
            t,
        ),
        Profile::Disk { center, radius } => make_cylinder(
            CylinderSpec {
                base: center,
                radius,
                height,
            },
            t,
        ),
    }
}
