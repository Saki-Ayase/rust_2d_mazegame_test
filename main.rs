use bevy::prelude::*;
use rand::{thread_rng};
use rand::prelude::SliceRandom;
use std::time::Instant;

const TILE_SIZE: f32 = 32.0;
const MAZE_WIDTH: usize = 21;
const MAZE_HEIGHT: usize = 21;

#[derive(Clone, Copy, PartialEq)]
enum TileType {
    Wall,
    Path,
}

#[derive(Resource)]
struct Maze(Vec<Vec<TileType>>);

#[derive(Resource)]
struct PlayerPosition(usize, usize);

#[derive(Resource)]
struct GoalPosition(usize, usize);

#[derive(Resource)]
struct MoveTimer(Timer);

#[derive(Resource)]
struct StartTime(Instant);

#[derive(Component)]
struct Player;

#[derive(Component)]
struct Goal;

#[derive(Component)]
struct WinText;

#[derive(Component)]
struct MazeTile;

#[derive(Component)]
struct RestartButton;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(PlayerPosition(1, 1))
        .insert_resource(generate_maze())
        .insert_resource(MoveTimer(Timer::from_seconds(0.12, TimerMode::Repeating)))
        .insert_resource(StartTime(Instant::now()))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Rust Maze Escape".into(),
                resolution: (MAZE_WIDTH as f32 * TILE_SIZE, MAZE_HEIGHT as f32 * TILE_SIZE).into(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(Update, player_input)
        .add_systems(Update, restart_button_system)
        .run();
}

fn generate_maze() -> Maze {
    let mut maze = vec![vec![TileType::Wall; MAZE_WIDTH]; MAZE_HEIGHT];

    fn carve(x: usize, y: usize, maze: &mut Vec<Vec<TileType>>) {
        let mut rng = thread_rng();
        let mut dirs = vec![(2, 0), (-2, 0), (0, 2), (0, -2)];
        dirs.shuffle(&mut rng);

        for (dx, dy) in dirs {
            let nx = x as isize + dx;
            let ny = y as isize + dy;

            if nx > 0 && ny > 0 && (nx as usize) < MAZE_WIDTH - 1 && (ny as usize) < MAZE_HEIGHT - 1 {
                if maze[ny as usize][nx as usize] == TileType::Wall {
                    maze[ny as usize][nx as usize] = TileType::Path;
                    maze[(y as isize + dy / 2) as usize][(x as isize + dx / 2) as usize] = TileType::Path;
                    carve(nx as usize, ny as usize, maze);
                }
            }
        }
    }

    maze[1][1] = TileType::Path;
    carve(1, 1, &mut maze);
    Maze(maze)
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    maze: Res<Maze>,
) {
    commands.spawn(Camera2dBundle::default());

    let mut goal_pos = (MAZE_WIDTH - 2, MAZE_HEIGHT - 2);
    'outer: for y in (1..MAZE_HEIGHT - 1).rev() {
        for x in (1..MAZE_WIDTH - 1).rev() {
            if maze.0[y][x] == TileType::Path {
                goal_pos = (x, y);
                break 'outer;
            }
        }
    }
    commands.insert_resource(GoalPosition(goal_pos.0, goal_pos.1));

    for (y, row) in maze.0.iter().enumerate() {
        for (x, &tile) in row.iter().enumerate() {
            let color = match tile {
                TileType::Wall => Color::DARK_GRAY,
                TileType::Path => Color::WHITE,
            };

            commands.spawn((
                SpriteBundle {
                    sprite: Sprite {
                        color,
                        custom_size: Some(Vec2::splat(TILE_SIZE)),
                        ..default()
                    },
                    transform: Transform::from_translation(Vec3::new(
                        x as f32 * TILE_SIZE - (MAZE_WIDTH as f32 / 2.0 * TILE_SIZE),
                        y as f32 * TILE_SIZE - (MAZE_HEIGHT as f32 / 2.0 * TILE_SIZE),
                        0.0,
                    )),
                    ..default()
                },
                MazeTile,
            ));
        }
    }

    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::GREEN,
                custom_size: Some(Vec2::splat(TILE_SIZE)),
                ..default()
            },
            transform: Transform::from_translation(Vec3::new(
                goal_pos.0 as f32 * TILE_SIZE - (MAZE_WIDTH as f32 / 2.0 * TILE_SIZE),
                goal_pos.1 as f32 * TILE_SIZE - (MAZE_HEIGHT as f32 / 2.0 * TILE_SIZE),
                0.5,
            )),
            ..default()
        },
        Goal,
    ));

    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::RED,
                custom_size: Some(Vec2::splat(TILE_SIZE * 0.8)),
                ..default()
            },
            transform: Transform::from_translation(Vec3::new(
                1.0 * TILE_SIZE - (MAZE_WIDTH as f32 / 2.0 * TILE_SIZE),
                1.0 * TILE_SIZE - (MAZE_HEIGHT as f32 / 2.0 * TILE_SIZE),
                1.0,
            )),
            ..default()
        },
        Player,
    ));
}

fn player_input(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut move_timer: ResMut<MoveTimer>,
    mut player_query: Query<(Entity, &mut Transform), With<Player>>,
    mut pos: ResMut<PlayerPosition>,
    maze: Res<Maze>,
    goal: Res<GoalPosition>,
    win_text_query: Query<Entity, With<WinText>>,
    asset_server: Res<AssetServer>,
    maze_entities: Query<Entity, With<MazeTile>>,
    goal_query: Query<Entity, With<Goal>>,
    start_time: Res<StartTime>,
) {
    if !win_text_query.is_empty() {
        return;
    }

    if !move_timer.0.tick(time.delta()).just_finished() {
        return;
    }

    let mut dx = 0;
    let mut dy = 0;

    if keys.pressed(KeyCode::ArrowUp) {
        dy += 1;
    } else if keys.pressed(KeyCode::ArrowDown) {
        dy -= 1;
    } else if keys.pressed(KeyCode::ArrowLeft) {
        dx -= 1;
    } else if keys.pressed(KeyCode::ArrowRight) {
        dx += 1;
    }

    if dx == 0 && dy == 0 {
        return;
    }

    let new_x = (pos.0 as isize + dx) as usize;
    let new_y = (pos.1 as isize + dy) as usize;

    if new_x < MAZE_WIDTH && new_y < MAZE_HEIGHT && maze.0[new_y][new_x] == TileType::Path {
        pos.0 = new_x;
        pos.1 = new_y;

        for (_, mut transform) in player_query.iter_mut() {
            transform.translation = Vec3::new(
                new_x as f32 * TILE_SIZE - (MAZE_WIDTH as f32 / 2.0 * TILE_SIZE),
                new_y as f32 * TILE_SIZE - (MAZE_HEIGHT as f32 / 2.0 * TILE_SIZE),
                1.0,
            );
        }

        if pos.0 == goal.0 && pos.1 == goal.1 && win_text_query.is_empty() {
            let elapsed = start_time.0.elapsed();
            let seconds = elapsed.as_secs();
            let millis = elapsed.subsec_millis();

            for (entity, _) in player_query.iter_mut() {
                commands.entity(entity).despawn_recursive();
            }
            for entity in maze_entities.iter() {
                commands.entity(entity).despawn_recursive();
            }
            for entity in goal_query.iter() {
                commands.entity(entity).despawn_recursive();
            }

            commands.spawn((
                TextBundle::from_section(
                    format!("You win!\nTime: {}.{:03} seconds", seconds, millis),
                    TextStyle {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                        font_size: 48.0,
                        color: Color::GOLD,
                    },
                ).with_style(Style {
                    position_type: PositionType::Absolute,
                    top: Val::Percent(35.0),
                    left: Val::Percent(30.0),
                    ..default()
                }),
                WinText,
            ));

            commands.spawn((
                ButtonBundle {
                    style: Style {
                        width: Val::Px(150.0),
                        height: Val::Px(65.0),
                        margin: UiRect::all(Val::Auto),
                        position_type: PositionType::Absolute,
                        top: Val::Percent(60.0),
                        left: Val::Percent(37.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    background_color: BackgroundColor(Color::DARK_GRAY),
                    ..default()
                },
                RestartButton,
            ));

            commands.spawn(TextBundle::from_section(
                "Restart",
                TextStyle {
                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    font_size: 40.0,
                    color: Color::WHITE,
                },
            ).with_style(Style {
                position_type: PositionType::Absolute,
                top: Val::Percent(63.0),
                left: Val::Percent(40.0),
                margin: UiRect::all(Val::Auto),
                ..default()
            }));
        }
    }
}

fn restart_button_system(
    mut commands: Commands,
    interaction_query: Query<(&Interaction, Entity), (Changed<Interaction>, With<RestartButton>)>,
    mut maze_res: ResMut<Maze>,
    mut player_pos_res: ResMut<PlayerPosition>,
    mut goal_pos_res: ResMut<GoalPosition>,
    win_text_query: Query<Entity, With<WinText>>,
    maze_entities: Query<Entity, With<MazeTile>>,
    goal_query: Query<Entity, With<Goal>>,
    restart_button_query: Query<Entity, With<RestartButton>>,
    text_query: Query<Entity, With<Text>>,
    mut start_time: ResMut<StartTime>,
) {
    for (interaction, _entity) in &interaction_query {
        if *interaction == Interaction::Pressed {
            for entity in win_text_query.iter() {
                commands.entity(entity).despawn_recursive();
            }
            for entity in restart_button_query.iter() {
                commands.entity(entity).despawn_recursive();
            }
            for entity in text_query.iter() {
                commands.entity(entity).despawn_recursive();
            }
            for entity in maze_entities.iter() {
                commands.entity(entity).despawn_recursive();
            }
            for entity in goal_query.iter() {
                commands.entity(entity).despawn_recursive();
            }

            let new_maze = generate_maze();
            *maze_res = new_maze;

            player_pos_res.0 = 1;
            player_pos_res.1 = 1;

            let maze = &maze_res.0;
            let mut goal_pos = (MAZE_WIDTH - 2, MAZE_HEIGHT - 2);
            'outer: for y in (1..MAZE_HEIGHT - 1).rev() {
                for x in (1..MAZE_WIDTH - 1).rev() {
                    if maze[y][x] == TileType::Path {
                        goal_pos = (x, y);
                        break 'outer;
                    }
                }
            }
            goal_pos_res.0 = goal_pos.0;
            goal_pos_res.1 = goal_pos.1;

            for y in 0..MAZE_HEIGHT {
                for x in 0..MAZE_WIDTH {
                    let color = match maze[y][x] {
                        TileType::Wall => Color::DARK_GRAY,
                        TileType::Path => Color::WHITE,
                    };
                    commands.spawn((
                        SpriteBundle {
                            sprite: Sprite {
                                color,
                                custom_size: Some(Vec2::splat(TILE_SIZE)),
                                ..default()
                            },
                            transform: Transform::from_translation(Vec3::new(
                                x as f32 * TILE_SIZE - (MAZE_WIDTH as f32 / 2.0 * TILE_SIZE),
                                y as f32 * TILE_SIZE - (MAZE_HEIGHT as f32 / 2.0 * TILE_SIZE),
                                0.0,
                            )),
                            ..default()
                        },
                        MazeTile,
                    ));
                }
            }

            commands.spawn((
                SpriteBundle {
                    sprite: Sprite {
                        color: Color::GREEN,
                        custom_size: Some(Vec2::splat(TILE_SIZE)),
                        ..default()
                    },
                    transform: Transform::from_translation(Vec3::new(
                        goal_pos.0 as f32 * TILE_SIZE - (MAZE_WIDTH as f32 / 2.0 * TILE_SIZE),
                        goal_pos.1 as f32 * TILE_SIZE - (MAZE_HEIGHT as f32 / 2.0 * TILE_SIZE),
                        0.5,
                    )),
                    ..default()
                },
                Goal,
            ));

            commands.spawn((
                SpriteBundle {
                    sprite: Sprite {
                        color: Color::RED,
                        custom_size: Some(Vec2::splat(TILE_SIZE * 0.8)),
                        ..default()
                    },
                    transform: Transform::from_translation(Vec3::new(
                        1.0 * TILE_SIZE - (MAZE_WIDTH as f32 / 2.0 * TILE_SIZE),
                        1.0 * TILE_SIZE - (MAZE_HEIGHT as f32 / 2.0 * TILE_SIZE),
                        1.0,
                    )),
                    ..default()
                },
                Player,
            ));

            // 重置计时器
            start_time.0 = Instant::now();
        }
    }
}
