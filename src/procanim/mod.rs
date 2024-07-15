//! a simple procedural animation attempt based on https://www.youtube.com/watch?v=qlfh_rv6khY

use bevy::{
    color::palettes::css::{PINK, RED, WHITE},
    prelude::*,
    window::PrimaryWindow,
};

use crate::screen::Screen;

#[derive(Component)]
pub struct AnimationMovementTarget {
    pub target_pos: Vec2,
    pub speed: f32,
}

#[derive(Component)]
pub struct AnimationRoot {
    pub children: Vec<Entity>,
}

#[derive(Component)]
pub struct AnimationNode {
    pub radius: f32,
    pub direction: Vec2,
}

impl AnimationNode {
    pub fn new(radius: f32) -> Self {
        Self {
            radius,
            direction: Vec2::ZERO,
        }
    }

    /// Gets the "left and right" 90 degree side points of the animation
    /// node based on the direction
    pub fn get_side_points(&self) -> (Vec2, Vec2) {
        let angle = self.direction.to_angle();

        let left_theta = angle - std::f32::consts::FRAC_PI_2;
        let right_theta = angle + std::f32::consts::FRAC_PI_2;

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
pub struct AnimationChild;

#[derive(Default, Reflect, GizmoConfigGroup)]
struct ProcanimGizmoGroup;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Update, (roots_follow_mouse, draw_anim_gizmos).chain())
        .add_systems(FixedUpdate, update_procedural_animations)
        .init_gizmo_group::<ProcanimGizmoGroup>()
        .add_systems(OnEnter(Screen::Playing), spawn_procedural_item);
}

pub fn update_procedural_animations(
    roots: Query<(&AnimationRoot, &AnimationNode, &Transform), Without<AnimationChild>>,
    mut nodes: Query<(&mut AnimationNode, &mut Transform), With<AnimationChild>>,
) {
    for (root, root_node, root_tx) in roots.iter() {
        let mut parent_pos = root_tx.translation;
        let mut parent_radius = root_node.radius;

        for node_entity in root.children.iter() {
            if let Ok((mut node, mut tx)) = nodes.get_mut(*node_entity) {
                let delta = tx.translation - parent_pos;
                tx.translation = parent_pos + delta.normalize_or_zero() * parent_radius;

                node.direction = (tx.translation - parent_pos).truncate();

                parent_pos = tx.translation;
                parent_radius = node.radius;
            } else {
                warn!("Missing child node, aborting animation");
                break;
            }
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////
// some test systems for playing around
////////////////////////////////////////////////////////////////////////////////////

fn spawn_procedural_item(mut commands: Commands) {
    info!("Spawning procanim item");

    let radii = [
        22., 26., 25., 22., 24., 25., 25., 25., 25., 25., 25., 25., 25., 25., 25., 25., 25., 25.,
        25., 25., 25., 25., 25., 25., 25., 20., 15., 10.,
    ];

    let children = radii
        .iter()
        .skip(1)
        .map(|radius| {
            commands
                .spawn((
                    SpatialBundle::from_transform(Transform::from_xyz(*radius, *radius, 0.)),
                    AnimationNode::new(*radius),
                    AnimationChild,
                ))
                .id()
        })
        .collect::<Vec<_>>();

    commands.spawn((
        SpatialBundle::default(),
        AnimationMovementTarget {
            target_pos: Vec2::ZERO,
            speed: 500.,
        },
        AnimationRoot { children },
        AnimationNode::new(radii[0]),
    ));
}

fn draw_anim_gizmos(
    mut gizmos: Gizmos<ProcanimGizmoGroup>,
    roots: Query<
        (
            &AnimationRoot,
            &AnimationNode,
            &Transform,
            &AnimationMovementTarget,
        ),
        Without<AnimationChild>,
    >,
    nodes: Query<(&AnimationNode, &Transform), With<AnimationChild>>,
) {
    for (root, root_node, root_tx, move_target) in roots.iter() {
        let (mut prev_r, mut prev_l) = root_node.get_side_points();
        let mut prev_pos = root_tx.translation.truncate();

        // draw the snake direction
        gizmos.line_2d(
            root_tx.translation.truncate(),
            root_tx.translation.truncate() + root_node.direction,
            RED,
        );

        // draw the movement target
        gizmos.circle_2d(move_target.target_pos, 2.0, PINK);

        // draw the snake head
        gizmos.arc_2d(
            root_tx.translation.truncate(),
            root_node.direction.to_angle() - std::f32::consts::FRAC_PI_2,
            std::f32::consts::PI,
            root_node.radius,
            WHITE,
        );

        // draw the animation radius for the root node
        gizmos.circle_2d(
            root_tx.translation.truncate(),
            root_node.radius,
            Color::Srgba(Srgba::new(0., 0., 1., 0.1)),
        );

        for node_entity in root.children.iter() {
            let (node, tx) = nodes.get(*node_entity).unwrap();
            let tx_2d = tx.translation.truncate();
            let (l, r) = node.get_side_points();

            // draw the centre
            gizmos.circle_2d(tx_2d, 2.0, Color::Srgba(Srgba::new(1., 1., 1., 0.1)));
            // draw the animation radius
            gizmos.circle_2d(
                tx_2d,
                node.radius,
                Color::Srgba(Srgba::new(1., 1., 0., 0.1)),
            );

            // draw the side points
            gizmos.circle_2d(tx_2d + l, 2.0, Color::Srgba(Srgba::new(1., 0., 0., 0.3)));
            gizmos.circle_2d(tx_2d + r, 2.0, Color::Srgba(Srgba::new(1., 0.687, 0., 0.3)));

            // connect the dots on the L/R sides
            gizmos.line_2d(prev_pos + prev_l, tx_2d + l, WHITE);
            gizmos.line_2d(prev_pos + prev_r, tx_2d + r, WHITE);

            // store the l/r points for next time
            prev_l = l;
            prev_r = r;
            prev_pos = tx_2d;
        }
    }
}

fn roots_follow_mouse(
    time: Res<Time>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<IsDefaultUiCamera>>,
    mut roots: Query<
        (
            &mut Transform,
            &mut AnimationMovementTarget,
            &mut AnimationNode,
        ),
        With<AnimationRoot>,
    >,
) {
    let (camera, camera_transform) = cameras.single();

    if let Some(target_pos) = windows
        .single()
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor))
        .map(|ray| ray.origin.truncate())
    {
        for (mut tx, mut movement, mut node) in roots.iter_mut() {
            movement.target_pos = target_pos;

            tx.translation = tx.translation.move_towards(
                target_pos.extend(0.0),
                time.delta_seconds() * movement.speed,
            );

            let delta = target_pos - tx.translation.truncate();
            // look towards the target
            if delta.length_squared() > 20. {
                node.direction = delta;
            }
        }
    }
}
