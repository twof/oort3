use super::index_set::{HasIndex, Index};
use crate::script;
use crate::simulation;
use crate::simulation::{bullet, Simulation};
use nalgebra::Vector2;
use rapier2d_f64::prelude::*;

#[derive(Hash, PartialEq, Eq, Copy, Clone, Debug)]
pub struct ShipHandle(pub Index);

impl HasIndex for ShipHandle {
    fn index(self) -> Index {
        self.0
    }
}

#[derive(Clone, Copy, Hash, Eq, PartialEq)]
pub enum ShipClass {
    Fighter,
    Asteroid { variant: i32 },
}

pub struct Weapon {
    reload_time: f64,
    reload_time_remaining: f64,
}

pub struct ShipData {
    pub class: ShipClass,
    pub weapons: Vec<Weapon>,
}

pub fn fighter() -> ShipData {
    ShipData {
        class: ShipClass::Fighter,
        weapons: vec![Weapon {
            reload_time: 0.2,
            reload_time_remaining: 0.0,
        }],
    }
}

pub fn asteroid(variant: i32) -> ShipData {
    ShipData {
        class: ShipClass::Asteroid { variant },
        weapons: vec![],
    }
}

pub fn create(
    sim: &mut Simulation,
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    h: f64,
    data: ShipData,
) -> ShipHandle {
    let rigid_body = RigidBodyBuilder::new_dynamic()
        .translation(vector![x, y])
        .linvel(vector![vx, vy])
        .rotation(h)
        .ccd_enabled(true)
        .build();
    let body_handle = sim.bodies.insert(rigid_body);
    let handle = ShipHandle(body_handle.0);
    match data.class {
        ShipClass::Fighter => {
            let vertices = crate::renderer::model::ship()
                .iter()
                .map(|&v| point![v.x as f64, v.y as f64])
                .collect::<Vec<_>>();
            let collider = ColliderBuilder::convex_hull(&vertices)
                .unwrap()
                .restitution(1.0)
                .active_events(ActiveEvents::CONTACT_EVENTS | ActiveEvents::INTERSECTION_EVENTS)
                .collision_groups(InteractionGroups::new(
                    1 << simulation::SHIP_COLLISION_GROUP,
                    1 << simulation::WALL_COLLISION_GROUP
                        | 1 << simulation::SHIP_COLLISION_GROUP
                        | 1 << simulation::BULLET_COLLISION_GROUP,
                ))
                .build();
            sim.colliders
                .insert_with_parent(collider, body_handle, &mut sim.bodies);
            let sim_ptr = sim as *mut Simulation;
            sim.ship_controllers
                .insert(handle, script::new_ship_controller(handle, sim_ptr));
        }
        ShipClass::Asteroid { variant } => {
            let vertices = crate::renderer::model::asteroid(variant)
                .iter()
                .map(|&v| point![v.x as f64, v.y as f64])
                .collect::<Vec<_>>();
            let collider = ColliderBuilder::convex_hull(&vertices)
                .unwrap()
                .restitution(1.0)
                .active_events(ActiveEvents::CONTACT_EVENTS | ActiveEvents::INTERSECTION_EVENTS)
                .collision_groups(InteractionGroups::new(
                    1 << simulation::SHIP_COLLISION_GROUP,
                    1 << simulation::WALL_COLLISION_GROUP
                        | 1 << simulation::SHIP_COLLISION_GROUP
                        | 1 << simulation::BULLET_COLLISION_GROUP,
                ))
                .build();
            sim.colliders
                .insert_with_parent(collider, body_handle, &mut sim.bodies);
        }
    }
    sim.ships.insert(handle);
    sim.ship_data.insert(handle, data);
    handle
}

pub struct ShipAccessor<'a> {
    pub(crate) simulation: &'a Simulation,
    pub(crate) handle: ShipHandle,
}

fn normalize_heading(mut h: f64) -> f64 {
    while h < 0.0 {
        h += std::f64::consts::TAU;
    }
    while h > std::f64::consts::TAU {
        h -= std::f64::consts::TAU;
    }
    h
}

impl<'a> ShipAccessor<'a> {
    pub fn body(&self) -> &'a RigidBody {
        self.simulation
            .bodies
            .get(RigidBodyHandle(self.handle.index()))
            .unwrap()
    }

    pub fn position(&self) -> Translation<Real> {
        self.body().position().translation
    }

    pub fn velocity(&self) -> Vector<Real> {
        *self.body().linvel()
    }

    pub fn heading(&self) -> Real {
        normalize_heading(self.body().rotation().angle())
    }

    pub fn angular_velocity(&self) -> Real {
        self.body().angvel()
    }

    pub fn data(&self) -> &ShipData {
        self.simulation.ship_data.get(&self.handle).unwrap()
    }
}

pub struct ShipAccessorMut<'a> {
    pub(crate) simulation: &'a mut Simulation,
    pub(crate) handle: ShipHandle,
}

impl<'a: 'b, 'b> ShipAccessorMut<'a> {
    pub fn body(&'b mut self) -> &'b mut RigidBody {
        self.simulation
            .bodies
            .get_mut(RigidBodyHandle(self.handle.index()))
            .unwrap()
    }

    pub fn accelerate(&mut self, acceleration: Vector2<f64>) {
        let body = self.body();
        let rotation_matrix = body.position().rotation.to_rotation_matrix();
        body.apply_force(rotation_matrix * acceleration * body.mass(), true);
    }

    pub fn torque(&mut self, acceleration: f64) {
        let inertia_sqrt = 1.0 / self.body().mass_properties().inv_principal_inertia_sqrt;
        let torque = acceleration * inertia_sqrt * inertia_sqrt;
        self.body().apply_torque(torque, true);
    }

    pub fn fire_weapon(&mut self, index: i64) {
        let ship_data = self.simulation.ship_data.get_mut(&self.handle).unwrap();
        {
            let weapon = &mut ship_data.weapons[index as usize];
            if weapon.reload_time_remaining > 0.0 {
                return;
            }
            weapon.reload_time_remaining += weapon.reload_time;
        }

        let speed = 1000.0;
        let offset = vector![20.0, 0.0];
        let body = self.body();
        let rot = body.position().rotation;
        let p = body.position().translation.vector + rot.transform_vector(&offset);
        let v = body.linvel() + rot.transform_vector(&vector![speed, 0.0]);
        bullet::create(&mut self.simulation, p.x, p.y, v.x, v.y);
    }

    pub fn explode(&mut self) {
        self.simulation.ships.remove(self.handle);
        self.simulation.bodies.remove(
            RigidBodyHandle(self.handle.index()),
            &mut self.simulation.island_manager,
            &mut self.simulation.colliders,
            &mut self.simulation.joints,
        );
    }

    pub fn tick(&mut self) {
        let ship_data = self.simulation.ship_data.get_mut(&self.handle).unwrap();
        for weapon in ship_data.weapons.iter_mut() {
            weapon.reload_time_remaining =
                (weapon.reload_time_remaining - simulation::PHYSICS_TICK_LENGTH).max(0.0);
        }
    }
}
