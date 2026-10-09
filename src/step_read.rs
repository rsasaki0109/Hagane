//! Bounded, original Part 21 parser and exact planar and bounded circular STEP reconstruction.
use crate::*;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
/// Shared native/WASM input limit. No allocation follows an unbounded declaration.
pub const STEP_IMPORT_MAX_BYTES: usize = 1_048_576;
const SYNTAX: Error = Error::InvalidInput("invalid or unsupported STEP Part 21 syntax");
#[derive(Clone, Debug, PartialEq)]
enum Value {
    Number(f64),
    String(String),
    Enum(String),
    Ref(u32),
    List(Vec<Value>),
    Typed(String, Vec<Value>),
    Omitted,
    Derived,
}
#[derive(Clone, Debug, PartialEq)]
enum Token {
    Name(String),
    Number(String),
    String(String),
    Enum(String),
    Symbol(u8),
    End,
}
struct Parser<'a> {
    input: &'a str,
    offset: usize,
    look: Token,
    values: usize,
}
impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Result<Self> {
        if input.len() > STEP_IMPORT_MAX_BYTES {
            return Err(Error::InvalidInput("STEP input exceeds 1 MiB"));
        }
        let mut p = Self {
            input,
            offset: 0,
            look: Token::End,
            values: 0,
        };
        p.look = p.lex()?;
        Ok(p)
    }
    fn lex(&mut self) -> Result<Token> {
        let bytes = self.input.as_bytes();
        loop {
            while bytes.get(self.offset).is_some_and(u8::is_ascii_whitespace) {
                self.offset += 1;
            }
            if bytes.get(self.offset..self.offset + 2) != Some(b"/*") {
                break;
            }
            self.offset += 2;
            let end = self.input[self.offset..].find("*/").ok_or(SYNTAX)?;
            self.offset += end + 2;
        }
        let Some(&b) = bytes.get(self.offset) else {
            return Ok(Token::End);
        };
        let start = self.offset;
        self.offset += 1;
        if b == b'\'' {
            let mut result = String::new();
            let mut part = self.offset;
            loop {
                let end = self.input[self.offset..].find('\'').ok_or(SYNTAX)? + self.offset;
                result.push_str(&self.input[part..end]);
                self.offset = end + 1;
                if bytes.get(self.offset) != Some(&b'\'') {
                    break;
                }
                result.push('\'');
                self.offset += 1;
                part = self.offset;
            }
            return Ok(Token::String(result));
        }
        if b.is_ascii_alphabetic() || b == b'_' {
            while bytes
                .get(self.offset)
                .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_' || *c == b'-')
            {
                self.offset += 1;
            }
            return Ok(Token::Name(
                self.input[start..self.offset].to_ascii_uppercase(),
            ));
        }
        if b == b'.' && bytes.get(self.offset).is_some_and(u8::is_ascii_alphabetic) {
            while bytes
                .get(self.offset)
                .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_')
            {
                self.offset += 1;
            }
            if bytes.get(self.offset) != Some(&b'.') {
                return Err(SYNTAX);
            }
            let name = self.input[start + 1..self.offset].to_ascii_uppercase();
            self.offset += 1;
            return Ok(Token::Enum(name));
        }
        if b.is_ascii_digit() || b == b'+' || b == b'-' || b == b'.' {
            while bytes
                .get(self.offset)
                .is_some_and(|c| c.is_ascii_digit() || b".+-Ee".contains(c))
            {
                self.offset += 1;
            }
            return Ok(Token::Number(self.input[start..self.offset].into()));
        }
        if b"#=(),;$*".contains(&b) {
            return Ok(Token::Symbol(b));
        }
        Err(SYNTAX)
    }
    fn take(&mut self) -> Result<Token> {
        let next = self.lex()?;
        Ok(std::mem::replace(&mut self.look, next))
    }
    fn symbol(&mut self, expected: u8) -> Result<()> {
        if self.take()? != Token::Symbol(expected) {
            return Err(SYNTAX);
        }
        Ok(())
    }
    fn name(&mut self) -> Result<String> {
        if let Token::Name(s) = self.take()? {
            Ok(s)
        } else {
            Err(SYNTAX)
        }
    }
    fn keyword(&mut self, word: &str) -> Result<()> {
        if self.name()? != word {
            return Err(SYNTAX);
        }
        Ok(())
    }
    fn id(&mut self) -> Result<u32> {
        self.symbol(b'#')?;
        let Token::Number(s) = self.take()? else {
            return Err(SYNTAX);
        };
        if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
            return Err(SYNTAX);
        }
        let id = s.parse::<u32>().map_err(|_| SYNTAX)?;
        if id == 0 {
            return Err(SYNTAX);
        }
        Ok(id)
    }
    fn args(&mut self, depth: usize) -> Result<Vec<Value>> {
        if depth > 16 {
            return Err(Error::InvalidInput("STEP nesting exceeds 16 levels"));
        }
        self.symbol(b'(')?;
        let mut result = Vec::new();
        if self.look != Token::Symbol(b')') {
            loop {
                if result.len() >= 4096 {
                    return Err(Error::InvalidInput("STEP list exceeds 4096 values"));
                }
                result.push(self.value(depth + 1)?);
                if self.look != Token::Symbol(b',') {
                    break;
                }
                self.symbol(b',')?;
            }
        }
        self.symbol(b')')?;
        Ok(result)
    }
    fn value(&mut self, depth: usize) -> Result<Value> {
        self.values += 1;
        if self.values > 131072 {
            return Err(Error::InvalidInput("STEP value budget exceeded"));
        }
        Ok(match &self.look {
            Token::Symbol(b'#') => Value::Ref(self.id()?),
            Token::Symbol(b'(') => Value::List(self.args(depth)?),
            Token::Symbol(b'$') => {
                self.take()?;
                Value::Omitted
            }
            Token::Symbol(b'*') => {
                self.take()?;
                Value::Derived
            }
            Token::Name(_) => {
                let name = self.name()?;
                Value::Typed(name, self.args(depth)?)
            }
            _ => match self.take()? {
                Token::String(s) => Value::String(s),
                Token::Enum(s) => Value::Enum(s),
                Token::Number(s) => {
                    let v = s.parse::<f64>().map_err(|_| SYNTAX)?;
                    if !v.is_finite() {
                        return Err(Error::InvalidInput("nonfinite STEP number"));
                    }
                    Value::Number(v)
                }
                _ => return Err(SYNTAX),
            },
        })
    }
    fn component(&mut self) -> Result<(String, Vec<Value>)> {
        let name = self.name()?;
        Ok((name, self.args(0)?))
    }
    fn document(mut self) -> Result<Database> {
        self.keyword("ISO-10303-21")?;
        self.symbol(b';')?;
        self.keyword("HEADER")?;
        self.symbol(b';')?;
        let mut header = BTreeMap::new();
        while self.look != Token::Name("ENDSEC".into()) {
            let (name, args) = self.component()?;
            self.symbol(b';')?;
            if !["FILE_DESCRIPTION", "FILE_NAME", "FILE_SCHEMA"].contains(&name.as_str())
                || header.insert(name, args).is_some()
            {
                return Err(SYNTAX);
            }
        }
        if header.len() != 3 {
            return Err(SYNTAX);
        }
        let description = &header["FILE_DESCRIPTION"];
        if description.len() != 2
            || !list(&description[0])?
                .iter()
                .all(|v| matches!(v, Value::String(_)))
            || description[1] != Value::String("2;1".into())
        {
            return Err(SYNTAX);
        }
        let filename = &header["FILE_NAME"];
        if filename.len() != 7
            || [0, 1, 4, 5, 6]
                .iter()
                .any(|&i| !matches!(filename[i], Value::String(_)))
            || !list(&filename[2])?
                .iter()
                .all(|v| matches!(v, Value::String(_)))
            || !list(&filename[3])?
                .iter()
                .all(|v| matches!(v, Value::String(_)))
        {
            return Err(SYNTAX);
        }
        if header["FILE_SCHEMA"]
            != vec![Value::List(vec![Value::String("AUTOMOTIVE_DESIGN".into())])]
        {
            return Err(Error::Unsupported(
                "STEP import supports AP214 AUTOMOTIVE_DESIGN only",
            ));
        }
        self.keyword("ENDSEC")?;
        self.symbol(b';')?;
        self.keyword("DATA")?;
        self.symbol(b';')?;
        let mut records = BTreeMap::new();
        while self.look != Token::Name("ENDSEC".into()) {
            if records.len() >= 32768 {
                return Err(Error::InvalidInput("STEP entity budget exceeded"));
            }
            let id = self.id()?;
            self.symbol(b'=')?;
            let mut components = Vec::new();
            if self.look == Token::Symbol(b'(') {
                self.symbol(b'(')?;
                while self.look != Token::Symbol(b')') {
                    components.push(self.component()?);
                    if components.len() > 8 {
                        return Err(SYNTAX);
                    }
                }
                self.symbol(b')')?;
            } else {
                components.push(self.component()?);
            }
            self.symbol(b';')?;
            if components.is_empty() || records.insert(id, components).is_some() {
                return Err(Error::InvalidInput("empty or duplicate STEP entity"));
            }
        }
        self.keyword("ENDSEC")?;
        self.symbol(b';')?;
        self.keyword("END-ISO-10303-21")?;
        self.symbol(b';')?;
        if self.look != Token::End {
            return Err(Error::InvalidInput("trailing STEP data"));
        }
        let db = Database {
            records,
            used: RefCell::new(BTreeSet::new()),
        };
        db.check_records()?;
        Ok(db)
    }
}
type Record = Vec<(String, Vec<Value>)>;
struct Database {
    records: BTreeMap<u32, Record>,
    used: RefCell<BTreeSet<u32>>,
}
fn list(value: &Value) -> Result<&[Value]> {
    if let Value::List(v) = value {
        Ok(v)
    } else {
        Err(SYNTAX)
    }
}
fn reference(value: &Value) -> Result<u32> {
    if let Value::Ref(id) = value {
        Ok(*id)
    } else {
        Err(SYNTAX)
    }
}
fn number(value: &Value) -> Result<f64> {
    if let Value::Number(v) = value {
        Ok(*v)
    } else {
        Err(SYNTAX)
    }
}
fn boolean(value: &Value) -> Result<bool> {
    match value {
        Value::Enum(s) if s == "T" => Ok(true),
        Value::Enum(s) if s == "F" => Ok(false),
        _ => Err(SYNTAX),
    }
}
fn named(args: &[Value], count: usize) -> Result<()> {
    if args.len() != count || !matches!(args.first(), Some(Value::String(_))) {
        return Err(SYNTAX);
    }
    Ok(())
}
fn all_refs(value: &Value, output: &mut Vec<u32>) {
    match value {
        Value::Ref(id) => output.push(*id),
        Value::List(values) | Value::Typed(_, values) => {
            for v in values {
                all_refs(v, output);
            }
        }
        _ => (),
    }
}
impl Database {
    fn check_records(&self) -> Result<()> {
        const SIMPLE: &[&str] = &[
            "CARTESIAN_POINT",
            "DIRECTION",
            "VECTOR",
            "LINE",
            "CIRCLE",
            "CYLINDRICAL_SURFACE",
            "SEAM_CURVE",
            "PCURVE",
            "DEFINITIONAL_REPRESENTATION",
            "AXIS2_PLACEMENT_3D",
            "PLANE",
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
            "UNCERTAINTY_MEASURE_WITH_UNIT",
            "APPLICATION_CONTEXT",
            "APPLICATION_PROTOCOL_DEFINITION",
            "PRODUCT_CONTEXT",
            "PRODUCT",
            "PRODUCT_DEFINITION_FORMATION",
            "PRODUCT_DEFINITION_CONTEXT",
            "PRODUCT_DEFINITION",
            "PRODUCT_DEFINITION_SHAPE",
            "SHAPE_DEFINITION_REPRESENTATION",
        ];
        for record in self.records.values() {
            let names: BTreeSet<_> = record.iter().map(|(name, _)| name.as_str()).collect();
            if names.len() != record.len() {
                return Err(SYNTAX);
            }
            if record.len() == 1 {
                if !SIMPLE.contains(&record[0].0.as_str()) {
                    return Err(Error::Unsupported("unsupported STEP entity (unknown curves, assemblies and extensions are rejected)"));
                }
                let signature = match record[0].0.as_str() {
                    "APPLICATION_CONTEXT" => Some("s"),
                    "APPLICATION_PROTOCOL_DEFINITION" => Some("ssnr"),
                    "PRODUCT_CONTEXT" | "PRODUCT_DEFINITION_CONTEXT" => Some("srs"),
                    "PRODUCT" => Some("sssl"),
                    "PRODUCT_DEFINITION_FORMATION" | "PRODUCT_DEFINITION_SHAPE" => Some("ssr"),
                    "PRODUCT_DEFINITION" => Some("ssrr"),
                    "SHAPE_DEFINITION_REPRESENTATION" => Some("rr"),
                    _ => None,
                };
                if let Some(signature) = signature {
                    let args = &record[0].1;
                    if args.len()!=signature.len() || args.iter().zip(signature.bytes()).any(|(v,kind)| match kind {
                        b's'=>!matches!(v,Value::String(_)), b'n'=>!matches!(v,Value::Number(_)), b'r'=>!matches!(v,Value::Ref(_)),
                        b'l'=>!matches!(v,Value::List(values) if !values.is_empty() && values.iter().all(|v| matches!(v,Value::Ref(_)))),_=>true,
                    }) { return Err(Error::InvalidInput("invalid known STEP product metadata fields")); }
                }
            } else {
                let context: BTreeSet<_> = [
                    "GEOMETRIC_REPRESENTATION_CONTEXT",
                    "GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT",
                    "GLOBAL_UNIT_ASSIGNED_CONTEXT",
                    "REPRESENTATION_CONTEXT",
                ]
                .into_iter()
                .collect();
                let is_context = names == context;
                let is_unit = ["LENGTH_UNIT", "PLANE_ANGLE_UNIT", "SOLID_ANGLE_UNIT"]
                    .into_iter()
                    .any(|unit| names == [unit, "NAMED_UNIT", "SI_UNIT"].into_iter().collect());
                let is_uv_context = names
                    == ["GEOMETRIC_REPRESENTATION_CONTEXT", "REPRESENTATION_CONTEXT"]
                        .into_iter()
                        .collect();
                if !is_context && !is_unit && !is_uv_context {
                    return Err(Error::Unsupported("unsupported complex STEP entity"));
                }
            }
            for (_, args) in record {
                for value in args {
                    let mut references = Vec::new();
                    all_refs(value, &mut references);
                    if references.iter().any(|id| !self.records.contains_key(id)) {
                        return Err(Error::InvalidInput("dangling STEP entity reference"));
                    }
                }
            }
        }
        for kind in [
            "PRODUCT",
            "PRODUCT_DEFINITION_FORMATION",
            "PRODUCT_DEFINITION",
            "PRODUCT_DEFINITION_SHAPE",
            "SHAPE_DEFINITION_REPRESENTATION",
        ] {
            if self
                .records
                .values()
                .filter(|r| r.len() == 1 && r[0].0 == kind)
                .count()
                > 1
            {
                return Err(Error::Unsupported(
                    "multiple STEP products or shape definitions are not supported",
                ));
            }
        }
        Ok(())
    }
    fn record(&self, id: u32) -> Result<&Record> {
        self.used.borrow_mut().insert(id);
        self.records
            .get(&id)
            .ok_or(Error::InvalidInput("missing STEP entity"))
    }
    fn component(&self, id: u32, name: &str) -> Result<&[Value]> {
        self.record(id)?
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, args)| args.as_slice())
            .ok_or(Error::InvalidInput(
                "STEP reference has the wrong entity type",
            ))
    }
    fn simple(&self, id: u32, name: &str, count: usize) -> Result<&[Value]> {
        let record = self.record(id)?;
        if record.len() != 1 || record[0].0 != name {
            return Err(Error::Unsupported(
                "STEP reference is outside the supported analytic entity subset",
            ));
        }
        let args = &record[0].1;
        named(args, count)?;
        Ok(args)
    }
    fn unique(&self, name: &str) -> Result<u32> {
        let ids: Vec<_> = self
            .records
            .iter()
            .filter(|(_, r)| r.len() == 1 && r[0].0 == name)
            .map(|(&id, _)| id)
            .collect();
        if ids.len() != 1 {
            return Err(Error::Unsupported(
                "STEP import requires one solid, one closed shell and one shape representation",
            ));
        }
        Ok(ids[0])
    }
    fn point(&self, id: u32, scale: f64) -> Result<Point3> {
        let args = self.simple(id, "CARTESIAN_POINT", 2)?;
        let coords = list(&args[1])?;
        if coords.len() != 3 {
            return Err(SYNTAX);
        }
        let point = Point3::new(
            number(&coords[0])? * scale,
            number(&coords[1])? * scale,
            number(&coords[2])? * scale,
        );
        if !point.finite() {
            return Err(Error::InvalidInput(
                "STEP unit conversion overflows coordinates",
            ));
        }
        Ok(point)
    }
    fn direction(&self, id: u32) -> Result<Vec3> {
        let args = self.simple(id, "DIRECTION", 2)?;
        let coords = list(&args[1])?;
        if coords.len() != 3 {
            return Err(SYNTAX);
        }
        Vec3::new(
            number(&coords[0])?,
            number(&coords[1])?,
            number(&coords[2])?,
        )
        .normalized()
    }
    fn placement(&self, id: u32, scale: f64) -> Result<Frame3> {
        let place = self.simple(id, "AXIS2_PLACEMENT_3D", 4)?;
        let origin = self.point(reference(&place[1])?, scale)?;
        let normal = if place[2] == Value::Omitted {
            Vec3::new(0., 0., 1.)
        } else {
            self.direction(reference(&place[2])?)?
        };
        let reference = if place[3] == Value::Omitted {
            Vec3::new(1., 0., 0.)
        } else {
            self.direction(reference(&place[3])?)?
        };
        let u = (reference - normal * reference.dot(normal)).normalized()?;
        let v = normal.cross(u);
        Frame3::new(origin, [u, v, normal], Tolerance::default())
    }
    fn surface(&self, id: u32, scale: f64, analytic: bool) -> Result<Surface> {
        let record = self.record(id)?;
        if analytic && record.len() == 1 && record[0].0 == "CYLINDRICAL_SURFACE" {
            let args = self.simple(id, "CYLINDRICAL_SURFACE", 3)?;
            let radius = number(&args[2])? * scale;
            if !radius.is_finite() || radius <= 0. {
                return Err(Error::InvalidInput("invalid STEP cylinder radius"));
            }
            return Ok(Surface::FramedCylinder {
                frame: self.placement(reference(&args[1])?, scale)?,
                radius,
                height: 0.,
            });
        }
        let args = self.simple(id, "PLANE", 2)?;
        let frame = self.placement(reference(&args[1])?, scale)?;
        Ok(Surface::Plane {
            origin: frame.origin(),
            u: frame.axes()[0],
            v: frame.axes()[1],
        })
    }
    fn units(&self, context: u32) -> Result<f64> {
        if self
            .records
            .values()
            .filter(|r| {
                r.iter()
                    .any(|(name, _)| name == "GLOBAL_UNIT_ASSIGNED_CONTEXT")
            })
            .count()
            != 1
        {
            return Err(Error::Unsupported(
                "STEP import requires one geometric representation context",
            ));
        }
        let labels = self.component(context, "REPRESENTATION_CONTEXT")?;
        if labels.len() != 2 || !labels.iter().all(|v| matches!(v, Value::String(_))) {
            return Err(SYNTAX);
        }
        let uncertainties = self.component(context, "GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT")?;
        if uncertainties.len() != 1 || list(&uncertainties[0])?.len() != 1 {
            return Err(Error::Unsupported(
                "STEP import requires one explicit length uncertainty",
            ));
        }
        let uncertainty_id = reference(&list(&uncertainties[0])?[0])?;
        let uncertainty = self.component(uncertainty_id, "UNCERTAINTY_MEASURE_WITH_UNIT")?;
        if uncertainty.len() != 4 {
            return Err(SYNTAX);
        }
        let Value::Typed(kind, values) = &uncertainty[0] else {
            return Err(SYNTAX);
        };
        if kind != "LENGTH_MEASURE" || values.len() != 1 || number(&values[0])? <= 0. {
            return Err(Error::InvalidInput("invalid STEP length uncertainty"));
        }
        let uncertainty_unit = reference(&uncertainty[1])?;
        let dimension = self.component(context, "GEOMETRIC_REPRESENTATION_CONTEXT")?;
        if dimension != [Value::Number(3.)] {
            return Err(Error::Unsupported(
                "STEP import requires a 3D representation context",
            ));
        }
        let units = self.component(context, "GLOBAL_UNIT_ASSIGNED_CONTEXT")?;
        if units.len() != 1 {
            return Err(SYNTAX);
        }
        let units = list(&units[0])?;
        if units.len() != 3 {
            return Err(Error::Unsupported(
                "STEP requires explicit length, radian and steradian units",
            ));
        }
        let mut length = None;
        let mut kinds = BTreeSet::new();
        for value in units {
            let id = reference(value)?;
            let record = self.record(id)?;
            let kind = record
                .iter()
                .find(|(name, _)| {
                    name.ends_with("_UNIT") && !["NAMED_UNIT", "SI_UNIT"].contains(&name.as_str())
                })
                .ok_or(SYNTAX)?;
            if !kind.1.is_empty()
                || !kinds.insert(kind.0.as_str())
                || self.component(id, "NAMED_UNIT")? != [Value::Derived]
            {
                return Err(SYNTAX);
            }
            let si = self.component(id, "SI_UNIT")?;
            if si.len() != 2 {
                return Err(SYNTAX);
            }
            match kind.0.as_str() {
                "LENGTH_UNIT" => {
                    if id != uncertainty_unit {
                        return Err(Error::InvalidInput(
                            "STEP uncertainty must use the representation length unit",
                        ));
                    }
                    if si[1] != Value::Enum("METRE".into()) {
                        return Err(Error::Unsupported(
                            "STEP import supports metre or millimetre SI length units",
                        ));
                    }
                    length = Some(match &si[0] {
                        Value::Omitted => 1000.,
                        Value::Enum(s) if s == "MILLI" => 1.,
                        _ => return Err(Error::Unsupported("unsupported STEP SI length prefix")),
                    });
                }
                "PLANE_ANGLE_UNIT" if si == [Value::Omitted, Value::Enum("RADIAN".into())] => (),
                "SOLID_ANGLE_UNIT" if si == [Value::Omitted, Value::Enum("STERADIAN".into())] => (),
                _ => return Err(Error::Unsupported("unsupported STEP angular unit")),
            }
        }
        length.ok_or(Error::InvalidInput("missing STEP length unit"))
    }
}
struct Builder<'a> {
    db: &'a Database,
    scale: f64,
    tol: Tolerance,
    solid: Solid,
    vertices: BTreeMap<u32, usize>,
    edges: BTreeMap<u32, usize>,
    coedge_count: usize,
    analytic: bool,
    seams: BTreeMap<usize, (u32, [PCurve; 2])>,
}
impl Builder<'_> {
    fn vertex(&mut self, id: u32) -> Result<usize> {
        if let Some(&index) = self.vertices.get(&id) {
            return Ok(index);
        }
        if self.vertices.len() >= 4096 {
            return Err(Error::Unsupported(
                "STEP import supports at most 4096 vertices and edges",
            ));
        }
        let args = self.db.simple(id, "VERTEX_POINT", 2)?;
        let point = self.db.point(reference(&args[1])?, self.scale)?;
        let index = self.solid.vertices.len();
        self.solid.vertices.push(Vertex { point });
        self.vertices.insert(id, index);
        Ok(index)
    }
    fn edge(&mut self, id: u32) -> Result<usize> {
        if let Some(&index) = self.edges.get(&id) {
            return Ok(index);
        }
        if self.edges.len() >= 4096 {
            return Err(Error::Unsupported(
                "STEP import supports at most 4096 vertices and edges",
            ));
        }
        let args = self.db.simple(id, "EDGE_CURVE", 5)?;
        let a = self.vertex(reference(&args[1])?)?;
        let b = self.vertex(reference(&args[2])?)?;
        let geometry_id = reference(&args[3])?;
        let record = self.db.record(geometry_id)?;
        if self.analytic && record.len() == 1 && record[0].0 == "CIRCLE" {
            if a != b {
                return Err(Error::Unsupported(
                    "STEP circular import requires a complete one-vertex circle",
                ));
            }
            if !boolean(&args[4])? {
                return Err(Error::Unsupported(
                    "STEP complete circles currently require positive same_sense",
                ));
            }
            let circle = self.db.simple(geometry_id, "CIRCLE", 3)?;
            let frame = self.db.placement(reference(&circle[1])?, self.scale)?;
            let radius = number(&circle[2])? * self.scale;
            if !radius.is_finite() || radius <= self.tol.linear {
                return Err(Error::InvalidInput(
                    "invalid or unresolved STEP circle radius",
                ));
            }
            let curve = Curve::FramedCircle { frame, radius };
            let budget = (64. * f64::EPSILON * radius).min(self.tol.linear / 1024.);
            if (curve.evaluate(0.) - self.solid.vertices[a].point).norm() > budget {
                return Err(Error::Unsupported(
                    "STEP circle vertex must resolve the placement's zero-angle seam",
                ));
            }
            let index = self.solid.edges.len();
            self.solid.edges.push(Edge {
                vertices: [a, b],
                curve,
            });
            self.edges.insert(id, index);
            return Ok(index);
        }
        let mut line_id = geometry_id;
        let mut seam = None;
        if self.analytic && record.len() == 1 && record[0].0 == "SEAM_CURVE" {
            let args = self.db.simple(geometry_id, "SEAM_CURVE", 4)?;
            if args[3] != Value::Enum("CURVE_3D".into()) {
                return Err(Error::Unsupported("STEP seam requires the 3D master curve"));
            }
            line_id = reference(&args[1])?;
            let associated = list(&args[2])?;
            if associated.len() != 2 || associated[0] == associated[1] {
                return Err(Error::InvalidTopology(
                    "STEP periodic seam requires two distinct pcurves",
                ));
            }
            let first = self.seam_pcurve(reference(&associated[0])?)?;
            let second = self.seam_pcurve(reference(&associated[1])?)?;
            if first.0 != second.0 {
                return Err(Error::InvalidTopology(
                    "STEP seam pcurves refer to different surfaces",
                ));
            }
            seam = Some((first.0, [first.1, second.1]));
        }
        let line = self.db.simple(line_id, "LINE", 3)?;
        let origin = self.db.point(reference(&line[1])?, self.scale)?;
        let vector = self.db.simple(reference(&line[2])?, "VECTOR", 3)?;
        let dir = self.db.direction(reference(&vector[1])?)?;
        let magnitude = number(&vector[2])? * self.scale;
        if !magnitude.is_finite() || magnitude <= 0. {
            return Err(Error::InvalidInput("invalid STEP line vector magnitude"));
        }
        let start = self.solid.vertices[a].point;
        let end = self.solid.vertices[b].point;
        let delta = end - start;
        let distances = [(start - origin).norm(), (end - origin).norm()];
        if !delta.norm().is_finite()
            || distances
                .iter()
                .any(|d| !d.is_finite() || 64. * f64::EPSILON * d > self.tol.linear)
        {
            return Err(Error::Unsupported(
                "STEP line coordinates exceed the caller tolerance precision budget",
            ));
        }
        let residuals = [
            (start - origin).cross(dir).norm(),
            (end - origin).cross(dir).norm(),
        ];
        let alignment = delta.dot(dir) * if boolean(&args[4])? { 1. } else { -1. };
        if residuals
            .iter()
            .any(|d| !d.is_finite() || *d > self.tol.linear)
            || !alignment.is_finite()
            || alignment <= self.tol.linear
        {
            return Err(Error::InvalidTopology(
                "STEP line geometry disagrees with edge endpoints or same_sense",
            ));
        }
        let index = self.solid.edges.len();
        self.solid.edges.push(Edge {
            vertices: [a, b],
            curve: Curve::Line { a: start, b: end },
        });
        self.edges.insert(id, index);
        if let Some(seam) = seam {
            self.seams.insert(index, seam);
        }
        Ok(index)
    }
    fn seam_pcurve(&self, id: u32) -> Result<(u32, PCurve)> {
        let args = self.db.simple(id, "PCURVE", 3)?;
        let surface = reference(&args[1])?;
        self.db.simple(surface, "CYLINDRICAL_SURFACE", 3)?;
        let rep = self
            .db
            .simple(reference(&args[2])?, "DEFINITIONAL_REPRESENTATION", 3)?;
        let curves = list(&rep[1])?;
        if curves.len() != 1 {
            return Err(Error::Unsupported("STEP seam pcurve requires one 2D line"));
        }
        let context = reference(&rep[2])?;
        let record = self.db.record(context)?;
        if record.len() != 2
            || self
                .db
                .component(context, "GEOMETRIC_REPRESENTATION_CONTEXT")?
                != [Value::Number(2.)]
            || self.db.component(context, "REPRESENTATION_CONTEXT")?.len() != 2
            || !self
                .db
                .component(context, "REPRESENTATION_CONTEXT")?
                .iter()
                .all(|v| matches!(v, Value::String(_)))
        {
            return Err(Error::InvalidInput("invalid STEP seam UV context"));
        }
        let line = self.db.simple(reference(&curves[0])?, "LINE", 3)?;
        let point = self.db.simple(reference(&line[1])?, "CARTESIAN_POINT", 2)?;
        let point = list(&point[1])?;
        let vector = self.db.simple(reference(&line[2])?, "VECTOR", 3)?;
        let direction = self.db.simple(reference(&vector[1])?, "DIRECTION", 2)?;
        let direction = list(&direction[1])?;
        if point.len() != 2 || direction.len() != 2 {
            return Err(Error::InvalidInput("STEP seam UV coordinates must be 2D"));
        }
        let dx = number(&direction[0])?;
        let dy = number(&direction[1])?;
        let normalized = Vec3::new(dx, dy, 0.).normalized()?;
        let magnitude = number(&vector[2])?;
        let origin = [number(&point[0])?, number(&point[1])? * self.scale];
        let direction = [
            normalized.x * magnitude,
            normalized.y * magnitude * self.scale,
        ];
        if !magnitude.is_finite()
            || magnitude <= 0.
            || origin
                .iter()
                .chain(direction.iter())
                .any(|v| !v.is_finite())
        {
            return Err(Error::InvalidInput(
                "STEP seam UV conversion is invalid or overflows",
            ));
        }
        if ![0., std::f64::consts::TAU].contains(&origin[0])
            || origin[1] != 0.
            || direction[0] != 0.
            || direction[1] <= 10. * self.tol.linear
        {
            return Err(Error::Unsupported(
                "STEP seam pcurves require u=0/2pi and positive axial generators from v=0",
            ));
        }
        Ok((surface, PCurve::Affine { origin, direction }))
    }
    fn edge_pcurve(
        &self,
        edge: usize,
        forward: bool,
        surface: &Surface,
        surface_id: u32,
    ) -> Result<PCurve> {
        match (&self.solid.edges[edge].curve, surface) {
            (Curve::Line { a, b }, Surface::Plane { .. }) => {
                if self.seams.contains_key(&edge) {
                    return Err(Error::Unsupported(
                        "STEP periodic seam cannot bound a plane",
                    ));
                }
                let origin = surface.parameters(*a);
                let end = surface.parameters(*b);
                Ok(PCurve::Affine {
                    origin,
                    direction: [end[0] - origin[0], end[1] - origin[1]],
                })
            }
            (Curve::FramedCircle { frame, radius }, Surface::Plane { origin, u, v }) => {
                let offset = frame.origin() - *origin;
                let budget =
                    (64. * f64::EPSILON * radius.max(offset.norm())).min(self.tol.linear / 1024.);
                if (frame.axes()[0] - *u).norm() > 64. * f64::EPSILON
                    || (frame.axes()[1] - *v).norm() > 64. * f64::EPSILON
                    || !offset.norm().is_finite()
                    || offset.dot(u.cross(*v)).abs() > budget
                {
                    return Err(Error::Unsupported("STEP circular cap requires an aligned positive circle/plane parameter frame"));
                }
                Ok(PCurve::Circle {
                    center: surface.parameters(frame.origin()),
                    radius: *radius,
                })
            }
            (
                Curve::FramedCircle {
                    frame: circle,
                    radius: rim_radius,
                },
                Surface::FramedCylinder { frame, radius, .. },
            ) => {
                let local = frame.local_point(circle.origin());
                let budget =
                    (64. * f64::EPSILON * radius.max(local.norm())).min(self.tol.linear / 1024.);
                if !local.finite()
                    || (rim_radius - radius).abs() > budget
                    || local.x.hypot(local.y) > budget
                    || circle
                        .axes()
                        .iter()
                        .zip(frame.axes())
                        .any(|(a, b)| (*a - b).norm() > 64. * f64::EPSILON)
                {
                    return Err(Error::Unsupported("STEP cylindrical rims require equal radius and aligned coaxial circle frames"));
                }
                Ok(PCurve::Affine {
                    origin: [0., local.z],
                    direction: [1., 0.],
                })
            }
            (Curve::Line { .. }, Surface::FramedCylinder { .. }) => {
                let (owner, pcurves) = self.seams.get(&edge).ok_or(Error::Unsupported(
                    "STEP cylinder generator requires an explicit SEAM_CURVE",
                ))?;
                if *owner != surface_id {
                    return Err(Error::InvalidTopology(
                        "STEP seam belongs to a different surface",
                    ));
                }
                let u = if forward { std::f64::consts::TAU } else { 0. };
                let matched: Vec<_> = pcurves.iter().filter(|p| p.evaluate(0.)[0] == u).collect();
                if matched.len() != 1 {
                    return Err(Error::InvalidTopology(
                        "STEP seam needs one pcurve at each periodic boundary",
                    ));
                }
                Ok(matched[0].clone())
            }
            _ => Err(Error::Unsupported(
                "unsupported STEP edge/surface parameter combination",
            )),
        }
    }
    fn face(&mut self, id: u32) -> Result<()> {
        let args = self.db.simple(id, "ADVANCED_FACE", 4)?;
        let surface_id = reference(&args[2])?;
        let mut surface = self.db.surface(surface_id, self.scale, self.analytic)?;
        let orientation = if boolean(&args[3])? { 1 } else { -1 };
        let bounds = list(&args[1])?;
        if bounds.is_empty() || bounds.len() > 65 {
            return Err(Error::Unsupported(
                "STEP import supports one outer and at most 64 inner bounds",
            ));
        }
        let mut wires = Vec::new();
        let mut outer = None;
        let mut seen = BTreeSet::new();
        let mut face_coedges = 0;
        for value in bounds {
            let bound_id = reference(value)?;
            if !seen.insert(bound_id) {
                return Err(Error::InvalidTopology("duplicate STEP face bound"));
            }
            let record = self.db.records.get(&bound_id).ok_or(SYNTAX)?;
            let is_outer = record.len() == 1 && record[0].0 == "FACE_OUTER_BOUND";
            let bound = self.db.simple(
                bound_id,
                if is_outer {
                    "FACE_OUTER_BOUND"
                } else {
                    "FACE_BOUND"
                },
                3,
            )?;
            if is_outer && outer.replace(wires.len()).is_some() {
                return Err(Error::InvalidTopology("multiple STEP outer bounds"));
            }
            let bound_forward = boolean(&bound[2])?;
            let edge_loop = self.db.simple(reference(&bound[1])?, "EDGE_LOOP", 2)?;
            let entries = list(&edge_loop[1])?;
            self.coedge_count += entries.len();
            face_coedges += entries.len();
            if face_coedges > 256 || self.coedge_count > 4096 {
                return Err(Error::Unsupported(
                    "STEP import supports 256 coedges per face and 4096 total",
                ));
            }
            let mut coedges = Vec::new();
            for value in entries {
                let oriented = self.db.simple(reference(value)?, "ORIENTED_EDGE", 5)?;
                if oriented[1] != Value::Derived || oriented[2] != Value::Derived {
                    return Err(SYNTAX);
                }
                let edge = self.edge(reference(&oriented[3])?)?;
                let forward = boolean(&oriented[4])? == bound_forward;
                let pcurve = self.edge_pcurve(edge, forward, &surface, surface_id)?;
                coedges.push(Coedge {
                    edge,
                    forward,
                    pcurve,
                });
            }
            if !bound_forward {
                coedges.reverse();
            }
            wires.push(Wire { coedges });
        }
        let outer = outer.ok_or(Error::InvalidTopology("missing STEP outer bound"))?;
        let outer_wire = wires.remove(outer);
        wires.insert(0, outer_wire);
        if let Surface::FramedCylinder { frame, radius, .. } = surface {
            if wires.len() != 1 || wires[0].coedges.len() != 4 {
                return Err(Error::Unsupported(
                    "STEP cylinder import requires one full four-coedge rectangle",
                ));
            }
            let coedges = &mut wires[0].coedges;
            let bottom = coedges
                .iter()
                .enumerate()
                .filter(|(_, c)| {
                    matches!(self.solid.edges[c.edge].curve, Curve::FramedCircle { .. })
                })
                .min_by(|(_, a), (_, b)| {
                    a.pcurve.evaluate(0.)[1].total_cmp(&b.pcurve.evaluate(0.)[1])
                })
                .map(|(i, _)| i)
                .ok_or(Error::InvalidTopology(
                    "STEP cylindrical face lacks circular rims",
                ))?;
            coedges.rotate_left(bottom);
            let lower = coedges[0].pcurve.evaluate(0.);
            let upper = coedges[2].pcurve.evaluate(0.);
            if lower != [0., 0.]
                || upper[0] != 0.
                || !upper[1].is_finite()
                || upper[1] <= 10. * self.tol.linear
            {
                return Err(Error::Unsupported("STEP cylinder placement must start at the lower rim with resolved positive height"));
            }
            surface = Surface::FramedCylinder {
                frame,
                radius,
                height: upper[1],
            };
        }
        self.solid.shell.faces.push(Face {
            surface,
            orientation,
            wires,
        });
        Ok(())
    }
}
/// Import one AP214 convex or certified prismatic planar straight-edge solid, converting SI mm/m to mm.
/// Caller tolerance is in mm and is never enlarged by file uncertainty metadata.
/// STEP identities preserve shared topology; no snapping, sewing or repair occurs.
/// Curved geometry, uncertified nonconvex solids, assemblies and unknown entities fail.
pub fn import_step_planar_mm(input: &str, tolerance: Tolerance) -> Result<Solid> {
    import_supported(input, tolerance, false, false)
}
/// Import only convex planar solids; polygon holes and concavity are rejected.
pub fn import_step_convex_planar_mm(input: &str, tolerance: Tolerance) -> Result<Solid> {
    import_supported(input, tolerance, true, false)
}
/// Import the planar subset, a full cylinder/tube, or polygon stock with normal through bores.
/// Circular imports require aligned zero-angle circle/plane/cylinder frames and
/// two explicit seam pcurves. Blind, partial-circle and general curved inputs fail.
pub fn import_step_mm(input: &str, tolerance: Tolerance) -> Result<Solid> {
    import_supported(input, tolerance, false, true)
}
fn import_supported(
    input: &str,
    tolerance: Tolerance,
    convex_only: bool,
    analytic: bool,
) -> Result<Solid> {
    Tolerance::new(tolerance.linear)?;
    let db = Parser::new(input)?.document()?;
    let root = db.unique("MANIFOLD_SOLID_BREP")?;
    let brep = db.simple(root, "MANIFOLD_SOLID_BREP", 2)?;
    let shell = reference(&brep[1])?;
    if db.unique("CLOSED_SHELL")? != shell {
        return Err(SYNTAX);
    }
    let faces = db.simple(shell, "CLOSED_SHELL", 2)?;
    let faces = list(&faces[1])?;
    if faces.is_empty() || faces.len() > 128 {
        return Err(Error::Unsupported("STEP import supports 1..128 faces"));
    }
    let representation = db.unique("ADVANCED_BREP_SHAPE_REPRESENTATION")?;
    let shape = db.simple(representation, "ADVANCED_BREP_SHAPE_REPRESENTATION", 3)?;
    if list(&shape[1])? != [Value::Ref(root)] {
        return Err(Error::Unsupported(
            "STEP representation must contain only its single solid",
        ));
    }
    let scale = db.units(reference(&shape[2])?)?;
    let mut builder = Builder {
        db: &db,
        scale,
        tol: tolerance,
        solid: Solid {
            vertices: Vec::new(),
            edges: Vec::new(),
            shell: Shell { faces: Vec::new() },
        },
        vertices: BTreeMap::new(),
        edges: BTreeMap::new(),
        coedge_count: 0,
        analytic,
        seams: BTreeMap::new(),
    };
    let mut seen = BTreeSet::new();
    for face in faces {
        let id = reference(face)?;
        if !seen.insert(id) {
            return Err(Error::InvalidTopology("duplicate STEP shell face"));
        }
        builder.face(id)?;
    }
    builder.solid.validate(tolerance)?;
    if !builder.solid.volume()?.is_finite()
        || !(builder.solid.bounds().max - builder.solid.bounds().min)
            .norm()
            .is_finite()
    {
        return Err(Error::InvalidTopology(
            "STEP solid metrics exceed finite arithmetic",
        ));
    }
    if analytic
        && builder
            .solid
            .shell
            .faces
            .iter()
            .any(|f| !matches!(f.surface, Surface::Plane { .. }))
    {
        match crate::circular_prism_validation::certify_validated_circular_prism(
            &builder.solid,
            tolerance,
        ) {
            Ok(_) => {}
            Err(Error::Unsupported(_)) => {
                crate::bored_prism_validation::certify_validated_bored_prism(
                    &builder.solid,
                    tolerance,
                )?;
            }
            Err(error) => return Err(error),
        }
    } else {
        match crate::booleans::convex_planes(
            &builder.solid,
            GeometryTolerance::try_from(tolerance)?,
        ) {
            Ok(_) => {}
            Err(Error::Unsupported(_)) if !convex_only => {
                crate::prism_validation::certify_validated_planar_prism(&builder.solid, tolerance)?;
            }
            Err(error) => return Err(error),
        }
    }
    const GEOMETRY: &[&str] = &[
        "CARTESIAN_POINT",
        "DIRECTION",
        "VECTOR",
        "LINE",
        "CIRCLE",
        "CYLINDRICAL_SURFACE",
        "SEAM_CURVE",
        "PCURVE",
        "DEFINITIONAL_REPRESENTATION",
        "GEOMETRIC_REPRESENTATION_CONTEXT",
        "AXIS2_PLACEMENT_3D",
        "PLANE",
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
                "STEP contains unused or unrepresented geometry",
            ));
        }
    }
    Ok(builder.solid)
}
/// Validate imported B-rep and derive its display mesh; never performs mesh CSG.
pub fn import_step_planar_json(input: &str) -> Result<String> {
    let t = Tolerance::default();
    step_report(import_step_planar_mm(input, t)?, t)
}
/// Import and derive a display mesh for the bounded planar/cylinder subset.
pub fn import_step_json(input: &str) -> Result<String> {
    let t = Tolerance::default();
    step_report(import_step_mm(input, t)?, t)
}
fn step_report(solid: Solid, t: Tolerance) -> Result<String> {
    let mesh: serde_json::Value = serde_json::from_str(&solid.mesh_json(0.05, t)?)
        .map_err(|_| Error::InvalidInput("STEP display serialization failed"))?;
    let step = export_step_mm(&solid, t)?;
    let bounds = solid.bounds();
    serde_json::to_string(
        &serde_json::json!({"units":"mm","schema":"AP214","mesh":mesh,"step":step,"vertices":solid.vertices.len(),"bounds":{"min":[bounds.min.x,bounds.min.y,bounds.min.z],"max":[bounds.max.x,bounds.max.y,bounds.max.z]}}),
    )
    .map_err(|_| Error::InvalidInput("STEP report serialization failed"))
}
