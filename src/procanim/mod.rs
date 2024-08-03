//! Procedural animation as per the paper Advanced Character Physics Thomas Jakobsen
//! IO Interactive, Farvergade 2 DK-1463 Copenhagen K Denmark
//! Email: tj@ioi.dk, www: www.ioi.dk/~tj

use bevy::{
    color::palettes::{
        css::WHITE_SMOKE,
        tailwind::{ORANGE_400, ORANGE_600},
    },
    input::common_conditions::input_just_pressed,
    prelude::*,
};

use crate::screen::Screen;

pub const NUM_ITERATIONS: usize = 5;
pub const PHYSICS_SCALE: f32 = 15.0;
pub const BOTTOM_BOUND: Vec3 = Vec3::new(-300., -300., 0.);
pub const TOP_BOUND: Vec3 = Vec3::new(0., 0., 0.);
pub const PARTICLE_START: Vec3 = Vec3::new(-150., -150., 0.);
pub const PARTICLE_START_PREV_OFFSET: Vec3 = Vec3::new(0., PHYSICS_SCALE * -0.25, 0.);
pub const DEFAULT_PARTICLE_GRAVITY: Vec3 = Vec3::new(0., PHYSICS_SCALE * -9.81, 0.0);

#[derive(Default, Reflect, GizmoConfigGroup)]
struct ProcanimGizmoGroup;

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<ParticleGravity>();
    app.init_gizmo_group::<ProcanimGizmoGroup>();
    app.add_systems(OnEnter(Screen::Playing), spawn_particle);
    app.add_systems(
        FixedUpdate,
        (update_particles, constrain_unliked_particles)
            .chain()
            .run_if(in_state(Screen::Playing)),
    );
    app.add_systems(
        Update,
        (
            draw_gizmos.run_if(in_state(Screen::Playing)),
            reset_particles
                .run_if(in_state(Screen::Playing).and_then(input_just_pressed(KeyCode::KeyR))),
        ),
    );
}

#[derive(Debug, Component)]
pub struct Particle {
    pub acceleration: Vec3,
    tx_prev: Vec3,
    colour: Color,
    mass: f32,
}

impl Default for Particle {
    fn default() -> Self {
        Self {
            acceleration: Vec3::ZERO,
            tx_prev: Vec3::ZERO,
            colour: Color::srgb(0.0, 1.0, 0.0),
            mass: 1.,
        }
    }
}

impl Particle {
    pub fn verlet(&mut self, transform: &mut Transform, dt: f32) {
        let x_prime = 2.0 * transform.translation - self.tx_prev + self.acceleration * dt * dt;
        self.tx_prev = transform.translation;
        transform.translation = x_prime;
    }

    pub fn accumulate_forces(&mut self, force: Vec3) {
        self.acceleration = force;
    }

    pub fn satisfy_constraints(tx1: &mut Transform) {
        tx1.translation = tx1.translation.clamp(BOTTOM_BOUND, TOP_BOUND);
    }
}

#[derive(Clone, Copy)]
pub enum ParticleLinkType {
    Exact(f32),
    Min(f32),
    Max(f32),
}

#[derive(Component)]
pub struct ParticleLink {
    pub a: Entity,
    pub b: Entity,
    pub link_type: ParticleLinkType,
}

impl ParticleLink {
    /// In the paper this is constraint 2, which pushes two [`Particle`]s
    /// either closer together or further apart of maintain a given link distance
    fn link_constraint(
        tx1: &mut Transform,
        p1: &Particle,
        tx2: &mut Transform,
        p2: &Particle,
        link_type: ParticleLinkType,
    ) {
        let delta = tx2.translation - tx1.translation;

        // we could use length here, but it involves a sqrt.
        let delta_length = delta.length();

        // handle various constraint types
        let link_length = match link_type {
            ParticleLinkType::Exact(length) => length,
            ParticleLinkType::Min(min_length) => delta_length.max(min_length),
            ParticleLinkType::Max(max_length) => delta_length.min(max_length),
        };

        let inv_mass_1 = if p1.mass.abs() < 0.0001 {
            100_000_000.0
        } else {
            1. / p1.mass
        };
        let inv_mass_2 = if p2.mass.abs() < 0.0001 {
            100_000_000.0
        } else {
            1. / p2.mass
        };

        let link_diff =
            delta * 0.5 * (delta_length - link_length) / (delta_length * (inv_mass_1 + inv_mass_2)); // the amount each particle must move by to maintain the link

        // instead we can try to "approximate the square root function by its
        // 1st order Taylor-expansion at a neighbourhood" of the link_length.
        // I'm going to pretend I know what this means. Currently this doesn't work tho
        // let link_diff =
        //     delta * link_length * link_length / (delta.dot(delta) + link_length * link_length) - 0.5;

        tx2.translation -= inv_mass_2 * link_diff;
        tx1.translation += inv_mass_1 * link_diff;
    }

    pub fn satisfy_constraints(&self, transforms: &mut Query<(&mut Transform, &mut Particle)>) {
        let [(mut tx_a, pa), (mut tx_b, pb)] = transforms
            .get_many_mut([self.a, self.b])
            .expect("find particle a");

        for _ in 1..=NUM_ITERATIONS {
            let before1: Vec3 = tx_a.translation;
            let before2 = tx_b.translation;

            Particle::satisfy_constraints(&mut tx_a);
            Particle::satisfy_constraints(&mut tx_b);

            Self::link_constraint(&mut tx_a, &pa, &mut tx_b, &pb, self.link_type);

            if tx_a.translation == before1 && tx_b.translation == before2 {
                break;
            }
        }
    }
}

#[derive(Deref, Resource)]
pub struct ParticleGravity(Vec3);

impl Default for ParticleGravity {
    fn default() -> Self {
        Self(DEFAULT_PARTICLE_GRAVITY)
    }
}

fn spawn_particle(mut commands: Commands) {
    spawn_demo_particles(&mut commands);
}

fn spawn_demo_particles(commands: &mut Commands) {
    // spawn a single "free" particle
    commands.spawn((
        SpatialBundle {
            transform: Transform::from_translation(PARTICLE_START),
            ..Default::default()
        },
        Particle {
            tx_prev: PARTICLE_START + PARTICLE_START_PREV_OFFSET,
            ..Default::default()
        },
    ));

    // spawn a linked particle
    let a = commands
        .spawn((
            SpatialBundle {
                transform: Transform::from_translation(Vec3::new(25., 10., 0.0) + PARTICLE_START),
                ..Default::default()
            },
            Particle {
                tx_prev: Vec3::new(22., 10., 0.0) + PARTICLE_START + PARTICLE_START_PREV_OFFSET,
                colour: ORANGE_600.into(),
                ..Default::default()
            },
        ))
        .id();
    let b = commands
        .spawn((
            SpatialBundle {
                transform: Transform::from_translation(Vec3::new(47., 12., 0.0) + PARTICLE_START),
                ..Default::default()
            },
            Particle {
                tx_prev: Vec3::new(47., 12., 0.0) + PARTICLE_START + PARTICLE_START_PREV_OFFSET,
                colour: ORANGE_400.into(),
                ..Default::default()
            },
        ))
        .id();
    let c = commands
        .spawn((
            SpatialBundle {
                transform: Transform::from_translation(Vec3::new(17., 22., 0.0) + PARTICLE_START),
                ..Default::default()
            },
            Particle {
                tx_prev: Vec3::new(17., 22., 0.0) + PARTICLE_START + PARTICLE_START_PREV_OFFSET,
                colour: ORANGE_400.into(),
                ..Default::default()
            },
        ))
        .id();
    let d = commands
        .spawn((
            SpatialBundle {
                transform: Transform::from_translation(Vec3::new(1., 42., 0.0) + PARTICLE_START),
                ..Default::default()
            },
            Particle {
                tx_prev: Vec3::new(17., 22., 0.0) + PARTICLE_START + PARTICLE_START_PREV_OFFSET,
                colour: ORANGE_400.into(),
                ..Default::default()
            },
        ))
        .id();
    let e = commands
        .spawn((
            SpatialBundle {
                transform: Transform::from_translation(Vec3::new(-5., 30., 0.0) + PARTICLE_START),
                ..Default::default()
            },
            Particle {
                tx_prev: Vec3::new(17., 22., 0.0) + PARTICLE_START + PARTICLE_START_PREV_OFFSET,
                colour: ORANGE_400.into(),
                ..Default::default()
            },
        ))
        .id();
    let f = commands
        .spawn((
            SpatialBundle {
                transform: Transform::from_translation(Vec3::new(-5., 30., 0.0) + PARTICLE_START),
                ..Default::default()
            },
            Particle {
                tx_prev: Vec3::new(17., 22., 0.0) + PARTICLE_START + PARTICLE_START_PREV_OFFSET,
                colour: ORANGE_400.into(),
                ..Default::default()
            },
        ))
        .id();

    commands.spawn(ParticleLink {
        a,
        b,
        link_type: ParticleLinkType::Exact(15.),
    });

    commands.spawn(ParticleLink {
        a,
        b: c,
        link_type: ParticleLinkType::Exact(25.),
    });

    commands.spawn(ParticleLink {
        a: b,
        b: c,
        link_type: ParticleLinkType::Exact(15.),
    });

    // link the two triangles
    commands.spawn(ParticleLink {
        a: c,
        b: d,
        link_type: ParticleLinkType::Max(30.),
    });

    // make the second triangle
    commands.spawn(ParticleLink {
        a: d,
        b: f,
        link_type: ParticleLinkType::Exact(9.1),
    });
    commands.spawn(ParticleLink {
        a: d,
        b: e,
        link_type: ParticleLinkType::Exact(11.1),
    });
    commands.spawn(ParticleLink {
        a: e,
        b: f,
        link_type: ParticleLinkType::Min(10.),
    });
}

fn update_particles(
    gravity: Res<ParticleGravity>,
    time: Res<Time>,
    links: Query<&ParticleLink>,
    mut particles: Query<(&mut Transform, &mut Particle)>,
) {
    let dt = time.delta_seconds();

    // update all the particles
    for (mut tx, mut particle) in &mut particles {
        particle.accumulate_forces(gravity.0);
        particle.verlet(&mut tx, dt);
    }

    // apply the constraints from the links and bounds
    for link in &links {
        link.satisfy_constraints(&mut particles);
    }
}

fn constrain_unliked_particles(mut particles: Query<&mut Transform, With<Particle>>) {
    for mut tx in &mut particles {
        Particle::satisfy_constraints(&mut tx);
    }
}

fn draw_gizmos(
    mut gizmos: Gizmos<ProcanimGizmoGroup>,
    links: Query<&ParticleLink>,
    particles: Query<(&Transform, &Particle)>,
) {
    let x = (BOTTOM_BOUND.x - TOP_BOUND.x) / 2.;
    let y = (BOTTOM_BOUND.y - TOP_BOUND.y) / 2.;

    // draw the bounding box
    gizmos.rect_2d(
        Vec2::new(x, y),
        Rot2::degrees(0.),
        Vec2::new(2. * x, 2. * y),
        Color::srgb(1., 0., 0.),
    );

    // draw the particles
    for (tx, particle) in &particles {
        gizmos.circle_2d(tx.translation.truncate(), 5.0, particle.colour);
    }

    // draw the links
    for link in &links {
        let [(a, _), (b, _)] = particles
            .get_many([link.a, link.b])
            .expect("get particles from link for gizmos");

        gizmos.line_2d(
            a.translation.truncate(),
            b.translation.truncate(),
            WHITE_SMOKE,
        );
    }
}

fn reset_particles(
    mut commands: Commands,
    particles: Query<Entity, With<Particle>>,
    links: Query<Entity, With<ParticleLink>>,
) {
    for entity in &particles {
        commands.entity(entity).despawn();
    }

    for link in &links {
        commands.entity(link).despawn();
    }

    spawn_demo_particles(&mut commands);
}
