use crate::ship::ShipHandle;
use crate::simulation::Simulation;
use nalgebra::{vector, Point2, UnitComplex, Vector4};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Line {
    pub a: Point2<f64>,
    pub b: Point2<f64>,
    pub color: Vector4<f32>,
}

pub fn emit_ship(sim: &mut Simulation, handle: ShipHandle) {
    let mut lines = vec![];
    lines.reserve(2 + sim.ship(handle).data().guns.len());
    let body = sim.ship(handle).body();
    let p = body.position().translation.vector.into();
    lines.push(Line {
        a: p,
        b: p + body.linvel(),
        color: vector![0.0, 0.81, 1.0, 1.0],
    });
    lines.push(Line {
        a: p,
        b: p + body
            .rotation()
            .transform_vector(&sim.ship(handle).data().acceleration),
        color: vector![0.0, 1.0, 0.2, 1.0],
    });
    for gun in sim.ship(handle).data().guns.iter() {
        if gun.min_angle == gun.max_angle {
            continue;
        }
        let turret_rot = UnitComplex::new(gun.heading);
        let p0 = p + body.rotation().transform_vector(&gun.offset);
        let p1 = p0 + turret_rot.transform_vector(&vector![10.0, 0.0]);
        lines.push(Line {
            a: p0,
            b: p1,
            color: vector![1.0, 0.0, 0.0, 1.0],
        });
    }
    sim.emit_debug_lines(handle, lines);
}
