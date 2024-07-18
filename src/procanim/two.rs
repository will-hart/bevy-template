use bevy::{
    color::palettes::css::{PINK, RED, WHITE},
    prelude::*,
    window::PrimaryWindow,
};

use std::f32::consts::{FRAC_PI_2, PI, TAU};

use crate::screen::Screen;

use super::ProcanimGizmoGroup;

pub(crate) fn plugin(app: &mut App) {
    app.add_systems(Update, (roots_follow_mouse, draw_anim_gizmos).chain())
        .add_systems(
            FixedUpdate,
            (resolve_chain, position_chain_children).chain(),
        )
        .init_gizmo_group::<ProcanimGizmoGroup>()
        .add_systems(OnEnter(Screen::Playing), spawn_chain_system);
}

#[derive(Component)]
pub struct ChainMovement {
    target: Vec3,
    speed: f32,
}

#[derive(Component)]
pub struct Chain {
    pub links: Vec<ChainLink>,
    pub link_length: f32,
    pub max_angle: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct ChainLink {
    /// angle from this joint to the next joint in the chain
    pub angle: f32,
    /// The 2d position of this joint compared to the previous joint
    pub position: Vec2,
    /// the radius of this joint (for display purposes)
    pub radius: f32,
}

impl ChainLink {
    /// Converts this link into relative coordinates from the previous location
    pub fn to_coordinates(&self, link_length: f32) -> Vec3 {
        Vec3::new(
            link_length * self.angle.cos(),
            link_length * self.angle.sin(),
            0.0,
        )
    }

    /// Gets the "left and right" 90 degree side points of the animation
    /// node based on the direction
    pub fn get_side_points(&self) -> (Vec2, Vec2) {
        let left_theta = self.angle - FRAC_PI_2;
        let right_theta = self.angle + FRAC_PI_2;

        (
            Vec2::new(
                self.radius * left_theta.cos(),
                self.radius * left_theta.sin(),
            ),
            Vec2::new(
                self.radius * right_theta.cos(),
                self.radius * right_theta.sin(),
            ),
        )
    }
}

#[derive(Component)]
pub struct ChainJoint;

fn spawn_chain_system(mut commands: Commands) {
    spawn_chain(
        &mut commands,
        40.,
        0.4,
        &[
            22., 26., 25., 22., 24., 23., 21., 19., 17., 15., 13., 11., 10., 10., 8., 6.,
        ],
    )
}

fn spawn_chain(commands: &mut Commands, link_length: f32, max_angle: f32, radii: &[f32]) {
    commands
        .spawn((
            SpatialBundle::from_transform(Transform::from_xyz(0.0, 0.0, 0.0)),
            Chain {
                links: radii
                    .iter()
                    .map(|r| ChainLink {
                        angle: 0.0,
                        position: Vec2::new(link_length, 0.0),
                        radius: *r,
                    })
                    .collect(),
                link_length,
                max_angle,
            },
            ChainMovement {
                target: Vec3::ZERO,
                speed: 500.,
            },
        ))
        .with_children(|root| {
            let mut initial_pos = 0.;

            radii.iter().for_each(|r| {
                initial_pos += *r;
                root.spawn((
                    SpatialBundle::from_transform(Transform::from_xyz(initial_pos, 0.0, 0.0)),
                    ChainJoint,
                ));
            });
        });
}

/// For each chain, loop through the chain links and position the children.
/// See https://github.com/argonautcode/animal-proc-anim/blob/main/Chain.pde
fn resolve_chain(mut chains: Query<(&mut Chain, &ChainMovement, &Transform)>) {
    for (mut chain, movement, chain_tx) in chains.iter_mut() {
        let max_angle = chain.max_angle;
        let link_length = chain.link_length;

        // chain is positioned elsewhere, just copy the position in here
        chain.links[0].position = chain_tx.translation.truncate();

        let delta = chain_tx.translation - movement.target;
        if delta.length_squared() > 50. {
            // prevent updating the first angle if we haven't moved
            chain.links[0].angle = delta.truncate().to_angle();
        }

        // then go an move all the child links
        let mut prev_link = chain.links[0];

        for link in chain.links.iter_mut().skip(1) {
            link.angle = (link.position - prev_link.position).to_angle();
            link.position =
                prev_link.position + Vec2::from_angle(link.angle).normalize_or_zero() * link_length;

            prev_link = link.clone();
        }
    }
}

// fn constrain_angle(angle: f32, anchor: f32, constraint: f32) -> f32 {
//     let diff = angle - anchor;

//     if diff.abs() < constraint {
//         angle
//     } else if diff > constraint {
//         anchor - constraint
//     } else {
//         anchor + constraint
//     }
// }

// fn angle_diff(angle_a: f32, angle_b: f32) -> f32 {
//     let mut angle = angle_a + PI - angle_b;
//     angle = simplify_angle(angle);
//     PI - angle
// }

// fn simplify_angle(angle: f32) -> f32 {
//     let mut angle = angle;

//     while angle < 0. {
//         angle += TAU;
//     }

//     angle % TAU
// }

/// For each chain, position the child transforms using the calculated chain positions
fn position_chain_children(
    chains: Query<(&Children, &Chain, &Transform)>,
    mut links: Query<&mut Transform, (With<ChainJoint>, Without<Chain>)>,
) {
    for (children, chain, chain_tx) in chains.iter() {
        // now move the children into the correct position
        let mut prev_translation = chain_tx.translation;

        for (idx, &child) in children.iter().enumerate() {
            if let Ok(mut tx) = links.get_mut(child) {
                let link = &chain.links[idx];

                tx.translation = link.to_coordinates(chain.link_length) + prev_translation;
                prev_translation = tx.translation;
            } else {
                warn!("Missing chain link");
            }
        }
    }
}

fn draw_anim_gizmos(
    mut gizmos: Gizmos<ProcanimGizmoGroup>,
    chains: Query<(&Chain, &ChainMovement, &Transform)>,
) {
    for (chain, move_target, chain_tx) in chains.iter() {
        // draw the snake direction
        gizmos.line_2d(
            chain_tx.translation.truncate(),
            move_target.target.truncate(),
            RED,
        );

        // draw the movement target
        gizmos.circle_2d(move_target.target.truncate(), 2.0, PINK);

        // draw the snake head
        gizmos.arc_2d(
            chain_tx.translation.truncate(),
            (move_target.target - chain_tx.translation)
                .truncate()
                .to_angle()
                - FRAC_PI_2,
            PI,
            chain.links[0].radius,
            WHITE,
        );

        // draw each child node
        for links in chain.links.windows(2) {
            let link = links[0];
            let next_link = links[1];

            let (l1, r1) = link.get_side_points();
            let (l2, r2) = next_link.get_side_points();

            // draw the centre of the prev link
            gizmos.circle_2d(
                link.position,
                2.0,
                Color::Srgba(Srgba::new(1., 1., 1., 0.1)),
            );

            // draw the animation radius
            gizmos.circle_2d(
                link.position,
                link.radius,
                Color::Srgba(Srgba::new(1., 1., 0., 0.1)),
            );

            // draw the side points
            gizmos.circle_2d(
                link.position + l1,
                2.0,
                Color::Srgba(Srgba::new(1., 0., 0., 0.3)),
            );
            gizmos.circle_2d(
                link.position + r1,
                2.0,
                Color::Srgba(Srgba::new(1., 0.687, 0., 0.3)),
            );

            // connect the dots on the L/R sides
            gizmos.line_2d(link.position + l1, next_link.position + l2, WHITE);
            gizmos.line_2d(link.position + r1, next_link.position + r2, WHITE);
        }
    }
}

fn roots_follow_mouse(
    time: Res<Time>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<IsDefaultUiCamera>>,
    mut roots: Query<(&mut Transform, &mut ChainMovement)>,
) {
    let (camera, camera_transform) = cameras.single();

    if let Some(target_pos) = windows
        .single()
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor))
        .map(|ray| ray.origin.truncate())
    {
        for (mut tx, mut movement) in roots.iter_mut() {
            movement.target = target_pos.extend(0.);

            if (movement.target - tx.translation).length_squared() > 25. {
                tx.translation = tx
                    .translation
                    .move_towards(movement.target, time.delta_seconds() * movement.speed);
            }
        }
    }
}
