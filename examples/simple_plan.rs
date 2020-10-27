use std::time::Duration;

use rand::Rng;

use bevy::{prelude::*, render::camera::OrthographicProjection};

use bevy_level_plan::{
    level_progress_system, LevelPlan, LevelPlanElement, Nop, Sequence, SetComponent, While,
};

fn main() {
    App::build()
        .add_default_plugins()
        .add_system(level_progress_system::<LevelContext>.thread_local_system())
        .add_system(player_system.system())
        .add_system(velocity_system.system())
        .add_system(diver_spawner.system())
        .add_system(swooper_spawner.system())
        .add_system(boss_spawner.system())
        .add_system(boss_system.system())
        .add_system(camera_system.system())
        .add_startup_system(setup.system())
        .run();
}

struct Player;
struct Velocity(Vec2, bool);
struct Boss;
struct LevelBounds(Rect<f32>);

struct LevelContext {
    player_loc: Vec3,
    camera: MainCamera,
}
impl bevy_level_plan::LevelContext for LevelContext {
    fn build(world: &World, resources: &Resources) -> Self {
        let mut player_loc = Vec3::zero();
        if let Some((_, transform)) = world.query::<(&Player, &Transform)>().iter().next() {
            player_loc = transform.translation();
        }
        Self {
            player_loc,
            camera: (*resources.get::<MainCamera>().unwrap()).clone(),
        }
    }
}

fn setup(mut commands: Commands, mut materials: ResMut<Assets<ColorMaterial>>) {
    let bounds = Rect {
        left: -500.0,
        right: 500.0,
        bottom: 0.0,
        top: 3000.0,
    };
    make_backdrop(&mut commands, &mut materials, bounds);
    commands
        .spawn(SpriteComponents {
            material: materials.add(Color::rgb(1.0, 0.0, 0.0).into()),
            sprite: Sprite::new(Vec2::new(32.0, 32.0)),
            transform: Transform::from_translation(Vec3::new(0.0, bounds.bottom + 32.0, 1.0)),
            ..Default::default()
        })
        .with(Player)
        .with(Velocity(Vec2::zero(), false))
        .with_children(|parent| {
            parent.spawn(Camera2dComponents::default());
        })
        .insert_resource(MainCamera::default())
        .insert_resource(LevelBounds(bounds))
        .spawn((LevelPlan::<LevelContext>::new(Sequence::new(vec![
            Box::new(While::<LevelContext>::new(
                move |context| context.player_loc.y() < bounds.bottom + 1000.0,
                SetComponent::new(DiverSpawner::default()),
            )),
            Box::new(While::<LevelContext>::new(
                move |context| context.player_loc.y() < bounds.top - 800.0,
                SetComponent::new(SwooperSpawner::default()),
            )),
            Box::new(While::<LevelContext>::new(
                move |context| {
                    context.camera.0.translation().y() + context.camera.1.top < bounds.top - 200.0
                },
                Nop,
            )),
            Box::new(SpawnBoss),
        ])),));
}

struct SpawnBoss;
impl LevelPlanElement<LevelContext> for SpawnBoss {
    fn activate(&mut self, _level: Entity, commands: &mut Commands, _context: &mut LevelContext) {
        commands.spawn((Boss,));
    }
}

fn make_backdrop(
    commands: &mut Commands,
    materials: &mut Assets<ColorMaterial>,
    bounds: Rect<f32>,
) {
    let white_material = materials.add(Color::rgb(1.0, 1.0, 1.0).into());
    let gray_material = materials.add(Color::rgb(0.9, 0.9, 0.9).into());

    let dy = (bounds.top - bounds.bottom) / 100.0;
    for i in 0..100 {
        let material = if i % 2 == 0 {
            white_material
        } else {
            gray_material
        };
        commands.spawn(SpriteComponents {
            material,
            sprite: Sprite::new(Vec2::new(bounds.right - bounds.left, dy)),
            transform: Transform::from_translation(Vec3::new(
                0.0,
                bounds.bottom + dy * i as f32 + dy / 2.0,
                1.0,
            )),
            ..Default::default()
        });
    }
}

fn player_system(
    keyboard_input: Res<Input<KeyCode>>,
    _player: &Player,
    mut velocity: Mut<Velocity>,
) {
    let mut direction = Vec2::splat(0.0);
    if keyboard_input.pressed(KeyCode::Left) {
        *direction.x_mut() -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::Right) {
        *direction.x_mut() += 1.0;
    }
    if keyboard_input.pressed(KeyCode::Up) {
        *direction.y_mut() += 1.0;
    }
    if keyboard_input.pressed(KeyCode::Down) {
        *direction.y_mut() -= 1.0;
    }
    velocity.0 = direction * 600.0;
}

fn velocity_system(
    mut commands: Commands,
    bounds: Res<LevelBounds>,
    time: Res<Time>,
    entity: Entity,
    velocity: &Velocity,
    sprite: &Sprite,
    mut transform: Mut<Transform>,
) {
    let motion = velocity.0 * time.delta_seconds;
    let translation = transform.translation_mut();

    let mut out_of_bounds = false;
    *translation.x_mut() += motion.x();
    if translation.x() < bounds.0.left + sprite.size.x() / 2.0 {
        out_of_bounds = true;
        *translation.x_mut() = bounds.0.left + sprite.size.x() / 2.0;
    } else if translation.x() > bounds.0.right - sprite.size.x() / 2.0 {
        out_of_bounds = true;
        *translation.x_mut() = bounds.0.right - sprite.size.x() / 2.0;
    }

    *translation.y_mut() += motion.y();
    if translation.y() > bounds.0.top - sprite.size.y() / 2.0 {
        out_of_bounds = true;
        *translation.y_mut() = bounds.0.top - sprite.size.y() / 2.0;
    } else if translation.y() < bounds.0.bottom + sprite.size.y() / 2.0 {
        out_of_bounds = true;
        *translation.y_mut() = bounds.0.bottom + sprite.size.y() / 2.0;
    }
    if velocity.1 && out_of_bounds {
        commands.despawn(entity);
    }
}

#[derive(Clone)]
struct DiverSpawner(Timer);
impl Default for DiverSpawner {
    fn default() -> Self {
        Self(Timer::new(Duration::from_secs_f32(1.0), true))
    }
}

fn diver_spawner(
    mut commands: Commands,
    bounds: Res<LevelBounds>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    time: Res<Time>,
    main_camera: Res<MainCamera>,
    mut spawner: Mut<DiverSpawner>,
    _level: &LevelPlan<LevelContext>,
) {
    spawner.0.tick(time.delta_seconds);
    if spawner.0.finished {
        let x = rand::thread_rng().gen_range(bounds.0.left + 16.0, bounds.0.right - 16.0);
        let y =
            (main_camera.0.translation().y() + main_camera.1.top + 50.0).min(bounds.0.top - 16.0);
        commands
            .spawn(SpriteComponents {
                material: materials.add(Color::rgb(0.0, 1.0, 0.0).into()),
                sprite: Sprite::new(Vec2::new(32.0, 32.0)),
                transform: Transform::from_translation(Vec3::new(x, y, 1.0)),
                ..Default::default()
            })
            .with(Velocity(Vec2::new(0.0, -500.0), true));
    }
}

#[derive(Clone)]
struct SwooperSpawner(Timer);
impl Default for SwooperSpawner {
    fn default() -> Self {
        Self(Timer::new(Duration::from_secs_f32(0.5), true))
    }
}

fn swooper_spawner(
    mut commands: Commands,
    bounds: Res<LevelBounds>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    time: Res<Time>,
    main_camera: Res<MainCamera>,
    mut spawner: Mut<SwooperSpawner>,
    _level: &LevelPlan<LevelContext>,
) {
    spawner.0.tick(time.delta_seconds);
    if spawner.0.finished {
        let (x, vx) = if rand::random() {
            (bounds.0.right - 16.0, -500.0)
        } else {
            (bounds.0.left + 16.0, 500.0)
        };
        let y = rand::thread_rng().gen_range(
            main_camera.0.translation().y() + main_camera.1.bottom + 16.0,
            main_camera.0.translation().y() + main_camera.1.top - 16.0,
        );
        commands
            .spawn(SpriteComponents {
                material: materials.add(Color::rgb(0.0, 0.0, 1.0).into()),
                sprite: Sprite::new(Vec2::new(32.0, 32.0)),
                transform: Transform::from_translation(Vec3::new(x, y, 1.0)),
                ..Default::default()
            })
            .with(Velocity(Vec2::new(vx, 0.0), true));
    }
}

fn boss_spawner(
    mut commands: Commands,
    mut bounds: ResMut<LevelBounds>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    main_camera: Res<MainCamera>,
    mut boss_query: Query<Without<Sprite, (Entity, &Boss)>>,
) {
    for (entity, _) in &mut boss_query.iter() {
        commands
            .insert(
                entity,
                SpriteComponents {
                    material: materials.add(Color::rgb(0.0, 0.0, 0.0).into()),
                    sprite: Sprite::new(Vec2::new(128.0, 128.0)),
                    transform: Transform::from_translation(Vec3::new(
                        0.0,
                        bounds.0.top - 128.0,
                        1.0,
                    )),
                    ..Default::default()
                },
            )
            .insert_one(entity, Velocity(Vec2::new(200.0, 0.0), true))
            .spawn(SpriteComponents {
                material: materials.add(Color::rgb(0.0, 0.0, 0.0).into()),
                sprite: Sprite::new(Vec2::new(bounds.0.right - bounds.0.left, 64.0)),
                transform: Transform::from_translation(Vec3::new(
                    0.0,
                    main_camera.0.translation().y() + main_camera.1.bottom + 200.0,
                    1.0,
                )),
                ..Default::default()
            });
        bounds.0.bottom = main_camera.0.translation().y() + main_camera.1.bottom + 232.0;
    }
}

fn boss_system(
    bounds: Res<LevelBounds>,
    transform: &Transform,
    sprite: &Sprite,
    mut velocity: Mut<Velocity>,
    _boss: &Boss,
) {
    if transform.translation().x() > bounds.0.right - sprite.size.x() / 2.0 - 16.0 {
        *velocity.0.x_mut() = -200.0;
    } else if transform.translation().x() < bounds.0.left + sprite.size.x() / 2.0 + 16.0 {
        *velocity.0.x_mut() = 200.0;
    }
}

#[derive(Clone, Default)]
struct MainCamera(Transform, OrthographicProjection);
fn camera_system(
    mut main_camera: ResMut<MainCamera>,
    projection: &OrthographicProjection,
    transform: &GlobalTransform,
) {
    main_camera.0 = Transform::new(*transform.value());
    main_camera.1 = projection.clone();
}
