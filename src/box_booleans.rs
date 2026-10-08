//! Exact coordinate arrangements for axis-aligned box specifications.
use crate::*;
#[derive(Clone, Copy, Debug)]
pub enum BoxBooleanOperation {
    Union,
    Difference,
    Intersection,
}
#[derive(Clone, Debug)]
pub enum BoxBooleanResult {
    Empty,
    Solid(Solid),
}
/// Regularized Boolean on two axis-aligned boxes. Exact coplanarity is allowed;
/// distinct boundary coordinates within ten local length budgets are rejected.
/// Nonempty results must have one closed connected shell (no enclosed cavities).
pub fn boolean_boxes(
    first: BoxSpec,
    second: BoxSpec,
    operation: BoxBooleanOperation,
    tol: GeometryTolerance,
) -> Result<BoxBooleanResult> {
    let a = make_box(first, tol.absolute())?;
    let b = make_box(second, tol.absolute())?;
    let aa = a.bounds();
    let bb = b.bounds();
    let lower = [
        [aa.min.x, aa.min.y, aa.min.z],
        [bb.min.x, bb.min.y, bb.min.z],
    ];
    let upper = [
        [aa.max.x, aa.max.y, aa.max.z],
        [bb.max.x, bb.max.y, bb.max.z],
    ];
    let budget = tol.length_at_scale((aa.max - aa.min).norm().max((bb.max - bb.min).norm()))?;
    let mut coordinates: [Vec<f64>; 3] = std::array::from_fn(|axis| {
        vec![
            lower[0][axis],
            upper[0][axis],
            lower[1][axis],
            upper[1][axis],
        ]
    });
    for axis in &mut coordinates {
        axis.sort_by(f64::total_cmp);
        axis.dedup();
        if axis.windows(2).any(|v| v[1] - v[0] <= 10. * budget) {
            return Err(Error::Unsupported("box arrangements require distinct boundary coordinates clear of the tolerance band"));
        }
    }
    let counts = coordinates.each_ref().map(|v| v.len() - 1);
    let inside = |index: [usize; 3], operand: usize| {
        (0..3).all(|axis| {
            coordinates[axis][index[axis]] >= lower[operand][axis]
                && coordinates[axis][index[axis] + 1] <= upper[operand][axis]
        })
    };
    let occupied = |index: [usize; 3]| {
        let a = inside(index, 0);
        let b = inside(index, 1);
        match operation {
            BoxBooleanOperation::Union => a || b,
            BoxBooleanOperation::Difference => a && !b,
            BoxBooleanOperation::Intersection => a && b,
        }
    };
    let basis = [
        Vec3::new(1., 0., 0.),
        Vec3::new(0., 1., 0.),
        Vec3::new(0., 0., 1.),
    ];
    let mut patches = Vec::new();
    for x in 0..counts[0] {
        for y in 0..counts[1] {
            for z in 0..counts[2] {
                let index = [x, y, z];
                if !occupied(index) {
                    continue;
                }
                for axis in 0..3 {
                    for positive in [false, true] {
                        let mut neighbor = index;
                        let exists = if positive {
                            neighbor[axis] += 1;
                            neighbor[axis] < counts[axis]
                        } else if index[axis] > 0 {
                            neighbor[axis] -= 1;
                            true
                        } else {
                            false
                        };
                        if exists && occupied(neighbor) {
                            continue;
                        }
                        let u = (axis + 1) % 3;
                        let v = (axis + 2) % 3;
                        let mut origin =
                            std::array::from_fn::<_, 3, _>(|i| coordinates[i][index[i]]);
                        origin[axis] = coordinates[axis][index[axis] + usize::from(positive)];
                        let mut ring = Vec::new();
                        for (high_u, high_v) in
                            [(false, false), (true, false), (true, true), (false, true)]
                        {
                            let mut p = origin;
                            p[u] = coordinates[u][index[u] + usize::from(high_u)];
                            p[v] = coordinates[v][index[v] + usize::from(high_v)];
                            ring.push(Point3::new(p[0], p[1], p[2]));
                        }
                        patches.push(PlanarFacePatch {
                            surface: Surface::Plane {
                                origin: Point3::new(origin[0], origin[1], origin[2]),
                                u: basis[u],
                                v: basis[v],
                            },
                            orientation: if positive { 1 } else { -1 },
                            rings: vec![ring],
                        });
                    }
                }
            }
        }
    }
    if patches.is_empty() {
        return Ok(BoxBooleanResult::Empty);
    }
    let solid = sew_planar_faces(&patches, tol)?;
    let overlap = (0..3)
        .map(|axis| {
            (upper[0][axis].min(upper[1][axis]) - lower[0][axis].max(lower[1][axis])).max(0.)
        })
        .product::<f64>();
    let expected = match operation {
        BoxBooleanOperation::Union => a.volume()? + b.volume()? - overlap,
        BoxBooleanOperation::Difference => a.volume()? - overlap,
        BoxBooleanOperation::Intersection => overlap,
    };
    if !expected.is_finite() || (solid.volume()? - expected).abs() > expected.abs() * 1e-10 {
        return Err(Error::InvalidTopology(
            "box Boolean does not conserve analytic volume",
        ));
    }
    Ok(BoxBooleanResult::Solid(solid))
}
pub(crate) fn box_contact_demo(offset: f64) -> Result<Solid> {
    match boolean_boxes(
        BoxSpec {
            min: Point3::new(-40., -30., -12.),
            size: Vec3::new(80., 60., 24.),
        },
        BoxSpec {
            min: Point3::new(40., offset - 10., -4.),
            size: Vec3::new(24., 20., 16.),
        },
        BoxBooleanOperation::Union,
        GeometryTolerance::default(),
    )? {
        BoxBooleanResult::Solid(s) => Ok(s),
        BoxBooleanResult::Empty => Err(Error::InvalidTopology("box contact demo is empty")),
    }
}
pub fn box_contact_demo_json(offset: f64) -> Result<String> {
    box_contact_demo(offset)?.mesh_json(0.05, Tolerance::default())
}
