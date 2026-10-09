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
/// Restricted primitive difference with one strictly interior Z-axis through bore.
pub fn subtract_through_cylinder(b: BoxSpec, c: CylinderSpec, t: Tolerance) -> Result<Solid> {
    subtract_through_cylinders(b, &[c], t)
}
/// Exact multiple-bore primitive difference. All cutters must overhang both
/// caps, clear the sides, and be mutually disjoint by more than the tolerance.
/// Empty cutters return an unchanged box. Overlapping/near-touching cuts are
/// rejected rather than pretending independent holes form a general Boolean.
pub fn subtract_through_cylinders(
    b: BoxSpec,
    cutters: &[CylinderSpec],
    t: Tolerance,
) -> Result<Solid> {
    check_through_bores(b, cutters, t)?;
    let mut s = make_box(b, t)?;
    for &c in cutters {
        append_through_bore(&mut s, b, c, t)?;
    }
    s.validate(t)?;
    Ok(s)
}

/// Exact restricted difference: one Z-axis cylindrical blind bore entering +Z cap.
/// Tool base is the retained flat floor; tool top must strictly overhang the box.
pub fn subtract_blind_cylinder(b: BoxSpec, c: CylinderSpec, t: Tolerance) -> Result<Solid> {
    subtract_blind_cylinders(b, &[c], t)
}
/// Multiple disjoint +Z blind bores, with independent radii and floor heights.
/// Floors must be strictly inside the box; intersecting tools, side/bottom
/// breakthroughs, zero depth and unresolved thin floors are unsupported.
pub fn subtract_blind_cylinders(
    b: BoxSpec,
    cutters: &[CylinderSpec],
    t: Tolerance,
) -> Result<Solid> {
    check_blind_bores(b, cutters, t)?;
    let mut solid = make_box(b, t)?;
    for &c in cutters {
        append_blind_bore(&mut solid, b, c);
    }
    solid.validate(t)?;
    Ok(solid)
}
/// Demonstration: 80×60×24 mm box with a top-entry flat-bottomed circular bore.
pub fn blind_bore_demo_json(radius: f64, depth: f64) -> Result<String> {
    let solid = subtract_blind_cylinder(
        BoxSpec {
            min: Point3::new(-40., -30., -12.),
            size: Vec3::new(80., 60., 24.),
        },
        CylinderSpec {
            base: Point3::new(0., 0., 12. - depth),
            radius,
            height: depth + 4.,
        },
        Tolerance::default(),
    )?;
    solid.mesh_json(0.05, Tolerance::default())
}

/// Entry face of an axis-aligned box; bore axes point inward normal to this face.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum BoxFace {
    MaxZ = 0,
    MinZ = 1,
    MaxX = 2,
    MinX = 3,
    MaxY = 4,
    MinY = 5,
}
impl TryFrom<u32> for BoxFace {
    type Error = Error;
    fn try_from(value: u32) -> Result<Self> {
        match value {
            0 => Ok(Self::MaxZ),
            1 => Ok(Self::MinZ),
            2 => Ok(Self::MaxX),
            3 => Ok(Self::MinX),
            4 => Ok(Self::MaxY),
            5 => Ok(Self::MinY),
            _ => Err(Error::InvalidInput("box entry face must be in 0..5")),
        }
    }
}
/// Mouth center in world coordinates on the selected box face; depth is inward.
#[derive(Clone, Copy, Debug)]
pub struct FaceBlindBore {
    pub center: Point3,
    pub radius: f64,
    pub depth: f64,
}
/// Exact restricted primitive difference for disjoint flat-bottom bores entering
/// one selected box face. Different-face/intersecting tools are unsupported.
/// Centers within one linear tolerance of the entry plane are projected to it.
pub fn subtract_blind_bores_from_face(
    b: BoxSpec,
    face: BoxFace,
    bores: &[FaceBlindBore],
    t: Tolerance,
) -> Result<Solid> {
    check_box(b, t)?;
    if bores.len() > 256 {
        return Err(Error::Unsupported(
            "at most 256 independent blind bores are supported",
        ));
    }
    if bores.is_empty() {
        return make_box(b, t);
    }
    let max = b.min + b.size;
    let x = Vec3::new(1., 0., 0.);
    let y = Vec3::new(0., 1., 0.);
    let z = Vec3::new(0., 0., 1.);
    let (origin, axes, size) = match face {
        BoxFace::MaxZ => (b.min, [x, y, z], b.size),
        BoxFace::MinZ => (
            Point3::new(b.min.x, max.y, max.z),
            [x, y * -1., z * -1.],
            b.size,
        ),
        BoxFace::MaxX => (b.min, [y, z, x], Vec3::new(b.size.y, b.size.z, b.size.x)),
        BoxFace::MinX => (
            Point3::new(max.x, b.min.y, max.z),
            [y, z * -1., x * -1.],
            Vec3::new(b.size.y, b.size.z, b.size.x),
        ),
        BoxFace::MaxY => (b.min, [z, x, y], Vec3::new(b.size.z, b.size.x, b.size.y)),
        BoxFace::MinY => (
            Point3::new(max.x, max.y, b.min.z),
            [z, x * -1., y * -1.],
            Vec3::new(b.size.z, b.size.x, b.size.y),
        ),
    };
    let frame = Frame3::new(origin, axes, t)?;
    let mut tools = Vec::with_capacity(bores.len());
    for bore in bores {
        if !bore.center.finite() || !positive(bore.depth, t) {
            return Err(Error::InvalidInput(
                "face blind bore requires finite center and resolved positive depth",
            ));
        }
        let local = frame.local_point(bore.center);
        if !local.finite() || (local.z - size.z).abs() > t.linear {
            return Err(Error::InvalidInput(
                "blind bore mouth must lie on selected entry face",
            ));
        }
        tools.push(CylinderSpec {
            base: Point3::new(local.x, local.y, size.z - bore.depth),
            radius: bore.radius,
            height: bore.depth + size.z,
        });
    }
    subtract_blind_cylinders(
        BoxSpec {
            min: Point3::new(0., 0., 0.),
            size,
        },
        &tools,
        t,
    )?
    .transformed(frame, t)
}
/// Demo box with a centered blind hole on one of its six faces.
pub fn box_face_blind_bore_demo_solid(face: BoxFace, radius: f64, depth: f64) -> Result<Solid> {
    let b = BoxSpec {
        min: Point3::new(-40., -30., -20.),
        size: Vec3::new(80., 60., 40.),
    };
    let center = match face {
        BoxFace::MaxZ => Point3::new(0., 0., 20.),
        BoxFace::MinZ => Point3::new(0., 0., -20.),
        BoxFace::MaxX => Point3::new(40., 0., 0.),
        BoxFace::MinX => Point3::new(-40., 0., 0.),
        BoxFace::MaxY => Point3::new(0., 30., 0.),
        BoxFace::MinY => Point3::new(0., -30., 0.),
    };
    subtract_blind_bores_from_face(
        b,
        face,
        &[FaceBlindBore {
            center,
            radius,
            depth,
        }],
        Tolerance::default(),
    )
}
/// Native/WASM six-face blind-bore fixture with explicit face identifiers 0..5.
pub fn box_face_blind_bore_demo_json(face: u32, radius: f64, depth: f64) -> Result<String> {
    box_face_blind_bore_demo_solid(BoxFace::try_from(face)?, radius, depth)?
        .mesh_json(0.05, Tolerance::default())
}

/// Straight-line XY profile with simple, disjoint polygonal holes. Rings omit
/// a repeated closing point. Input winding is normalized without changing points.
#[derive(Clone, Debug)]
pub struct PolygonProfile {
    pub origin: Point3,
    pub outer: Vec<[f64; 2]>,
    pub holes: Vec<Vec<[f64; 2]>>,
}
/// Extrudes a simple planar polygon (convex or concave, optionally with holes)
/// along any vector with a nonzero Z span exceeding ten tolerances.
/// All surfaces and shared edges are exact planes and lines. Self-intersections,
/// touching/nested holes and redundant corners are explicit input errors.
pub fn extrude_polygon(profile: &PolygonProfile, direction: Vec3, t: Tolerance) -> Result<Solid> {
    use crate::planar::{polygon_area, validate_polygon, validate_region, PlanarLoop};
    Tolerance::new(t.linear)?;
    if !profile.origin.finite()
        || !direction.finite()
        || !positive(direction.z.abs(), t)
        || !(profile.origin + direction).finite()
    {
        return Err(Error::InvalidInput("extrusion requires finite origin/direction and nonzero Z span exceeding ten tolerances"));
    }
    if profile.outer.len() + profile.holes.iter().map(Vec::len).sum::<usize>() > 4096 {
        return Err(Error::Unsupported(
            "at most 4096 total profile corners are supported",
        ));
    }
    if profile.holes.len() > 256 {
        return Err(Error::Unsupported(
            "at most 256 polygon holes are supported",
        ));
    }
    let mut loops = Vec::new();
    for (i, input) in std::iter::once(&profile.outer)
        .chain(&profile.holes)
        .enumerate()
    {
        validate_polygon(input, t)?;
        let mut points = input.clone();
        if (polygon_area(&points) > 0.0) != (i == 0) {
            points.reverse();
        }
        loops.push(points);
    }
    let trims: Vec<_> = loops.iter().cloned().map(PlanarLoop::Polygon).collect();
    validate_region(&trims[0], &trims[1..], t)?;
    let count: usize = loops.iter().map(Vec::len).sum();
    let mut s = Solid {
        vertices: Vec::with_capacity(2 * count),
        edges: Vec::with_capacity(3 * count),
        shell: Shell { faces: Vec::new() },
    };
    for delta in [Vec3::new(0.0, 0.0, 0.0), direction] {
        for points in &loops {
            for p in points {
                let point = profile.origin + Vec3::new(p[0], p[1], 0.0) + delta;
                if !point.finite() {
                    return Err(Error::InvalidInput("extrusion exceeds finite coordinates"));
                }
                s.vertices.push(Vertex { point });
            }
        }
    }
    let mut rings = Vec::new();
    let mut offset = 0;
    for points in &loops {
        rings.push((offset..offset + points.len()).collect::<Vec<_>>());
        offset += points.len();
    }
    let mut add_edge = |a: usize, b: usize| {
        s.edges.push(Edge {
            vertices: [a, b],
            curve: Curve::Line {
                a: s.vertices[a].point,
                b: s.vertices[b].point,
            },
        });
    };
    for ring in &rings {
        for i in 0..ring.len() {
            let a = ring[i];
            let b = ring[(i + 1) % ring.len()];
            add_edge(a, b);
            add_edge(a + count, b + count);
            add_edge(a, a + count);
        }
    }
    let sign = if direction.z > 0.0 { 1 } else { -1 };
    for (offset, orientation) in [(0, -sign), (count, sign)] {
        let surface = Surface::Plane {
            origin: profile.origin
                + if offset == 0 {
                    Vec3::new(0.0, 0.0, 0.0)
                } else {
                    direction
                },
            u: Vec3::new(1.0, 0.0, 0.0),
            v: Vec3::new(0.0, 1.0, 0.0),
        };
        let wires = rings
            .iter()
            .map(|ring| {
                plane_wire(
                    &s,
                    &ring.iter().map(|i| i + offset).collect::<Vec<_>>(),
                    &surface,
                )
            })
            .collect();
        s.shell.faces.push(Face {
            surface,
            wires,
            orientation,
        });
    }
    for ring in &rings {
        for i in 0..ring.len() {
            let a = ring[i];
            let b = ring[(i + 1) % ring.len()];
            let origin = s.vertices[a].point;
            let u = (s.vertices[b].point - origin).normalized()?;
            let v = (direction - u * u.dot(direction)).normalized()?;
            let surface = Surface::Plane { origin, u, v };
            let wire = plane_wire(&s, &[a, b, b + count, a + count], &surface);
            s.shell.faces.push(Face {
                surface,
                wires: vec![wire],
                orientation: sign,
            });
        }
    }
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

/// Exact coaxial hollow Z cylinder, including annular plane caps.
#[derive(Clone, Copy, Debug)]
pub struct TubeSpec {
    pub base: Point3,
    pub outer_radius: f64,
    pub inner_radius: f64,
    pub height: f64,
}
pub fn make_tube(spec: TubeSpec, t: Tolerance) -> Result<Solid> {
    let outer = CylinderSpec {
        base: spec.base,
        radius: spec.outer_radius,
        height: spec.height,
    };
    let inner = CylinderSpec {
        base: spec.base,
        radius: spec.inner_radius,
        height: spec.height,
    };
    check_cylinder(outer, t)?;
    check_cylinder(inner, t)?;
    if !positive(spec.outer_radius - spec.inner_radius, t) {
        return Err(Error::InvalidInput(
            "tube wall must exceed ten linear tolerances",
        ));
    }
    let mut s = make_cylinder(outer, t)?;
    let bottom = ring(&mut s, spec.base, spec.inner_radius);
    let top = ring(
        &mut s,
        spec.base + Vec3::new(0.0, 0.0, spec.height),
        spec.inner_radius,
    );
    for (fi, e) in [(0, bottom), (1, top)] {
        let wire = cap_ring(&s, e, &s.shell.faces[fi].surface, false);
        s.shell.faces[fi].wires.push(wire);
    }
    cylindrical_face(&mut s, bottom, top, inner, -1);
    s.validate(t)?;
    Ok(s)
}

/// Extrudes a polygon specified in frame-local coordinates along a world-space
/// direction. `profile.origin` is local to `frame`; ring coordinates are local
/// XY offsets. The normal span must exceed ten linear tolerances. Construction
/// and validation use exact planes/lines, then the checked rigid placement.
pub fn extrude_polygon_in_frame(
    profile: &PolygonProfile,
    world_direction: Vec3,
    frame: Frame3,
    tol: Tolerance,
) -> Result<Solid> {
    extrude_polygon(profile, frame.local_vector(world_direction), tol)?.transformed(frame, tol)
}

fn append_through_bore(s: &mut Solid, b: BoxSpec, c: CylinderSpec, t: Tolerance) -> Result<()> {
    let base = Point3::new(c.base.x, c.base.y, b.min.z);
    let cutter_surface = Surface::Cylinder {
        center: c.base,
        radius: c.radius,
        height: c.height,
    };
    let lower = cylinder_plane(&cutter_surface, &s.shell.faces[0].surface, t)?;
    let upper = cylinder_plane(&cutter_surface, &s.shell.faces[1].surface, t)?;
    let bottom = circle_edge(s, lower);
    let top = circle_edge(s, upper);
    for (fi, e) in [(0, bottom), (1, top)] {
        let w = cap_ring(s, e, &s.shell.faces[fi].surface, false);
        s.shell.faces[fi].wires.push(w);
    }
    cylindrical_face(
        s,
        bottom,
        top,
        CylinderSpec {
            base,
            radius: c.radius,
            height: b.size.z,
        },
        -1,
    );
    Ok(())
}

fn append_blind_bore(solid: &mut Solid, b: BoxSpec, c: CylinderSpec) {
    let max = b.min + b.size;
    let bottom = ring(solid, c.base, c.radius);
    let top = ring(solid, Point3::new(c.base.x, c.base.y, max.z), c.radius);
    let wire = cap_ring(solid, top, &solid.shell.faces[1].surface, false);
    solid.shell.faces[1].wires.push(wire);
    cylindrical_face(
        solid,
        bottom,
        top,
        CylinderSpec {
            base: c.base,
            radius: c.radius,
            height: max.z - c.base.z,
        },
        -1,
    );
    let surface = Surface::Plane {
        origin: c.base,
        u: Vec3::new(1., 0., 0.),
        v: Vec3::new(0., 1., 0.),
    };
    let wire = cap_ring(solid, bottom, &surface, true);
    solid.shell.faces.push(Face {
        surface,
        orientation: 1,
        wires: vec![wire],
    });
}

/// Mixed top-entry blind and through bores with disjoint XY footprints.
/// `None` means through; `Some(depth)` is measured inward from the +Z cap.
#[derive(Clone, Copy, Debug)]
pub struct BoxBore {
    pub center: [f64; 2],
    pub radius: f64,
    pub depth: Option<f64>,
}
/// Exact box difference; at most 256 bores, all clearances exceed ten linear tolerances.
pub fn subtract_box_bores(b: BoxSpec, bores: &[BoxBore], t: Tolerance) -> Result<Solid> {
    let tools = checked_box_bore_tools(b, bores, t)?;
    let mut solid = make_box(b, t)?;
    for (bore, c) in bores.iter().zip(tools) {
        apply_checked_prism_bore(&mut solid, b, *bore, c, t)?;
    }
    solid.validate(t)?;
    Ok(solid)
}

fn check_through_bores(b: BoxSpec, cutters: &[CylinderSpec], t: Tolerance) -> Result<()> {
    check_box(b, t)?;
    if cutters.len() > 256 {
        return Err(Error::Unsupported(
            "at most 256 independent bores are supported",
        ));
    }
    let max = b.min + b.size;
    for (i, &c) in cutters.iter().enumerate() {
        check_cylinder(c, t)?;
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
            return Err(Error::Unsupported("hole must be strictly inside box sides; tangencies and intersections are unsupported"));
        }
        for prev in &cutters[..i] {
            if (c.base.x - prev.base.x).hypot(c.base.y - prev.base.y)
                <= c.radius + prev.radius + t.linear
            {
                return Err(Error::Unsupported(
                    "bores overlap, nest, touch or nearly touch",
                ));
            }
        }
    }
    Ok(())
}

fn check_blind_bores(b: BoxSpec, cutters: &[CylinderSpec], t: Tolerance) -> Result<()> {
    check_box(b, t)?;
    if cutters.len() > 256 {
        return Err(Error::Unsupported(
            "at most 256 independent blind bores are supported",
        ));
    }
    let max = b.min + b.size;
    for (i, &c) in cutters.iter().enumerate() {
        check_cylinder(c, t)?;
        if c.base.z - b.min.z <= 10. * t.linear
            || max.z - c.base.z <= 10. * t.linear
            || c.base.z + c.height - max.z <= 10. * t.linear
        {
            return Err(Error::Unsupported(
                "blind tool requires an interior resolved floor and strict top overhang",
            ));
        }
        if c.base.x - c.radius - b.min.x <= 10. * t.linear
            || c.base.y - c.radius - b.min.y <= 10. * t.linear
            || max.x - c.base.x - c.radius <= 10. * t.linear
            || max.y - c.base.y - c.radius <= 10. * t.linear
        {
            return Err(Error::Unsupported(
                "blind bore must stay strictly inside box sides",
            ));
        }
        for previous in &cutters[..i] {
            if (c.base.x - previous.base.x).hypot(c.base.y - previous.base.y)
                - c.radius
                - previous.radius
                <= 10. * t.linear
            {
                return Err(Error::Unsupported(
                    "blind bores overlap, nest, touch or nearly touch",
                ));
            }
        }
    }
    Ok(())
}

// Internal: validate all tools before touching a cached prefix. Public callers
// cannot append an unchecked tool to an arbitrary or differently trimmed solid.
pub(crate) fn checked_box_bore_tools(
    b: BoxSpec,
    bores: &[BoxBore],
    t: Tolerance,
) -> Result<Vec<CylinderSpec>> {
    check_box(b, t)?;
    if bores.len() > 256 {
        return Err(Error::Unsupported("at most 256 box bores are supported"));
    }
    let mut through = Vec::new();
    let mut blind = Vec::new();
    let mut tools = Vec::new();
    for (i, bore) in bores.iter().enumerate() {
        let depth = bore.depth.unwrap_or(b.size.z);
        if !depth.is_finite() || depth <= 10. * t.linear {
            return Err(Error::InvalidInput(
                "bore depth must be finite and resolved",
            ));
        }
        let c = CylinderSpec {
            base: Point3::new(
                bore.center[0],
                bore.center[1],
                if bore.depth.is_some() {
                    b.min.z + b.size.z - depth
                } else {
                    b.min.z - b.size.z
                },
            ),
            radius: bore.radius,
            height: if bore.depth.is_some() {
                depth + b.size.z
            } else {
                3. * b.size.z
            },
        };
        check_cylinder(c, t)?;
        let max = b.min + b.size;
        let clearance = (c.base.x - b.min.x)
            .min(max.x - c.base.x)
            .min(c.base.y - b.min.y)
            .min(max.y - c.base.y)
            - c.radius;
        if !clearance.is_finite() || clearance <= 10. * t.linear {
            return Err(Error::Unsupported(
                "box bore requires resolved side clearance",
            ));
        }
        for previous in &bores[..i] {
            let gap = (bore.center[0] - previous.center[0])
                .hypot(bore.center[1] - previous.center[1])
                - bore.radius
                - previous.radius;
            if !gap.is_finite() || gap <= 10. * t.linear {
                return Err(Error::Unsupported(
                    "box bores overlap, nest, touch or nearly touch",
                ));
            }
        }
        if bore.depth.is_some() {
            blind.push(c);
        } else {
            through.push(c);
        }
        tools.push(c);
    }
    // Reuse the same checked domains as the single-mode public operations.
    check_through_bores(b, &through, t)?;
    check_blind_bores(b, &blind, t)?;
    Ok(tools)
}
// Internal: stock is only the prism envelope/Z span. The owning cap planes
// and polygon trims come from the already-validated stock B-rep.
pub(crate) fn apply_checked_prism_bore(
    solid: &mut Solid,
    b: BoxSpec,
    bore: BoxBore,
    tool: CylinderSpec,
    t: Tolerance,
) -> Result<()> {
    if bore.depth.is_some() {
        append_blind_bore(solid, b, tool);
    } else {
        append_through_bore(solid, b, tool, t)?;
    }
    Ok(())
}

/// Exact +Z prism of a simple world-XY polygon, centered about Z=0,
/// with disjoint circular through/blind bores. No profile holes or skew axes.
/// Up to 256 corners and 256 tools; all clearances exceed ten linear tolerances.
pub fn subtract_polygon_prism_bores(
    outer: &[[f64; 2]],
    height: f64,
    bores: &[BoxBore],
    t: Tolerance,
) -> Result<Solid> {
    subtract_polygon_region_prism_bores(outer, &[], height, bores, t)
}
/// Exact polygon-region prism with polygon through openings and optional
/// independent circular cuts. At most 64 profile holes and 256 total corners.
pub fn subtract_polygon_region_prism_bores(
    outer: &[[f64; 2]],
    holes: &[Vec<[f64; 2]>],
    height: f64,
    bores: &[BoxBore],
    t: Tolerance,
) -> Result<Solid> {
    if bores.len() > 256 {
        return Err(Error::Unsupported(
            "at most 256 polygon prism bores are supported",
        ));
    }
    let stock = checked_polygon_region_prism_stock(outer, holes, height, t)?;
    for bore in bores {
        if polygon_region_bore_clearance(outer, holes, bore.center, bore.radius)?.0
            <= 10. * t.linear
        {
            return Err(Error::Unsupported(
                "bore footprint touches or crosses the polygon boundary",
            ));
        }
    }
    let tools = checked_box_bore_tools(stock, bores, t)?;
    let mut solid = extrude_polygon(
        &PolygonProfile {
            origin: Point3::new(0., 0., -height / 2.),
            outer: outer.to_vec(),
            holes: holes.to_vec(),
        },
        Vec3::new(0., 0., height),
        t,
    )?;
    for (&bore, tool) in bores.iter().zip(tools) {
        apply_checked_prism_bore(&mut solid, stock, bore, tool, t)?;
    }
    solid.validate(t)?;
    Ok(solid)
}
pub(crate) fn checked_polygon_region_prism_stock(
    outer: &[[f64; 2]],
    holes: &[Vec<[f64; 2]>],
    height: f64,
    t: Tolerance,
) -> Result<BoxSpec> {
    Tolerance::new(t.linear)?;
    if holes.len() > 64 || outer.len() + holes.iter().map(Vec::len).sum::<usize>() > 256 {
        return Err(Error::Unsupported(
            "polygon prism bores support at most 64 profile holes and 256 total corners",
        ));
    }
    if !height.is_finite() || height <= 10. * t.linear {
        return Err(Error::InvalidInput(
            "polygon prism requires a finite positive resolved height",
        ));
    }
    crate::planar::validate_polygon(outer, t)?;
    for hole in holes {
        crate::planar::validate_polygon(hole, t)?;
    }
    let hole_loops: Vec<_> = holes
        .iter()
        .cloned()
        .map(crate::planar::PlanarLoop::Polygon)
        .collect();
    crate::planar::validate_region(
        &crate::planar::PlanarLoop::Polygon(outer.to_vec()),
        &hole_loops,
        Tolerance::new(10. * t.linear)?,
    )?;
    let mut min = [f64::INFINITY; 2];
    let mut max = [f64::NEG_INFINITY; 2];
    for point in outer {
        for axis in 0..2 {
            min[axis] = min[axis].min(point[axis]);
            max[axis] = max[axis].max(point[axis]);
        }
    }
    let stock = BoxSpec {
        min: Point3::new(min[0], min[1], -height / 2.),
        size: Vec3::new(max[0] - min[0], max[1] - min[1], height),
    };
    check_box(stock, t)?;
    Ok(stock)
}
// The connected circular footprint cannot cross a validated simple boundary
// when its center is inside and every boundary segment is farther than radius.
pub(crate) fn polygon_bore_clearance(
    outer: &[[f64; 2]],
    center: [f64; 2],
    radius: f64,
) -> Result<f64> {
    loop_bore_clearance(outer, center, radius, true)
}
fn loop_bore_clearance(
    outer: &[[f64; 2]],
    center: [f64; 2],
    radius: f64,
    inside: bool,
) -> Result<f64> {
    if !radius.is_finite() || radius <= 0. {
        return Err(Error::InvalidInput(
            "polygon bore radius must be finite and positive",
        ));
    }
    let location = locate_point_in_polygon(center, outer)?;
    let mut distance = f64::INFINITY;
    for i in 0..outer.len() {
        distance = distance.min(crate::planar::segment_distance(
            center,
            outer[i],
            outer[(i + 1) % outer.len()],
        )?);
    }
    let gap = if (location == PointLocation::Inside) == inside {
        distance - radius
    } else {
        -distance - radius
    };
    if !gap.is_finite() {
        return Err(Error::Unsupported(
            "polygon bore clearance cannot be resolved with finite arithmetic",
        ));
    }
    Ok(gap)
}

pub(crate) fn polygon_region_bore_clearance(
    outer: &[[f64; 2]],
    holes: &[Vec<[f64; 2]>],
    center: [f64; 2],
    radius: f64,
) -> Result<(f64, Option<usize>)> {
    let mut gap = polygon_bore_clearance(outer, center, radius)?;
    let mut boundary = None;
    for (i, hole) in holes.iter().enumerate() {
        let candidate = loop_bore_clearance(hole, center, radius, false)?;
        if candidate < gap {
            gap = candidate;
            boundary = Some(i);
        }
    }
    Ok((gap, boundary))
}

/// Exact skew polygon stock minus disjoint world-Z through/blind cylinders.
/// The lower profile is at Z=-height/2; its upper copy is translated by offset.
/// The swept footprint over each tool depth must clear all profile boundaries
/// by ten linear tolerances. Blind depths are measured from the +Z cap; a
/// resolved floor is required. Side-crossing and overlapping tools are rejected.
pub fn subtract_skew_polygon_region_prism_bores(
    outer: &[[f64; 2]],
    holes: &[Vec<[f64; 2]>],
    height: f64,
    offset: [f64; 2],
    bores: &[BoxBore],
    t: Tolerance,
) -> Result<Solid> {
    let stock = checked_polygon_region_prism_stock(outer, holes, height, t)?;
    if offset.iter().any(|v| !v.is_finite()) || bores.len() > 256 {
        return Err(Error::InvalidInput(
            "invalid skew offset or excessive tool count",
        ));
    }
    for bore in bores {
        if swept_polygon_region_bore_clearance(
            outer,
            holes,
            bore.center,
            bore.radius,
            offset,
            height,
            [1. - bore.depth.unwrap_or(height) / height, 1.],
        )?
        .0 <= 10. * t.linear
        {
            return Err(Error::Unsupported(
                "tool crosses or nearly touches a swept stock boundary",
            ));
        }
    }
    let tools = checked_skew_prism_bore_tools(stock, offset, bores, t)?;
    let mut solid = extrude_polygon(
        &PolygonProfile {
            origin: Point3::new(0., 0., -height / 2.),
            outer: outer.to_vec(),
            holes: holes.to_vec(),
        },
        Vec3::new(offset[0], offset[1], height),
        t,
    )?;
    for (&bore, tool) in bores.iter().zip(tools) {
        apply_checked_prism_bore(&mut solid, stock, bore, tool, t)?;
    }
    solid.validate(t)?;
    Ok(solid)
}
// In moving profile coordinates a fixed world-Z tool traces the center
// segment over its actual depth, ending at c-offset on the upper cap.
// Segment/boundary separation certifies every intermediate cross section,
// including concave re-entry and an opening crossed between two valid caps.
// Divide by a slope bound to obtain a conservative physical wall clearance.
pub(crate) fn swept_polygon_region_bore_clearance(
    outer: &[[f64; 2]],
    holes: &[Vec<[f64; 2]>],
    center: [f64; 2],
    radius: f64,
    offset: [f64; 2],
    height: f64,
    interval: [f64; 2],
) -> Result<(f64, Option<usize>)> {
    if interval.iter().any(|v| !v.is_finite())
        || interval[0] < 0.
        || interval[1] > 1.
        || interval[0] >= interval[1]
    {
        return Err(Error::InvalidInput(
            "invalid tool depth for swept clearance",
        ));
    }
    if offset == [0., 0.] {
        return polygon_region_bore_clearance(outer, holes, center, radius);
    }
    let start_fraction = interval[0];
    let start = [
        center[0] - offset[0] * start_fraction,
        center[1] - offset[1] * start_fraction,
    ];
    let end = [
        center[0] - offset[0] * interval[1],
        center[1] - offset[1] * interval[1],
    ];
    let factor = 1_f64.hypot(offset[0].hypot(offset[1]) / height);
    if start.iter().chain(&end).any(|v| !v.is_finite()) || !factor.is_finite() || height <= 0. {
        return Err(Error::Unsupported("unresolved swept tool coordinates"));
    }
    let mut result = polygon_region_bore_clearance(outer, holes, start, radius)?;
    let endpoint = polygon_region_bore_clearance(outer, holes, end, radius)?;
    if endpoint.0 < result.0 {
        result = endpoint;
    }
    for (index, boundary) in std::iter::once(outer)
        .chain(holes.iter().map(Vec::as_slice))
        .enumerate()
    {
        for i in 0..boundary.len() {
            let gap = crate::planar::segments_distance(
                start,
                end,
                boundary[i],
                boundary[(i + 1) % boundary.len()],
            )? - radius;
            if gap < result.0 {
                result = (gap, index.checked_sub(1));
            }
        }
    }
    result.0 /= factor;
    Ok(result)
}

// Tool validation needs the envelope of both stock caps, not only the lower
// footprint: a shallow blind hole can lie wholly outside the lower footprint.
pub(crate) fn checked_skew_prism_bore_tools(
    stock: BoxSpec,
    offset: [f64; 2],
    bores: &[BoxBore],
    t: Tolerance,
) -> Result<Vec<CylinderSpec>> {
    let envelope = BoxSpec {
        min: Point3::new(
            stock.min.x + offset[0].min(0.),
            stock.min.y + offset[1].min(0.),
            stock.min.z,
        ),
        size: Vec3::new(
            stock.size.x + offset[0].abs(),
            stock.size.y + offset[1].abs(),
            stock.size.z,
        ),
    };
    checked_box_bore_tools(envelope, bores, t)
}

// Checked workflow calls only: world-Z blind cut entering the lower cap.
pub(crate) fn append_bottom_blind_bore(solid: &mut Solid, stock: BoxSpec, bore: BoxBore) {
    let depth = bore.depth.expect("validated blind depth");
    let base = Point3::new(bore.center[0], bore.center[1], stock.min.z);
    let bottom = ring(solid, base, bore.radius);
    let floor_center = base + Vec3::new(0., 0., depth);
    let top = ring(solid, floor_center, bore.radius);
    let wire = cap_ring(solid, bottom, &solid.shell.faces[0].surface, false);
    solid.shell.faces[0].wires.push(wire);
    cylindrical_face(
        solid,
        bottom,
        top,
        CylinderSpec {
            base,
            radius: bore.radius,
            height: depth,
        },
        -1,
    );
    let surface = Surface::Plane {
        origin: floor_center,
        u: Vec3::new(1., 0., 0.),
        v: Vec3::new(0., 1., 0.),
    };
    let wire = cap_ring(solid, top, &surface, true);
    solid.shell.faces.push(Face {
        surface,
        orientation: -1,
        wires: vec![wire],
    });
}
