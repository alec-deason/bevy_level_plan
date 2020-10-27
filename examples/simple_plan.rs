use std::time::Duration;

use rand::Rng;

use bevy::{prelude::*, render::camera::OrthographicProjection, sprite::collide_aabb::collide};

use bevy_level_plan::{
    level_plan_system, Conditional, Cycle, LevelPlan, LevelPlanElement, Sequence, SetComponent,
    While, Nop, LevelContext,
};

/// LevelPlan related stuff

fn make_level_plan(level_length: f32) -> LevelPlan<ExampleLevelContext> {
    LevelPlan::<ExampleLevelContext>::new(
        Sequence::default()
            .push(ForDistance::new(
                level_length - 1800.0,
                Cycle::new(
                    Sequence::default()
                        .push(ForDistance::new(
                            500.0,
                            SetComponent::new(DiverSpawner::default()),
                        ))
                        .push(ForDistance::new(
                            500.0,
                            SetComponent::new(SwooperSpawner::default()),
                        )),
                ),
            ))
            .push(ForDistance::new(
                1000.0,
                Nop
            ))
            .push(Conditional::<ExampleLevelContext>::new(
                move |context| context.player_health < 4,
                SpawnPowerups,
            ))
            .push(While::<ExampleLevelContext>::new(
                |context| context.boss_spawned,
                SpawnBoss,
            ))
            .push(YouWin),
    )
}

struct ExampleLevelContext {
    player_loc: Vec3,
    player_health: u32,
    boss_spawned: bool,
}
impl LevelContext for ExampleLevelContext {
    fn build(world: &World, _resources: &Resources) -> Self {
        let mut player_loc = Vec3::zero();
        let mut player_health = 4;
        if let Some((_, transform, health)) = world
            .query::<(&Player, &Transform, &Health)>()
            .iter()
            .next()
        {
            player_loc = transform.translation();
            player_health = health.0;
        }
        Self {
            player_loc,
            player_health,
            boss_spawned: world.query::<&Boss>().iter().count() > 0,
        }
    }
}

pub struct ForDistance {
    length: f32,
    start: f32,
    element: Box<dyn LevelPlanElement<ExampleLevelContext>>,
}
impl ForDistance {
    fn new(length: f32, element: impl LevelPlanElement<ExampleLevelContext> + 'static) -> Self {
        Self {
            length,
            start: f32::MAX,
            element: Box::new(element),
        }
    }
}
impl LevelPlanElement<ExampleLevelContext> for ForDistance {
    fn step(&mut self, level: Entity, commands: &mut Commands, context: &mut ExampleLevelContext) -> bool {
        if context.player_loc.y() < self.start + self.length {
            self.element.step(level, commands, context)
        } else {
            false
        }
    }

    fn activate(&mut self, level: Entity, commands: &mut Commands, context: &mut ExampleLevelContext) {
        self.start = context.player_loc.y();
        self.element.activate(level, commands, context);
    }

    fn deactivate(&mut self, level: Entity, commands: &mut Commands, context: &mut ExampleLevelContext) {
        self.element.deactivate(level, commands, context);
    }
}

struct SpawnPowerups;
impl LevelPlanElement<ExampleLevelContext> for SpawnPowerups {
    fn step(
        &mut self,
        _level: Entity,
        _commands: &mut Commands,
        _context: &mut ExampleLevelContext,
    ) -> bool {
        false
    }

    fn activate(&mut self, _level: Entity, commands: &mut Commands, _context: &mut ExampleLevelContext) {
        for _ in 0..3 {
            commands.spawn((Powerup,));
        }
    }
}

struct SpawnBoss;
impl<T> LevelPlanElement<T> for SpawnBoss {
    fn activate(&mut self, _level: Entity, commands: &mut Commands, _context: &mut T) {
        commands.spawn((Boss,));
    }
}

struct YouWin;
impl<T> LevelPlanElement<T> for YouWin {
    fn activate(&mut self, _level: Entity, _commands: &mut Commands, _context: &mut T) {
        println!("You win!");
        std::process::exit(0);
    }
}

#[derive(Clone)]
struct DiverSpawner(Timer);
impl Default for DiverSpawner {
    fn default() -> Self {
        Self(Timer::new(Duration::from_secs_f32(0.5), true))
    }
}

fn diver_spawner(
    mut commands: Commands,
    bounds: Res<LevelBounds>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    time: Res<Time>,
    main_camera: Res<MainCamera>,
    mut spawner: Mut<DiverSpawner>,
    _level: &LevelPlan<ExampleLevelContext>,
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
            .with(Velocity(Vec2::new(0.0, -500.0), true))
            .with(Health(1))
            .with(Enemy);
    }
}

#[derive(Clone)]
struct SwooperSpawner(Timer);
impl Default for SwooperSpawner {
    fn default() -> Self {
        Self(Timer::new(Duration::from_secs_f32(0.25), true))
    }
}

fn swooper_spawner(
    mut commands: Commands,
    bounds: Res<LevelBounds>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    time: Res<Time>,
    main_camera: Res<MainCamera>,
    mut spawner: Mut<SwooperSpawner>,
    _level: &LevelPlan<ExampleLevelContext>,
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
            .with(Velocity(Vec2::new(vx, 0.0), true))
            .with(Health(1))
            .with(Enemy);
    }
}

/// General game stuff

fn main() {
    App::build()
        .add_default_plugins()
        .add_system(level_plan_system::<ExampleLevelContext>.thread_local_system())
        .add_system(player_controls.system())
        .add_system(movement.system())
        .add_system(diver_spawner.system())
        .add_system(swooper_spawner.system())
        .add_system(boss_spawner.system())
        .add_system(collision.system())
        .add_system(death_monitor.system())
        .add_system(powerup_pickup.system())
        .add_system(powerup_spawner.system())
        .add_system(flash.system())
        .add_system_to_stage(stage::POST_UPDATE, start_flash.system())
        .add_system(player_health_ui.system())
        .add_system(camera_system.system())
        .add_startup_system(setup.system())
        .run();
}

struct Player;
struct Health(u32);
struct HealthFlash(u32);
struct Enemy;
struct Velocity(Vec2, bool);
struct Boss;
struct Powerup;
#[derive(Clone)]
struct LevelBounds(Rect<f32>);

fn setup(mut commands: Commands, mut materials: ResMut<Assets<ColorMaterial>>) {
    let bounds = Rect {
        left: -500.0,
        right: 500.0,
        bottom: 0.0,
        top: 5000.0,
    };
    make_backdrop(&mut commands, &mut materials, bounds);
    make_health_ui(&mut commands, &mut materials);
    make_player(&mut commands, &mut materials, bounds);
    commands
        .spawn(UiCameraComponents::default())
        .insert_resource(MainCamera::default())
        .spawn((make_level_plan(bounds.top - bounds.bottom),))
        .insert_resource(LevelBounds(bounds));
}

fn make_health_ui(commands: &mut Commands, materials: &mut Assets<ColorMaterial>) {
    commands
        .spawn(NodeComponents {
            style: Style {
                size: Size::new(Val::Px(98.0), Val::Px(98.0)),
                position: Rect {
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    ..Default::default()
                },
                position_type: PositionType::Absolute,
                ..Default::default()
            },
            material: materials.add(Color::NONE.into()),
            ..Default::default()
        })
        .with_children(|parent| {
            parent
                .spawn(NodeComponents {
                    style: Style {
                        size: Size::new(Val::Percent(50.0), Val::Percent(100.0)),
                        flex_direction: FlexDirection::Column,
                        ..Default::default()
                    },
                    material: materials.add(Color::NONE.into()),
                    ..Default::default()
                })
                .with_children(|parent| {
                    parent
                        .spawn(NodeComponents {
                            style: Style {
                                size: Size::new(Val::Percent(100.0), Val::Percent(50.0)),
                                margin: Rect::all(Val::Px(2.0)),
                                ..Default::default()
                            },
                            material: materials.add(Color::rgb(1.0, 0.0, 0.0).into()),
                            ..Default::default()
                        })
                        .with(HealthUi(2))
                        .spawn(NodeComponents {
                            style: Style {
                                size: Size::new(Val::Percent(100.0), Val::Percent(50.0)),
                                margin: Rect::all(Val::Px(2.0)),
                                ..Default::default()
                            },
                            material: materials.add(Color::rgb(1.0, 0.0, 0.0).into()),
                            ..Default::default()
                        })
                        .with(HealthUi(4));
                })
                .spawn(NodeComponents {
                    style: Style {
                        size: Size::new(Val::Percent(50.0), Val::Percent(100.0)),
                        flex_direction: FlexDirection::Column,
                        ..Default::default()
                    },
                    material: materials.add(Color::NONE.into()),
                    ..Default::default()
                })
                .with_children(|parent| {
                    parent
                        .spawn(NodeComponents {
                            style: Style {
                                size: Size::new(Val::Percent(100.0), Val::Percent(50.0)),
                                margin: Rect::all(Val::Px(2.0)),
                                ..Default::default()
                            },
                            material: materials.add(Color::rgb(1.0, 0.0, 0.0).into()),
                            ..Default::default()
                        })
                        .with(HealthUi(1))
                        .spawn(NodeComponents {
                            style: Style {
                                size: Size::new(Val::Percent(100.0), Val::Percent(50.0)),
                                margin: Rect::all(Val::Px(2.0)),
                                ..Default::default()
                            },
                            material: materials.add(Color::rgb(1.0, 0.0, 0.0).into()),
                            ..Default::default()
                        })
                        .with(HealthUi(3));
                });
        });
}

fn make_player(commands: &mut Commands, materials: &mut Assets<ColorMaterial>, bounds: Rect<f32>) {
    commands
        .spawn(SpriteComponents {
            material: materials.add(Color::rgba(1.0, 0.0, 0.0, 1.0).into()),
            sprite: Sprite::new(Vec2::new(32.0, 32.0)),
            transform: Transform::from_translation(Vec3::new(0.0, bounds.bottom + 32.0, 1.0)),
            draw: Draw {
                is_transparent: true,
                ..Default::default()
            },
            ..Default::default()
        })
        .with(Player)
        .with(Health(4))
        .with(HealthFlash(0))
        .with(Velocity(Vec2::zero(), false))
        .with_children(|parent| {
            parent.spawn(Camera2dComponents::default());
        });
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

fn player_controls(
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

fn movement(
    bounds: Res<LevelBounds>,
    time: Res<Time>,
    mut velocity: Mut<Velocity>,
    sprite: &Sprite,
    mut health: Mut<Health>,
    maybe_flash: Option<&HealthFlash>,
    mut transform: Mut<Transform>,
) {
    let motion = velocity.0 * time.delta_seconds;
    let translation = transform.translation_mut();

    let mut out_of_bounds_x = false;
    let mut out_of_bounds_y = false;
    *translation.x_mut() += motion.x();
    if translation.x() < bounds.0.left + sprite.size.x() / 2.0 {
        out_of_bounds_x = true;
        *translation.x_mut() = bounds.0.left + sprite.size.x() / 2.0;
    } else if translation.x() > bounds.0.right - sprite.size.x() / 2.0 {
        out_of_bounds_x = true;
        *translation.x_mut() = bounds.0.right - sprite.size.x() / 2.0;
    }

    *translation.y_mut() += motion.y();
    if translation.y() > bounds.0.top - sprite.size.y() / 2.0 {
        out_of_bounds_y = true;
        *translation.y_mut() = bounds.0.top - sprite.size.y() / 2.0;
    } else if translation.y() < bounds.0.bottom + sprite.size.y() / 2.0 {
        out_of_bounds_y = true;
        *translation.y_mut() = bounds.0.bottom + sprite.size.y() / 2.0;
    }
    if velocity.1 && (out_of_bounds_x || out_of_bounds_y) {
        let mut is_invincible = false;
        if let Some(flash) = maybe_flash {
            if flash.0 > 0 {
                is_invincible = true;
            }
        }
        if !is_invincible {
            if let Some(h) = health.0.checked_sub(1) {
                health.0 = h;
            }
        }
        if out_of_bounds_x {
            *velocity.0.x_mut() *= -1.0;
        }
        if out_of_bounds_y {
            *velocity.0.y_mut() *= -1.0;
        }
    }
}

fn powerup_spawner(
    mut commands: Commands,
    bounds: Res<LevelBounds>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    main_camera: Res<MainCamera>,
    mut powerup_query: Query<Without<Sprite, (Entity, &Powerup)>>,
) {
    let mut rng = rand::thread_rng();
    for (entity, _) in &mut powerup_query.iter() {
        let x = rng.gen_range(bounds.0.left + 16.0, bounds.0.right - 16.0);
        let y = rng.gen_range(
            main_camera.0.translation().y() + main_camera.1.bottom,
            main_camera.0.translation().y() + main_camera.1.top,
        );
        commands.insert(
            entity,
            SpriteComponents {
                material: materials.add(Color::rgba(1.0, 0.0, 0.0, 1.0).into()),
                sprite: Sprite::new(Vec2::new(32.0, 32.0)),
                transform: Transform::from_translation(Vec3::new(x, y, 1.0)),
                ..Default::default()
            },
        );
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
            .insert(
                entity,
                (
                    Velocity(Vec2::new(300.0, -300.0), true),
                    Enemy,
                    Health(10),
                    HealthFlash(0),
                ),
            )
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

struct HealthUi(u32);
fn player_health_ui(
    mut player_query: Query<(&Player, &Health)>,
    mut ui_query: Query<(&HealthUi, &mut Draw)>,
) {
    if let Some((_player, health)) = player_query.iter().iter().next() {
        for (ui, mut draw) in &mut ui_query.iter() {
            if ui.0 > health.0 {
                draw.is_visible = false;
            } else {
                draw.is_visible = true;
            }
        }
    }
}

fn collision(
    mut player_query: Query<(&Player, &mut Health, &HealthFlash, &Transform, &Sprite)>,
    mut enemy_query: Query<(
        &Enemy,
        &mut Health,
        Option<&HealthFlash>,
        &Transform,
        &Sprite,
    )>,
) {
    if let Some((_player, mut player_health, player_flash, player_transform, player_sprite)) =
        player_query.iter().iter().next()
    {
        for (_, mut enemy_health, maybe_enemy_flash, enemy_transform, enemy_sprite) in
            &mut enemy_query.iter()
        {
            let collision = collide(
                enemy_transform.translation(),
                enemy_sprite.size,
                player_transform.translation(),
                player_sprite.size,
            );
            if collision.is_some() {
                if player_flash.0 == 0 {
                    if let Some(h) = player_health.0.checked_sub(1) {
                        player_health.0 = h;
                    }
                }
                let mut is_invincible = false;
                if let Some(flash) = maybe_enemy_flash {
                    if flash.0 > 0 {
                        is_invincible = true;
                    }
                }
                if !is_invincible {
                    if let Some(h) = enemy_health.0.checked_sub(1) {
                        enemy_health.0 = h;
                    }
                }
            }
        }
    }
}

fn death_monitor(
    mut commands: Commands,
    entity: Entity,
    health: &Health,
    maybe_player: Option<&Player>,
) {
    if health.0 == 0 {
        commands.despawn(entity);
        if maybe_player.is_some() {
            println!("You lose");
            std::process::exit(0);
        }
    }
}

fn start_flash(_health: Changed<Health>, mut flash: Mut<HealthFlash>) {
    flash.0 = 10;
}

fn flash(
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut flash: Mut<HealthFlash>,
    material: &Handle<ColorMaterial>,
    _draw: Mut<Draw>,
) {
    if flash.0 > 0 {
        flash.0 -= 1;
        if let Some(mut material) = materials.get_mut(material) {
            material.color.a = 0.1;
        }
    } else if let Some(mut material) = materials.get_mut(material) {
        material.color.a = 1.0;
    }
}

fn powerup_pickup(
    mut commands: Commands,
    mut player_query: Query<(&Player, &mut Health, &Transform, &Sprite)>,
    mut powerup_query: Query<(Entity, &Powerup, &Transform, &Sprite)>,
) {
    if let Some((_player, mut player_health, player_transform, player_sprite)) =
        player_query.iter().iter().next()
    {
        for (powerup_entity, _, powerup_transform, powerup_sprite) in &mut powerup_query.iter() {
            let collision = collide(
                powerup_transform.translation(),
                powerup_sprite.size,
                player_transform.translation(),
                player_sprite.size,
            );
            if collision.is_some() {
                player_health.0 = (player_health.0 + 1).min(4);
                commands.despawn(powerup_entity);
            }
        }
    }
}
