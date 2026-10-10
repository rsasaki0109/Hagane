//! Original ISO 10303-21 / AP214 writer for the canonical polynomial graph family.
//! Actual retained spline bases and same-parameter UV uses are serialized without fitting.
use crate::*;
use std::fmt::Write;
#[derive(Default)]
struct Writer {
    records: Vec<String>,
}
fn number(value: f64) -> String {
    let s = value.to_string();
    if s.contains('.') || s.contains('e') || s.contains('E') {
        s.replace('e', "E")
    } else {
        format!("{s}.")
    }
}
fn refs(ids: &[usize]) -> String {
    ids.iter()
        .map(|id| format!("#{id}"))
        .collect::<Vec<_>>()
        .join(",")
}
fn logical(value: bool) -> &'static str {
    if value {
        ".T."
    } else {
        ".F."
    }
}
fn knot_data(knots: &[f64]) -> Result<(String, String)> {
    if knots.is_empty() || knots.iter().any(|x| !x.is_finite()) {
        return Err(Error::InvalidInput("STEP spline knots must be finite"));
    }
    let mut values = Vec::new();
    let mut mults = Vec::new();
    for &knot in knots {
        if values.last().is_some_and(|last| *last == knot) {
            *mults.last_mut().unwrap() += 1usize;
        } else {
            values.push(knot);
            mults.push(1);
        }
    }
    Ok((
        mults
            .iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join(","),
        values.into_iter().map(number).collect::<Vec<_>>().join(","),
    ))
}
pub(crate) fn decompose_affine(origin: [f64; 2], direction: [f64; 2]) -> Result<([f64; 2], f64)> {
    // Use the same elementary operations on native and WASM: platform
    // hypot implementations can round differently and change STEP bytes.
    // Scaling avoids squaring large coordinates or underflowing tiny ones.
    let scale = direction[0].abs().max(direction[1].abs());
    let length = if scale > 0. && scale.is_finite() {
        let x = direction[0] / scale;
        let y = direction[1] / scale;
        scale * (x * x + y * y).sqrt()
    } else {
        scale
    };
    if origin.iter().chain(&direction).any(|x| !x.is_finite())
        || !length.is_finite()
        || length == 0.
    {
        return Err(Error::InvalidTopology("graph STEP pcurve is unresolved"));
    }
    Ok(([direction[0] / length, direction[1] / length], length))
}
impl Writer {
    fn direction3(&mut self, v: Vec3) -> Result<usize> {
        if !v.finite() {
            return Err(Error::InvalidInput("STEP direction is nonfinite"));
        }
        self.entity(format!(
            "DIRECTION('',({},{},{}))",
            number(v.x),
            number(v.y),
            number(v.z)
        ))
    }
    fn line3(&mut self, a: Point3, b: Point3) -> Result<usize> {
        let delta = b - a;
        let scale = delta.x.abs().max(delta.y.abs()).max(delta.z.abs());
        if !delta.finite() || !scale.is_finite() || scale <= 0. {
            return Err(Error::InvalidTopology("STEP line direction is unresolved"));
        }
        let unit = Vec3::new(delta.x / scale, delta.y / scale, delta.z / scale);
        let length = scale * (unit.x * unit.x + unit.y * unit.y + unit.z * unit.z).sqrt();
        if !length.is_normal() {
            return Err(Error::Unsupported("STEP line magnitude is unresolved"));
        }
        let point = self.point(a)?;
        let direction = self.direction3(Vec3::new(
            delta.x / length,
            delta.y / length,
            delta.z / length,
        ))?;
        let vector = self.entity(format!("VECTOR('',#{direction},{})", number(length)))?;
        self.entity(format!("LINE('',#{point},#{vector})"))
    }
    fn plane3(&mut self, origin: Point3, u: Vec3, v: Vec3, tolerance: Tolerance) -> Result<usize> {
        if !crate::geometry::plane_basis_valid(u, v, GeometryTolerance::try_from(tolerance)?) {
            return Err(Error::InvalidTopology("STEP plane basis is unresolved"));
        }
        let point = self.point(origin)?;
        let axis = self.direction3(u.cross(v).normalized()?)?;
        let reference = self.direction3(u)?;
        let placement = self.entity(format!(
            "AXIS2_PLACEMENT_3D('',#{point},#{axis},#{reference})"
        ))?;
        self.entity(format!("PLANE('',#{placement})"))
    }
    fn entity(&mut self, body: String) -> Result<usize> {
        if self.records.len() >= 100000 {
            return Err(Error::Unsupported(
                "graph STEP export exceeds entity budget",
            ));
        }
        self.records.push(body);
        Ok(self.records.len())
    }
    fn point(&mut self, p: Point3) -> Result<usize> {
        if !p.finite() {
            return Err(Error::InvalidInput("STEP control point is nonfinite"));
        }
        self.entity(format!(
            "CARTESIAN_POINT('',({},{},{}))",
            number(p.x),
            number(p.y),
            number(p.z)
        ))
    }
    fn curve(&mut self, curve: &NurbsCurve) -> Result<usize> {
        if curve.weights().iter().any(|w| !w.is_finite() || *w <= 0.) {
            return Err(Error::InvalidInput(
                "STEP rational curve weights must be positive and finite",
            ));
        }
        let points = curve
            .control_points()
            .iter()
            .map(|p| self.point(*p))
            .collect::<Result<Vec<_>>>()?;
        self.curve_definition(curve, &points)
    }
    fn curve_uv(&mut self, curve: &NurbsCurve) -> Result<usize> {
        if curve
            .control_points()
            .iter()
            .any(|p| !p.finite() || p.z != 0.)
            || curve.weights().iter().any(|w| !w.is_finite() || *w <= 0.)
        {
            return Err(Error::InvalidInput(
                "STEP rational UV curve must have finite planar controls and positive weights",
            ));
        }
        let points = curve
            .control_points()
            .iter()
            .map(|p| {
                self.entity(format!(
                    "CARTESIAN_POINT('',({},{}))",
                    number(p.x),
                    number(p.y)
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        self.curve_definition(curve, &points)
    }
    fn curve_definition(&mut self, curve: &NurbsCurve, points: &[usize]) -> Result<usize> {
        let (mults, knots) = knot_data(curve.knots())?;
        if curve.weights().iter().all(|w| *w == 1.) {
            self.entity(format!("B_SPLINE_CURVE_WITH_KNOTS('',{},({}),.UNSPECIFIED.,.F.,.F.,({mults}),({knots}),.UNSPECIFIED.)",curve.degree(),refs(points)))
        } else {
            let weights = curve
                .weights()
                .iter()
                .copied()
                .map(number)
                .collect::<Vec<_>>()
                .join(",");
            self.entity(format!("(BOUNDED_CURVE() B_SPLINE_CURVE({},({}),.UNSPECIFIED.,.F.,.F.) B_SPLINE_CURVE_WITH_KNOTS(({mults}),({knots}),.UNSPECIFIED.) CURVE() GEOMETRIC_REPRESENTATION_ITEM() RATIONAL_B_SPLINE_CURVE(({weights})) REPRESENTATION_ITEM(''))",curve.degree(),refs(points)))
        }
    }
    fn surface(&mut self, surface: &NurbsSurface) -> Result<usize> {
        if surface.weights().iter().any(|w| !w.is_finite() || *w <= 0.) {
            return Err(Error::InvalidInput(
                "STEP rational surface weights must be positive and finite",
            ));
        }
        let [nu, nv] = surface.control_counts();
        let mut rows = Vec::new();
        for i in 0..nu {
            let points = surface.control_points()[i * nv..(i + 1) * nv]
                .iter()
                .map(|p| self.point(*p))
                .collect::<Result<Vec<_>>>()?;
            rows.push(format!("({})", refs(&points)));
        }
        let (um, uk) = knot_data(surface.knots(0)?)?;
        let (vm, vk) = knot_data(surface.knots(1)?)?;
        let [ud, vd] = surface.degrees();
        if surface.weights().iter().all(|w| *w == 1.) {
            self.entity(format!("B_SPLINE_SURFACE_WITH_KNOTS('',{ud},{vd},({}),.UNSPECIFIED.,.F.,.F.,.F.,({um}),({vm}),({uk}),({vk}),.UNSPECIFIED.)",rows.join(",")))
        } else {
            let weights = surface
                .weights()
                .chunks(nv)
                .map(|row| {
                    format!(
                        "({})",
                        row.iter()
                            .copied()
                            .map(number)
                            .collect::<Vec<_>>()
                            .join(",")
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            self.entity(format!("(BOUNDED_SURFACE() B_SPLINE_SURFACE({ud},{vd},({}),.UNSPECIFIED.,.F.,.F.,.F.) B_SPLINE_SURFACE_WITH_KNOTS(({um}),({vm}),({uk}),({vk}),.UNSPECIFIED.) GEOMETRIC_REPRESENTATION_ITEM() RATIONAL_B_SPLINE_SURFACE(({weights})) REPRESENTATION_ITEM('') SURFACE())",rows.join(",")))
        }
    }
    fn pcurve(&mut self, surface: usize, pcurve: &PCurve, context: usize) -> Result<usize> {
        if let PCurve::Nurbs(curve) = pcurve {
            let geometry = self.curve_uv(curve)?;
            let representation = self.entity(format!(
                "DEFINITIONAL_REPRESENTATION('',(#{geometry}),#{context})"
            ))?;
            return self.entity(format!("PCURVE('',#{surface},#{representation})"));
        }
        let PCurve::Affine { origin, direction } = pcurve else {
            return Err(Error::Unsupported(
                "graph STEP export requires retained affine pcurves",
            ));
        };
        let (ratios, length) = decompose_affine(*origin, *direction)?;
        let point = self.entity(format!(
            "CARTESIAN_POINT('',({},{}))",
            number(origin[0]),
            number(origin[1])
        ))?;
        let axis = self.entity(format!(
            "DIRECTION('',({},{}))",
            number(ratios[0]),
            number(ratios[1])
        ))?;
        let vector = self.entity(format!("VECTOR('',#{axis},{})", number(length)))?;
        let line = self.entity(format!("LINE('',#{point},#{vector})"))?;
        let representation = self.entity(format!(
            "DEFINITIONAL_REPRESENTATION('',(#{line}),#{context})"
        ))?;
        self.entity(format!("PCURVE('',#{surface},#{representation})"))
    }
}
pub(crate) fn write(solid: &Solid, tolerance: Tolerance) -> Result<String> {
    Tolerance::new(tolerance.linear)?;
    if solid.vertices.len() > 4096 || solid.edges.len() > 4096 || solid.shell.faces.len() > 512 {
        return Err(Error::Unsupported(
            "graph STEP export exceeds topology budget",
        ));
    }
    let mut controls = 0usize;
    for edge in &solid.edges {
        match &edge.curve {
            Curve::Nurbs(c) => controls = controls.saturating_add(c.control_points().len()),
            Curve::Line { .. } => (),
            _ => {
                return Err(Error::Unsupported(
                    "typed spline STEP requires spline or straight edge geometry",
                ))
            }
        }
    }
    for face in &solid.shell.faces {
        match &face.surface {
            Surface::Nurbs(s) => controls = controls.saturating_add(s.control_points().len()),
            Surface::Plane { .. } => (),
            _ => {
                return Err(Error::Unsupported(
                    "typed spline STEP requires spline or planar face geometry",
                ))
            }
        }
        for coedge in face.wires.iter().flat_map(|w| &w.coedges) {
            if let PCurve::Nurbs(curve) = &coedge.pcurve {
                controls = controls.saturating_add(curve.control_points().len());
            }
        }
    }
    if controls > 65536 {
        return Err(Error::Unsupported(
            "graph STEP export exceeds control budget",
        ));
    }
    let mut w = Writer::default();
    let mut vertices = Vec::new();
    for vertex in &solid.vertices {
        let point = w.point(vertex.point)?;
        vertices.push(w.entity(format!("VERTEX_POINT('',#{point})"))?);
    }
    let mut surfaces = Vec::new();
    for face in &solid.shell.faces {
        surfaces.push(match &face.surface {
            Surface::Nurbs(surface) => w.surface(surface)?,
            Surface::Plane { origin, u, v } => w.plane3(*origin, *u, *v, tolerance)?,
            _ => unreachable!(),
        });
    }
    let uv_context =
        w.entity("(GEOMETRIC_REPRESENTATION_CONTEXT(2) REPRESENTATION_CONTEXT('',''))".into())?;
    let mut uses = vec![Vec::new(); solid.edges.len()];
    for (fi, face) in solid.shell.faces.iter().enumerate() {
        for c in face.wires.iter().flat_map(|wire| &wire.coedges) {
            if c.edge >= uses.len() {
                return Err(Error::InvalidTopology("STEP coedge reference is invalid"));
            }
            uses[c.edge].push((fi, c));
        }
    }
    let mut edges = Vec::new();
    for (ei, edge) in solid.edges.iter().enumerate() {
        if uses[ei].len() != 2
            || uses[ei][0].0 == uses[ei][1].0
            || edge.vertices.iter().any(|v| *v >= vertices.len())
        {
            return Err(Error::InvalidTopology(
                "graph STEP requires two distinct face uses per shared edge",
            ));
        }
        let geometry = match &edge.curve {
            Curve::Nurbs(curve) => w.curve(curve)?,
            Curve::Line { a, b } => w.line3(*a, *b)?,
            _ => unreachable!(),
        };
        let pcurves = uses[ei]
            .iter()
            .map(|(fi, c)| w.pcurve(surfaces[*fi], &c.pcurve, uv_context))
            .collect::<Result<Vec<_>>>()?;
        let geometry = w.entity(format!(
            "SURFACE_CURVE('',#{geometry},({}),.CURVE_3D.)",
            refs(&pcurves)
        ))?;
        edges.push(w.entity(format!(
            "EDGE_CURVE('',#{},#{},#{geometry},.T.)",
            vertices[edge.vertices[0]], vertices[edge.vertices[1]]
        ))?);
    }
    let mut faces = Vec::new();
    for (fi, face) in solid.shell.faces.iter().enumerate() {
        let mut bounds = Vec::new();
        for (wi, wire) in face.wires.iter().enumerate() {
            let oriented = wire
                .coedges
                .iter()
                .map(|c| {
                    w.entity(format!(
                        "ORIENTED_EDGE('',*,*,#{},{})",
                        edges[c.edge],
                        logical(c.forward)
                    ))
                })
                .collect::<Result<Vec<_>>>()?;
            let edge_loop = w.entity(format!("EDGE_LOOP('',({}))", refs(&oriented)))?;
            bounds.push(w.entity(format!(
                "{}('',#{edge_loop},.T.)",
                if wi == 0 {
                    "FACE_OUTER_BOUND"
                } else {
                    "FACE_BOUND"
                }
            ))?);
        }
        faces.push(w.entity(format!(
            "ADVANCED_FACE('',({}),#{},{})",
            refs(&bounds),
            surfaces[fi],
            logical(face.orientation > 0)
        ))?);
    }
    let shell = w.entity(format!("CLOSED_SHELL('',({}))", refs(&faces)))?;
    let brep = w.entity(format!("MANIFOLD_SOLID_BREP('Hagane graph part',#{shell})"))?;
    let length = w.entity("(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.))".into())?;
    let angle = w.entity("(NAMED_UNIT(*) PLANE_ANGLE_UNIT() SI_UNIT($,.RADIAN.))".into())?;
    let solid_angle =
        w.entity("(NAMED_UNIT(*) SI_UNIT($,.STERADIAN.) SOLID_ANGLE_UNIT())".into())?;
    let uncertainty = w.entity(format!(
        "UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE({}),#{length},'distance_accuracy_value','')",
        number(tolerance.linear)
    ))?;
    let context=w.entity(format!("(GEOMETRIC_REPRESENTATION_CONTEXT(3) GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#{uncertainty})) GLOBAL_UNIT_ASSIGNED_CONTEXT((#{length},#{angle},#{solid_angle})) REPRESENTATION_CONTEXT('',''))"))?;
    let representation = w.entity(format!(
        "ADVANCED_BREP_SHAPE_REPRESENTATION('',(#{brep}),#{context})"
    ))?;
    let application = w.entity("APPLICATION_CONTEXT('automotive_design')".into())?;
    w.entity(format!("APPLICATION_PROTOCOL_DEFINITION('international standard','automotive_design',2000,#{application})"))?;
    let product_context = w.entity(format!("PRODUCT_CONTEXT('',#{application},'mechanical')"))?;
    let product = w.entity(format!(
        "PRODUCT('hagane','Hagane graph part','',(#{product_context}))"
    ))?;
    let formation = w.entity(format!("PRODUCT_DEFINITION_FORMATION('','',#{product})"))?;
    let definition_context = w.entity(format!(
        "PRODUCT_DEFINITION_CONTEXT('part definition',#{application},'design')"
    ))?;
    let definition = w.entity(format!(
        "PRODUCT_DEFINITION('design','',#{formation},#{definition_context})"
    ))?;
    let shape = w.entity(format!("PRODUCT_DEFINITION_SHAPE('','',#{definition})"))?;
    w.entity(format!(
        "SHAPE_DEFINITION_REPRESENTATION(#{shape},#{representation})"
    ))?;
    let mut result=String::from("ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('Hagane exact polynomial graph B-rep'),'2;1');\nFILE_NAME('hagane-graph.step','',('Hagane'),(''),'Hagane','Hagane','');\nFILE_SCHEMA(('AUTOMOTIVE_DESIGN'));\nENDSEC;\nDATA;\n");
    for (i, record) in w.records.iter().enumerate() {
        writeln!(result, "#{}={record};", i + 1).expect("writing into String");
        if result.len() > 32 * 1024 * 1024 {
            return Err(Error::Unsupported("graph STEP output exceeds 32 MiB"));
        }
    }
    result.push_str("ENDSEC;\nEND-ISO-10303-21;\n");
    Ok(result)
}
impl NurbsGraphSolid {
    /// Export this validated polynomial B-rep as AP214, interpreting lengths as mm.
    /// Original spline knots/UV domains and both PCURVE uses are preserved.
    pub fn export_step_mm(&self, tol: Tolerance) -> Result<String> {
        self.validate(tol)?;
        write(self.brep(), tol)
    }
}
impl NurbsGraphHoledSolid {
    /// Export the actual holed caps and eight shared ruled walls, never a mesh.
    pub fn export_step_mm(&self, tol: Tolerance) -> Result<String> {
        self.validate(tol)?;
        write(self.brep(), tol)
    }
}

impl NurbsGraphPolygonSolid {
    /// AP214 export of actual shared polygon boundaries and rational ruled walls.
    /// Nonunit positive weights are preserved, including binary64 near-unit values.
    pub fn export_step_mm(&self, tol: Tolerance) -> Result<String> {
        self.validate(tol)?;
        write(self.brep(), tol)
    }
}
impl NurbsGraphPolygonHoledSolid {
    /// Export the retained genus-one B-rep, including both annular cap wires.
    pub fn export_step_mm(&self, tol: Tolerance) -> Result<String> {
        self.validate(tol)?;
        write(self.brep(), tol)
    }
}
impl NurbsGraphPolygonMultiHoledSolid {
    /// Export the actual multiply-holed manifold B-rep as AP214 in millimetres.
    /// Every cap wire, shared edge, affine pcurve and rational weight is retained.
    pub fn export_step_mm(&self, tol: Tolerance) -> Result<String> {
        self.validate(tol)?;
        write(self.brep(), tol)
    }
}

impl NurbsGraphCircularHoledSolid {
    /// Export the actual circular bore, including degree-eight rational rims and
    /// degree-two rational UV pcurves, as AP214 with three-dimensional lengths in mm.
    pub fn export_step_mm(&self, tol: Tolerance) -> Result<String> {
        self.validate(tol)?;
        write(self.brep(), tol)
    }
}
