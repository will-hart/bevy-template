//! a simple procedural animation attempt based on https://www.youtube.com/watch?v=qlfh_rv6khY

use bevy::{
    color::palettes::css::{BLUE, WHITE, YELLOW},
    prelude::*,
    window::PrimaryWindow,
};

use crate::screen::Screen;

#[derive(Component)]
pub struct AnimationRoot {
    pub children: Vec<Entity>,
}

#[derive(Component)]
pub struct AnimationNode {
    pub radius: f32,
}

#[derive(Component)]
pub struct AnimationChild;

#[derive(Default, Reflect, GizmoConfigGroup)]
struct ProcanimGizmoGroup;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (
            update_root_position,
            update_procedural_animations,
            draw_anim_gizmos,
        )
            .chain(),
    )
    .init_gizmo_group::<ProcanimGizmoGroup>()
    .add_systems(OnEnter(Screen::Playing), spawn_procedural_item);
}

pub fn update_procedural_animations(
    roots: Query<(&AnimationRoot, &AnimationNode, &Transform), Without<AnimationChild>>,
    mut nodes: Query<(&AnimationNode, &mut Transform), With<AnimationChild>>,
) {
    for (root, root_node, root_tx) in roots.iter() {
        let mut parent_pos = root_tx.translation;
        let mut parent_radius = root_node.radius;

        for node_entity in root.children.iter() {
            if let Ok((node, mut tx)) = nodes.get_mut(*node_entity) {
                let delta = tx.translation - parent_pos;
                tx.translation = parent_pos + delta.normalize_or_zero() * parent_radius;

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

    let children = (0..20)
        .map(|idx| {
            let idx = idx as f32 + 1.;

            commands
                .spawn((
                    SpatialBundle::from_transform(Transform::from_xyz(idx * 20., idx * 20., 0.)),
                    AnimationNode { radius: 20. },
                    AnimationChild,
                ))
                .id()
        })
        .collect::<Vec<_>>();

    commands.spawn((
        SpatialBundle::default(),
        AnimationRoot { children },
        AnimationNode { radius: 50. },
    ));
}

fn draw_anim_gizmos(
    mut gizmos: Gizmos<ProcanimGizmoGroup>,
    nodes: Query<(&AnimationNode, &Transform, Option<&AnimationRoot>)>,
) {
    for (node, tx, root) in nodes.iter() {
        gizmos.circle_2d(tx.translation.truncate(), 2.0, WHITE);
        gizmos.circle_2d(
            tx.translation.truncate(),
            node.radius,
            if root.is_some() { BLUE } else { YELLOW },
        );
    }
}

fn update_root_position(
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<IsDefaultUiCamera>>,
    mut roots: Query<&mut Transform, With<AnimationRoot>>,
) {
    let (camera, camera_transform) = cameras.single();

    if let Some(position) = windows
        .single()
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor))
        .map(|ray| ray.origin.truncate())
    {
        for mut tx in roots.iter_mut() {
            tx.translation = position.extend(0.);
        }
    }
}
