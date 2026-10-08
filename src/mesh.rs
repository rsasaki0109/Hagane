use crate::*;
use std::f64::consts::TAU;
#[derive(Clone, Debug, Default)]
pub struct Mesh {
    pub positions: Vec<Point3>,
    pub normals: Vec<Vec3>,
    pub triangles: Vec<[usize; 3]>,
    pub face_ids: Vec<usize>,
}
impl Mesh {
    pub fn signed_volume(&self) -> f64 {
        self.triangles
            .iter()
            .map(|t| {
                self.positions[t[0]].dot(self.positions[t[1]].cross(self.positions[t[2]])) / 6.0
            })
            .sum()
    }
    fn triangle(&mut self, mut p: [Point3; 3], mut n: [Vec3; 3], face: usize, orientation: i8) {
        if orientation < 0 {
            p.swap(1, 2);
            n.swap(1, 2);
        }
        let start = self.positions.len();
        self.positions.extend(p);
        self.normals.extend(n.map(|x| x * orientation as f64));
        self.triangles.push([start, start + 1, start + 2]);
        self.face_ids.push(face);
    }
}
/// A conservative chordal segment count. Error is the circle sagitta, in length units.
pub fn circle_segments(radius: f64, chord_error: f64) -> Result<usize> {
    if !radius.is_finite() || radius <= 0.0 || !chord_error.is_finite() || chord_error <= 0.0 {
        return Err(Error::InvalidInput(
            "positive finite radius and chord error required",
        ));
    }
    let angle = 4.0 * (chord_error / (2.0 * radius)).min(0.5).sqrt().asin();
    let count = (TAU / angle).ceil().max(12.0);
    if !count.is_finite() || count > 65536.0 {
        return Err(Error::Tessellation(
            "requested accuracy exceeds 65536 segments",
        ));
    }
    Ok(count as usize)
}
impl Solid {
    /// Meshes the exact supported B-rep surfaces and trims; never performs mesh CSG.
    pub fn tessellate(&self, chord_error: f64, tol: Tolerance) -> Result<Mesh> {
        self.validate(tol)?;
        if !chord_error.is_finite() || chord_error <= 0.0 {
            return Err(Error::InvalidInput("positive finite chord error required"));
        }
        let mut mesh = Mesh::default();
        for (fi, f) in self.shell.faces.iter().enumerate() {
            match f.surface {
                Surface::Plane { .. } => {
                    let mut coords = Vec::new();
                    let mut holes = Vec::new();
                    for (wi, w) in f.wires.iter().enumerate() {
                        if wi > 0 {
                            holes.push(coords.len() / 2);
                        }
                        for c in &w.coedges {
                            let edge = &self.edges[c.edge];
                            let count = match edge.curve {
                                Curve::Circle { radius, .. }
                                | Curve::FramedCircle { radius, .. } => {
                                    circle_segments(radius, chord_error)?
                                }
                                Curve::Line { .. } => 1,
                            };
                            let range = edge.curve.range();
                            for k in 0..count {
                                let frac = k as f64 / count as f64;
                                let t = if c.forward {
                                    range[0] + (range[1] - range[0]) * frac
                                } else {
                                    range[1] - (range[1] - range[0]) * frac
                                };
                                coords.extend(c.pcurve.evaluate(t));
                            }
                        }
                    }
                    let triangles = earcutr::earcut(&coords, &holes, 2)
                        .map_err(|_| Error::Tessellation("planar trim triangulation failed"))?;
                    if triangles.is_empty() {
                        return Err(Error::Tessellation("no planar triangles"));
                    }
                    for tri in triangles.as_chunks::<3>().0 {
                        let p = [tri[0], tri[1], tri[2]]
                            .map(|i| f.surface.evaluate(coords[i * 2], coords[i * 2 + 1]));
                        let normal = f.surface.normal(0.0);
                        let mut p = p;
                        if (p[1] - p[0]).cross(p[2] - p[0]).dot(normal) < 0.0 {
                            p.swap(1, 2);
                        }
                        mesh.triangle(p, [normal; 3], fi, f.orientation);
                    }
                }
                Surface::Cylinder { radius, height, .. }
                | Surface::FramedCylinder { radius, height, .. } => {
                    // A complete periodic cylindrical rectangle is the currently supported trim.
                    if f.wires.len() != 1 || f.wires[0].coedges.len() != 4 {
                        return Err(Error::Unsupported(
                            "partial cylindrical trims cannot be tessellated",
                        ));
                    }
                    let n = circle_segments(radius, chord_error)?;
                    for i in 0..n {
                        let a = TAU * i as f64 / n as f64;
                        let b = TAU * (i + 1) as f64 / n as f64;
                        let p = [
                            f.surface.evaluate(a, 0.0),
                            f.surface.evaluate(b, 0.0),
                            f.surface.evaluate(b, height),
                            f.surface.evaluate(a, height),
                        ];
                        let na = f.surface.normal(a);
                        let nb = f.surface.normal(b);
                        mesh.triangle([p[0], p[1], p[2]], [na, nb, nb], fi, f.orientation);
                        mesh.triangle([p[0], p[2], p[3]], [na, nb, na], fi, f.orientation);
                    }
                }
            }
        }
        Ok(mesh)
    }
}
/// Browser/native single-bore fixture. Kept as a compatible shorthand.
pub fn demo_json(radius: f64, chord_error: f64) -> Result<String> {
    demo_preset_json(0, radius, chord_error)
}
/// Select actual kernel operations: 0 single bore, 1 four bores, 2 concave
/// polygon extrusion with a polygon hole, 3 coaxial tube, 4 rigidly placed
/// four-bore part. Unknown IDs fail.
pub fn demo_preset_json(preset: u32, radius: f64, chord_error: f64) -> Result<String> {
    let tol = Tolerance::default();
    let b = BoxSpec {
        min: Point3::new(-40.0, -30.0, -12.0),
        size: Vec3::new(80.0, 60.0, 24.0),
    };
    let tool = |x, y, radius| CylinderSpec {
        base: Point3::new(x, y, -20.0),
        radius,
        height: 40.0,
    };
    let s = match preset {
        0 => subtract_through_cylinder(b, tool(0.0, 0.0, radius), tol)?,
        // Half the radius slider: four independent bores at fixed centers.
        1 => subtract_through_cylinders(
            b,
            &[
                tool(-20.0, -14.0, radius * 0.5),
                tool(20.0, -14.0, radius * 0.5),
                tool(20.0, 14.0, radius * 0.5),
                tool(-20.0, 14.0, radius * 0.5),
            ],
            tol,
        )?,
        2 => extrude_polygon(
            &PolygonProfile {
                origin: Point3::new(0.0, 0.0, -12.0),
                outer: vec![
                    [-40.0, -30.0],
                    [40.0, -30.0],
                    [40.0, -5.0],
                    [-5.0, -5.0],
                    [-5.0, 30.0],
                    [-40.0, 30.0],
                ],
                holes: vec![vec![
                    [-32.0, -16.0],
                    [-20.0, -16.0],
                    [-20.0, 12.0],
                    [-32.0, 12.0],
                ]],
            },
            Vec3::new(6.0, 3.0, 24.0),
            tol,
        )?,
        4 => subtract_through_cylinders(
            b,
            &[
                tool(-20.0, -14.0, radius * 0.5),
                tool(20.0, -14.0, radius * 0.5),
                tool(20.0, 14.0, radius * 0.5),
                tool(-20.0, 14.0, radius * 0.5),
            ],
            tol,
        )?
        .transformed(
            Transform::translation(Vec3::new(8.0, -4.0, 6.0))?
                .compose(Transform::rotation(Vec3::new(1.0, 2.0, 0.5), 0.8)?)?,
            tol,
        )?,
        3 => make_tube(
            TubeSpec {
                base: Point3::new(0.0, 0.0, -12.0),
                outer_radius: 30.0,
                inner_radius: radius,
                height: 24.0,
            },
            tol,
        )?,
        _ => return Err(Error::Unsupported("unknown demo preset")),
    };
    s.mesh_json(chord_error, tol)
}
impl Solid {
    /// Display-only JSON transport for the WebGL demo. Not a B-rep interchange format.
    pub fn mesh_json(&self, chord_error: f64, tol: Tolerance) -> Result<String> {
        let m = self.tessellate(chord_error, tol)?;
        let mut out = format!(
            "{{\"volume\":{},\"faces\":{},\"edges\":{},\"positions\":[",
            self.volume()?,
            self.shell.faces.len(),
            self.edges.len()
        );
        for (i, p) in m.positions.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str(&format!("{},{},{}", p.x, p.y, p.z));
        }
        out.push_str("],\"normals\":[");
        for (i, p) in m.normals.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str(&format!("{},{},{}", p.x, p.y, p.z));
        }
        out.push_str("]}");
        Ok(out)
    }
}
