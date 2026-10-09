//! Exact, narrow AP214 recognition of unplaced full-domain polynomial graph bodies.
use crate::step_read::{boolean, list, number, reference, Database, Parser, Value};
use crate::*;
use std::collections::{BTreeMap, BTreeSet};
fn unsupported() -> Error {
    Error::Unsupported(
        "STEP NURBS graph import requires an exact unplaced full-domain canonical graph B-rep",
    )
}
fn integer(value: &Value, maximum: usize) -> Result<usize> {
    let v = number(value)?;
    if v < 1. || v > maximum as f64 || v.fract() != 0. {
        return Err(unsupported());
    }
    Ok(v as usize)
}
fn knots(mults: &Value, values: &Value, degree: usize, count: usize) -> Result<Vec<f64>> {
    let m = list(mults)?;
    let k = list(values)?;
    if m.len() != k.len() || m.len() > 64 {
        return Err(unsupported());
    }
    let mut result = Vec::new();
    let mut last = None;
    for (m, k) in m.iter().zip(k) {
        let value = number(k)?;
        let times = integer(m, degree + 1)?;
        if !value.is_finite()
            || last.is_some_and(|last| value <= last)
            || result.len() + times > count + degree + 1
        {
            return Err(unsupported());
        }
        last = Some(value);
        result.extend(std::iter::repeat_n(value, times));
    }
    if result.len() != count + degree + 1 {
        return Err(unsupported());
    }
    Ok(result)
}
fn curve(db: &Database, id: u32, scale: f64) -> Result<NurbsCurve> {
    let a = db.simple(id, "B_SPLINE_CURVE_WITH_KNOTS", 9)?;
    let degree = integer(&a[1], 16)?;
    let points = list(&a[2])?;
    if points.len() > 32
        || a[3] != Value::Enum("UNSPECIFIED".into())
        || boolean(&a[4])?
        || boolean(&a[5])?
        || a[8] != Value::Enum("UNSPECIFIED".into())
    {
        return Err(unsupported());
    }
    let points = points
        .iter()
        .map(|p| db.point(reference(p)?, scale))
        .collect::<Result<Vec<_>>>()?;
    let weights = vec![1.; points.len()];
    let knots = knots(&a[6], &a[7], degree, points.len())?;
    NurbsCurve::new(degree, knots, points, weights)
}
fn surface(db: &Database, id: u32, scale: f64) -> Result<NurbsSurface> {
    let a = db.simple(id, "B_SPLINE_SURFACE_WITH_KNOTS", 13)?;
    let degrees = [integer(&a[1], 16)?, integer(&a[2], 16)?];
    let rows = list(&a[3])?;
    if rows.is_empty()
        || rows.len() > 32
        || a[4] != Value::Enum("UNSPECIFIED".into())
        || (5..8).any(|i| boolean(&a[i]).unwrap_or(true))
        || a[12] != Value::Enum("UNSPECIFIED".into())
    {
        return Err(unsupported());
    }
    let nv = list(&rows[0])?.len();
    if nv == 0 || nv > 32 {
        return Err(unsupported());
    }
    let mut points = Vec::new();
    for row in rows {
        let row = list(row)?;
        if row.len() != nv {
            return Err(unsupported());
        }
        for p in row {
            points.push(db.point(reference(p)?, scale)?);
        }
    }
    let counts = [rows.len(), nv];
    let weights = vec![1.; points.len()];
    let knot_vectors = [
        knots(&a[8], &a[10], degrees[0], counts[0])?,
        knots(&a[9], &a[11], degrees[1], counts[1])?,
    ];
    NurbsSurface::new(degrees, knot_vectors, counts, points, weights)
}
fn coordinates2(db: &Database, id: u32, kind: &str) -> Result<[f64; 2]> {
    let a = db.simple(id, kind, 2)?;
    let values = list(&a[1])?;
    if values.len() != 2 {
        return Err(unsupported());
    }
    let result = [number(&values[0])?, number(&values[1])?];
    if result.iter().any(|x| !x.is_finite()) {
        return Err(unsupported());
    }
    Ok(result)
}
fn pcurve(db: &Database, id: u32) -> Result<(u32, PCurve)> {
    let a = db.simple(id, "PCURVE", 3)?;
    let surface = reference(&a[1])?;
    let representation = db.simple(reference(&a[2])?, "DEFINITIONAL_REPRESENTATION", 3)?;
    let items = list(&representation[1])?;
    if items.len() != 1 {
        return Err(unsupported());
    }
    let context = reference(&representation[2])?;
    if db.component(context, "GEOMETRIC_REPRESENTATION_CONTEXT")? != [Value::Number(2.)]
        || db.component(context, "REPRESENTATION_CONTEXT")?
            != [Value::String(String::new()), Value::String(String::new())]
    {
        return Err(unsupported());
    }
    let line = db.simple(reference(&items[0])?, "LINE", 3)?;
    let origin = coordinates2(db, reference(&line[1])?, "CARTESIAN_POINT")?;
    let vector = db.simple(reference(&line[2])?, "VECTOR", 3)?;
    let direction = coordinates2(db, reference(&vector[1])?, "DIRECTION")?;
    let length = number(&vector[2])?;
    let magnitude = direction[0].hypot(direction[1]);
    // Scoped graph PCurves have exact unit coordinate-axis directions. Never
    // normalize an approximate direction or silently change its parameter.
    if magnitude != 1. || !length.is_finite() || length <= 0. {
        return Err(unsupported());
    }
    let delta = direction.map(|v| v * length);
    if delta.iter().any(|x| !x.is_finite()) {
        return Err(unsupported());
    }
    Ok((
        surface,
        PCurve::Affine {
            origin,
            direction: delta,
        },
    ))
}
struct Builder<'a> {
    db: &'a Database,
    scale: f64,
    solid: Solid,
    vertices: BTreeMap<u32, usize>,
    edges: BTreeMap<u32, usize>,
    associations: BTreeMap<usize, Vec<(u32, PCurve)>>,
    surfaces: Vec<u32>,
    holed: bool,
}
impl Builder<'_> {
    fn vertex(&mut self, id: u32) -> Result<usize> {
        if let Some(&v) = self.vertices.get(&id) {
            return Ok(v);
        }
        if self.solid.vertices.len() >= if self.holed { 16 } else { 8 } {
            return Err(unsupported());
        }
        let a = self.db.simple(id, "VERTEX_POINT", 2)?;
        let point = self.db.point(reference(&a[1])?, self.scale)?;
        let index = self.solid.vertices.len();
        self.solid.vertices.push(Vertex { point });
        self.vertices.insert(id, index);
        Ok(index)
    }
    fn edge(&mut self, id: u32) -> Result<usize> {
        if let Some(&edge) = self.edges.get(&id) {
            return Ok(edge);
        }
        if self.solid.edges.len() >= if self.holed { 24 } else { 12 } {
            return Err(unsupported());
        }
        let a = self.db.simple(id, "EDGE_CURVE", 5)?;
        if !boolean(&a[4])? {
            return Err(unsupported());
        }
        let vertices = [
            self.vertex(reference(&a[1])?)?,
            self.vertex(reference(&a[2])?)?,
        ];
        let geometry = self.db.simple(reference(&a[3])?, "SURFACE_CURVE", 4)?;
        if geometry[3] != Value::Enum("CURVE_3D".into()) {
            return Err(unsupported());
        }
        let curve = curve(self.db, reference(&geometry[1])?, self.scale)?;
        let associated = list(&geometry[2])?;
        if associated.len() != 2 {
            return Err(unsupported());
        }
        let associations = associated
            .iter()
            .map(|a| pcurve(self.db, reference(a)?))
            .collect::<Result<Vec<_>>>()?;
        if associations[0].0 == associations[1].0 {
            return Err(unsupported());
        }
        let index = self.solid.edges.len();
        self.solid.edges.push(Edge {
            vertices,
            curve: Curve::Nurbs(Box::new(curve)),
        });
        self.associations.insert(index, associations);
        self.edges.insert(id, index);
        Ok(index)
    }
    fn face(&mut self, id: u32) -> Result<()> {
        let a = self.db.simple(id, "ADVANCED_FACE", 4)?;
        let bounds = list(&a[1])?;
        if bounds.is_empty() || bounds.len() > if self.holed { 2 } else { 1 } {
            return Err(unsupported());
        }
        let surface_id = reference(&a[2])?;
        if self.surfaces.contains(&surface_id) {
            return Err(unsupported());
        }
        let surface = surface(self.db, surface_id, self.scale)?;
        let mut outer = None;
        let mut inner = None;
        for bound in bounds {
            let id = reference(bound)?;
            let record = self.db.records.get(&id).ok_or_else(unsupported)?;
            if record.len() != 1 {
                return Err(unsupported());
            }
            let kind = record[0].0.as_str();
            if kind != "FACE_OUTER_BOUND" && !(self.holed && kind == "FACE_BOUND") {
                return Err(unsupported());
            }
            let bound = self.db.simple(id, kind, 3)?;
            if !boolean(&bound[2])? {
                return Err(unsupported());
            }
            let edge_loop = self.db.simple(reference(&bound[1])?, "EDGE_LOOP", 2)?;
            let uses = list(&edge_loop[1])?;
            if uses.len() != 4 {
                return Err(unsupported());
            }
            let mut coedges = Vec::new();
            for value in uses {
                let oriented = self.db.simple(reference(value)?, "ORIENTED_EDGE", 5)?;
                if oriented[1] != Value::Derived || oriented[2] != Value::Derived {
                    return Err(unsupported());
                }
                let edge = self.edge(reference(&oriented[3])?)?;
                let pcurve = self.associations[&edge]
                    .iter()
                    .find(|(id, _)| *id == surface_id)
                    .ok_or_else(unsupported)?
                    .1
                    .clone();
                coedges.push(Coedge {
                    edge,
                    forward: boolean(&oriented[4])?,
                    pcurve,
                });
            }
            let slot = if kind == "FACE_OUTER_BOUND" {
                &mut outer
            } else {
                &mut inner
            };
            if slot.replace(Wire { coedges }).is_some() {
                return Err(unsupported());
            }
        }
        let mut wires = vec![outer.ok_or_else(unsupported)?];
        if let Some(inner) = inner {
            wires.push(inner);
        }
        self.surfaces.push(surface_id);
        self.solid.shell.faces.push(Face {
            surface: Surface::Nurbs(Box::new(surface)),
            wires,
            orientation: if boolean(&a[3])? { 1 } else { -1 },
        });
        Ok(())
    }
}
fn same_curve(a: &Curve, b: &Curve) -> bool {
    let (Curve::Nurbs(a), Curve::Nurbs(b)) = (a, b) else {
        return false;
    };
    a.degree() == b.degree()
        && a.knots() == b.knots()
        && a.weights() == b.weights()
        && a.control_points() == b.control_points()
}
fn same_surface(a: &Surface, b: &Surface) -> Result<bool> {
    let (Surface::Nurbs(a), Surface::Nurbs(b)) = (a, b) else {
        return Ok(false);
    };
    Ok(a.degrees() == b.degrees()
        && a.control_counts() == b.control_counts()
        && a.knots(0)? == b.knots(0)?
        && a.knots(1)? == b.knots(1)?
        && a.weights() == b.weights()
        && a.control_points() == b.control_points())
}
fn reindex(mut actual: Solid, expected: &Solid) -> Result<Solid> {
    let mut vertex_map = Vec::new();
    let mut seen = BTreeSet::new();
    for vertex in &actual.vertices {
        let index = expected
            .vertices
            .iter()
            .position(|v| v.point == vertex.point)
            .ok_or_else(unsupported)?;
        if !seen.insert(index) {
            return Err(unsupported());
        }
        vertex_map.push(index);
    }
    let mut vertices = expected.vertices.clone();
    for (index, vertex) in actual.vertices.into_iter().enumerate() {
        vertices[vertex_map[index]] = vertex;
    }
    actual.vertices = vertices;
    let mut edge_map = Vec::new();
    seen.clear();
    for edge in &mut actual.edges {
        edge.vertices = edge.vertices.map(|i| vertex_map[i]);
        let index = expected
            .edges
            .iter()
            .position(|e| e.vertices == edge.vertices && same_curve(&e.curve, &edge.curve))
            .ok_or_else(unsupported)?;
        if !seen.insert(index) {
            return Err(unsupported());
        }
        edge_map.push(index);
    }
    let mut edges = expected.edges.clone();
    for (index, edge) in actual.edges.into_iter().enumerate() {
        edges[edge_map[index]] = edge;
    }
    actual.edges = edges;
    let mut faces = expected.shell.faces.clone();
    seen.clear();
    for mut face in actual.shell.faces {
        let mut matching = None;
        for (index, e) in expected.shell.faces.iter().enumerate() {
            if same_surface(&e.surface, &face.surface)? {
                matching = Some(index);
                break;
            }
        }
        let index = matching.ok_or_else(unsupported)?;
        if !seen.insert(index) {
            return Err(unsupported());
        }
        if face.wires.len() != expected.shell.faces[index].wires.len() {
            return Err(unsupported());
        }
        for (wire, target_wire) in face
            .wires
            .iter_mut()
            .zip(&expected.shell.faces[index].wires)
        {
            for c in &mut wire.coedges {
                c.edge = edge_map[c.edge];
            }
            let target = target_wire.coedges[0].edge;
            let start = wire
                .coedges
                .iter()
                .position(|c| c.edge == target)
                .ok_or_else(unsupported)?;
            wire.coedges.rotate_left(start);
        }
        faces[index] = face;
    }
    actual.shell.faces = faces;
    Ok(actual)
}
fn recognize(actual: Solid, tol: Tolerance) -> Result<NurbsGraphSolid> {
    if actual.vertices.len() != 8 || actual.edges.len() != 12 || actual.shell.faces.len() != 6 {
        return Err(unsupported());
    }
    let mut candidates = Vec::new();
    for face in &actual.shell.faces {
        let Surface::Nurbs(s) = &face.surface else {
            return Err(unsupported());
        };
        if s.degrees() != [2, 2]
            || s.control_counts() != [3, 3]
            || s.domain() != [[0., 1.], [0., 1.]]
        {
            continue;
        }
        let p = s.control_points();
        let l = p[6].x;
        let w = p[2].y;
        let h = p[0].z;
        let b = p[4].z - h;
        if let Ok(candidate) = NurbsGraphSolid::new([l, w, h], b, tol) {
            if same_surface(&face.surface, &candidate.brep().shell.faces[1].surface)? {
                candidates.push(candidate);
            }
        }
    }
    if candidates.len() != 1 {
        return Err(unsupported());
    }
    let mut candidate = candidates.pop().unwrap();
    let actual = reindex(actual, candidate.brep())?;
    candidate.solid = actual;
    candidate.validate(tol).map_err(|_| unsupported())?;
    Ok(candidate)
}
// Decode one bounded graph-family body before strict geometric recognition.
// An optional kind constraint keeps both original typed entry points strict.
fn read_solid(input: &str, tol: Tolerance, expected_holed: Option<bool>) -> Result<Solid> {
    Tolerance::new(tol.linear)?;
    let db = Parser::new(input)?.document_graph()?;
    let root = db.unique("MANIFOLD_SOLID_BREP")?;
    let brep = db.simple(root, "MANIFOLD_SOLID_BREP", 2)?;
    let shell = reference(&brep[1])?;
    if db.unique("CLOSED_SHELL")? != shell {
        return Err(unsupported());
    }
    let shell = db.simple(shell, "CLOSED_SHELL", 2)?;
    let faces = list(&shell[1])?;
    let holed = match faces.len() {
        6 => false,
        10 => true,
        _ => return Err(unsupported()),
    };
    if expected_holed.is_some_and(|expected| expected != holed) {
        return Err(unsupported());
    }
    let representation = db.unique("ADVANCED_BREP_SHAPE_REPRESENTATION")?;
    let shape = db.simple(representation, "ADVANCED_BREP_SHAPE_REPRESENTATION", 3)?;
    if list(&shape[1])? != [Value::Ref(root)] {
        return Err(unsupported());
    }
    let scale = db.units(reference(&shape[2])?)?;
    let mut builder = Builder {
        db: &db,
        scale,
        solid: Solid {
            vertices: Vec::new(),
            edges: Vec::new(),
            shell: Shell { faces: Vec::new() },
        },
        vertices: BTreeMap::new(),
        edges: BTreeMap::new(),
        associations: BTreeMap::new(),
        surfaces: Vec::new(),
        holed,
    };
    let mut seen = BTreeSet::new();
    for face in faces {
        let id = reference(face)?;
        if !seen.insert(id) {
            return Err(unsupported());
        }
        builder.face(id)?;
    }
    const GEOMETRY: &[&str] = &[
        "CARTESIAN_POINT",
        "DIRECTION",
        "VECTOR",
        "LINE",
        "B_SPLINE_CURVE_WITH_KNOTS",
        "B_SPLINE_SURFACE_WITH_KNOTS",
        "SURFACE_CURVE",
        "PCURVE",
        "DEFINITIONAL_REPRESENTATION",
        "GEOMETRIC_REPRESENTATION_CONTEXT",
        "AXIS2_PLACEMENT_3D",
        "PLANE",
        "CIRCLE",
        "CYLINDRICAL_SURFACE",
        "SEAM_CURVE",
        "VERTEX_POINT",
        "EDGE_CURVE",
        "ORIENTED_EDGE",
        "EDGE_LOOP",
        "FACE_BOUND",
        "FACE_OUTER_BOUND",
        "ADVANCED_FACE",
        "CLOSED_SHELL",
        "MANIFOLD_SOLID_BREP",
        "ADVANCED_BREP_SHAPE_REPRESENTATION",
    ];
    for (&id, record) in &db.records {
        if record
            .iter()
            .any(|(name, _)| GEOMETRY.contains(&name.as_str()))
            && !db.used.borrow().contains(&id)
        {
            return Err(Error::Unsupported(
                "STEP graph contains unused or unrepresented geometry",
            ));
        }
    }
    Ok(builder.solid)
}

/// Strict plain-only import; holed bodies use the separate scoped importer.
pub fn import_step_nurbs_graph_mm(input: &str, tol: Tolerance) -> Result<NurbsGraphSolid> {
    recognize(read_solid(input, tol, Some(false))?, tol)
}

fn coefficient_neighbors(seed: f64) -> Vec<f64> {
    let mut result = vec![seed];
    let mut lo = seed;
    let mut hi = seed;
    for _ in 0..64 {
        lo = lo.next_down();
        hi = hi.next_up();
        if lo.is_finite() {
            result.push(lo);
        }
        if hi.is_finite() {
            result.push(hi);
        }
    }
    result
}
fn recognize_holed(actual: Solid, tol: Tolerance) -> Result<NurbsGraphHoledSolid> {
    if actual.vertices.len() != 16 || actual.edges.len() != 24 || actual.shell.faces.len() != 10 {
        return Err(unsupported());
    }
    for face in &actual.shell.faces {
        let Surface::Nurbs(roof) = &face.surface else {
            return Err(unsupported());
        };
        if face.wires.len() != 2
            || roof.degrees() != [2, 2]
            || roof.control_counts() != [7, 7]
            || roof.domain() != [[0., 1.], [0., 1.]]
        {
            continue;
        }
        let points = roof.control_points();
        let dimensions = [points[42].x, points[6].y, points[0].z];
        let h = dimensions[2];
        if dimensions.iter().any(|v| !v.is_finite() || *v <= 0.) {
            continue;
        }
        let mut hole = [[f64::INFINITY, f64::NEG_INFINITY]; 2];
        for coedge in &face.wires[1].coedges {
            for t in actual.edges[coedge.edge].curve.range() {
                let uv = coedge.pcurve.evaluate(t);
                if uv.iter().any(|value| !value.is_finite()) {
                    return Err(unsupported());
                }
                for axis in 0..2 {
                    hole[axis][0] = hole[axis][0].min(uv[axis]);
                    hole[axis][1] = hole[axis][1].max(uv[axis]);
                }
            }
        }
        // Unit-weight refinement treats XYZ components independently. A flat
        // canonical certificate therefore provides the identical XY coefficients
        // and corner coordinates, regardless of roof height offset. Reject an
        // incompatible placement/topology footprint before coefficient search.
        let Ok(flat_source) = NurbsGraphSolid::new(dimensions, 0., tol) else {
            continue;
        };
        let Ok(flat) = NurbsGraphHoledSolid::new(&flat_source, hole, tol) else {
            continue;
        };
        let Surface::Nurbs(flat_roof) = &flat.brep().shell.faces[1].surface else {
            unreachable!()
        };
        if roof.knots(0)? != flat_roof.knots(0)?
            || roof.knots(1)? != flat_roof.knots(1)?
            || roof
                .control_points()
                .iter()
                .zip(flat_roof.control_points())
                .any(|(a, b)| a.x != b.x || a.y != b.y)
            || actual.vertices.iter().any(|v| {
                !flat
                    .brep()
                    .vertices
                    .iter()
                    .any(|p| p.point.x == v.point.x && p.point.y == v.point.y)
            })
        {
            continue;
        }
        let mut blossom: [Vec<f64>; 2] = [Vec::new(), Vec::new()];
        for (axis, values) in blossom.iter_mut().enumerate() {
            let knots = roof.knots(axis)?;
            for i in 0..7 {
                let s = knots[i + 1];
                let t = knots[i + 2];
                values.push(s + t - 2. * s * t);
            }
        }
        let mut largest = 0.;
        let mut location = 0;
        for i in 0..7 {
            for j in 0..7 {
                let coefficient = blossom[0][i] * blossom[1][j];
                if !coefficient.is_finite() {
                    return Err(unsupported());
                }
                if coefficient > largest {
                    largest = coefficient;
                    location = i * 7 + j;
                }
            }
        }
        if largest <= 0. {
            continue;
        }
        let seed = (points[location].z - h) / largest;
        if !seed.is_finite() {
            continue;
        }
        // B1²(s,t)=s+t−2st is the quadratic Bernstein blossom. It only supplies
        // bounded candidate seeds: acceptance below requires every actual
        // surface/curve coefficient, basis, PCurve and topology to match exactly.
        let mut candidates = vec![0.];
        if seed.abs() >= 0.5 * h {
            candidates.extend(coefficient_neighbors(seed));
        } else {
            let center = h + seed;
            if !center.is_finite() {
                continue;
            }
            candidates.extend(coefficient_neighbors(center).into_iter().map(|c| c - h));
        }
        let mut tried = BTreeSet::new();
        for b in candidates {
            if !b.is_finite() || !tried.insert(b.to_bits()) {
                continue;
            }
            let Ok(source) = NurbsGraphSolid::new(dimensions, b, tol) else {
                continue;
            };
            let Ok(mut candidate) = NurbsGraphHoledSolid::new(&source, hole, tol) else {
                continue;
            };
            if !same_surface(&face.surface, &candidate.brep().shell.faces[1].surface)? {
                continue;
            }
            let Ok(parsed) = reindex(actual.clone(), candidate.brep()) else {
                continue;
            };
            candidate.solid = parsed;
            if candidate.validate(tol).is_ok() {
                return Ok(candidate);
            }
        }
    }
    Err(Error::Unsupported("canonical holed graph coefficient recovery is unresolved within the bounded exact recognition search"))
}
/// Import one unplaced, full-source-domain canonical graph with one rectangular opening.
/// Candidate coefficient recovery is bounded; only exact actual B-rep matches succeed.
/// The recovered canonical representative need not reproduce an original input recipe.
pub fn import_step_nurbs_graph_holed_mm(
    input: &str,
    tol: Tolerance,
) -> Result<NurbsGraphHoledSolid> {
    recognize_holed(read_solid(input, tol, Some(true))?, tol)
}

/// A checked graph-family STEP body. Variants preserve their exact typed certificates.
/// This is not a generic NURBS shell or an analytic STEP import result.
#[derive(Debug, Clone)]
pub enum ImportedNurbsGraph {
    Plain(Box<NurbsGraphSolid>),
    Holed(Box<NurbsGraphHoledSolid>),
}

impl ImportedNurbsGraph {
    /// Actual retained shared topology and spline geometry, without reconstruction.
    pub fn brep(&self) -> &Solid {
        match self {
            Self::Plain(body) => body.brep(),
            Self::Holed(body) => body.brep(),
        }
    }
    pub fn validate(&self, tol: Tolerance) -> Result<()> {
        match self {
            Self::Plain(body) => body.validate(tol),
            Self::Holed(body) => body.validate(tol),
        }
    }
    pub fn volume(&self) -> Result<f64> {
        match self {
            Self::Plain(body) => body.volume(),
            Self::Holed(body) => body.volume(),
        }
    }
    pub fn bounds(&self) -> Result<Bounds> {
        match self {
            Self::Plain(body) => body.bounds(),
            Self::Holed(body) => body.bounds(),
        }
    }
    pub fn mass_properties(&self, tol: Tolerance) -> Result<NurbsGraphMassProperties> {
        match self {
            Self::Plain(body) => body.mass_properties(tol),
            Self::Holed(body) => body.mass_properties(tol),
        }
    }
    pub fn inertia_properties(&self, tol: Tolerance) -> Result<NurbsGraphInertiaProperties> {
        match self {
            Self::Plain(body) => body.inertia_properties(tol),
            Self::Holed(body) => body.inertia_properties(tol),
        }
    }
    pub fn roof_section(
        &self,
        start: [f64; 2],
        end: [f64; 2],
        tol: GeometryTolerance,
    ) -> Result<NurbsGraphRoofSection> {
        match self {
            Self::Plain(body) => body.roof_section(start, end, tol),
            Self::Holed(body) => body.roof_section(start, end, tol),
        }
    }
    pub fn tessellate_bounded(
        &self,
        error: f64,
        max_cells: usize,
        tol: Tolerance,
    ) -> Result<NurbsGraphMesh> {
        match self {
            Self::Plain(body) => body.tessellate_bounded(error, max_cells, tol),
            Self::Holed(body) => body.tessellate_bounded(error, max_cells, tol),
        }
    }
    pub fn classify_point(&self, point: Point3, tol: GeometryTolerance) -> Result<PointLocation> {
        match self {
            Self::Plain(body) => body.classify_point(point, tol),
            Self::Holed(body) => body.classify_point(point, tol),
        }
    }
    pub fn vertical_section(
        &self,
        uv: [f64; 2],
        tol: GeometryTolerance,
    ) -> Result<NurbsGraphVerticalSection> {
        match self {
            Self::Plain(body) => body.vertical_section(uv, tol),
            Self::Holed(body) => body.vertical_section(uv, tol),
        }
    }
    pub fn export_step_mm(&self, tol: Tolerance) -> Result<String> {
        match self {
            Self::Plain(body) => body.export_step_mm(tol),
            Self::Holed(body) => body.export_step_mm(tol),
        }
    }
}

/// Parse once and recognize either supported graph-family body from actual topology.
/// Six faces select plain recognition; ten select single-opening recognition.
/// Face counts only select validation: every retained invariant must still match.
/// A failed recognizer is returned directly, never retried as another shape kind.
pub fn import_step_nurbs_graph_auto_mm(input: &str, tol: Tolerance) -> Result<ImportedNurbsGraph> {
    let actual = read_solid(input, tol, None)?;
    match actual.shell.faces.len() {
        6 => recognize(actual, tol).map(|body| ImportedNurbsGraph::Plain(Box::new(body))),
        10 => recognize_holed(actual, tol).map(|body| ImportedNurbsGraph::Holed(Box::new(body))),
        _ => Err(unsupported()),
    }
}
