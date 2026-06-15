// In Bevy 0.18, touch events use the Message system; TouchInput is a Message not an Event.
use bevy::{camera::ScalingMode, input::touch::*, prelude::*};
use rand::Rng;

const GRID_W: i32 = 10; // number of columns
const GRID_H: i32 = 10; // number of rows
const CELL: f32 = 40.0; // pixel size of one grid cell
const MOVE_SECS: f32 = 0.18; // seconds between each snake step

/// Converts a grid cell (integer) to a world-space pixel position (float).
///
/// Bevy's world origin (0, 0) is the centre of the screen. Without this
/// conversion, cell (0,0) would sit at the centre and the grid would extend
/// to the right and up only. The formula shifts everything so the grid is
/// centred: subtracting half the grid width moves the origin to the left edge,
/// then adding 0.5 nudges it to the centre of the first cell rather than its
/// corner.
///
/// Z is 0.0 here; callers pass `.with_z(...)` to layer game objects correctly
/// (background at -2, grid cells at -1, food at 0.9, body at 0.8, head at 1.0).
fn to_world(p: IVec2) -> Vec3 {
    Vec3::new(
        (p.x as f32 - GRID_W as f32 / 2.0 + 0.5) * CELL,
        (p.y as f32 - GRID_H as f32 / 2.0 + 0.5) * CELL,
        0.0,
    )
}

// ── State ─────────────────────────────────────────────────────────────────────

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
enum GameState {
    #[default]
    Playing,
    GameOver,
}

// ── Direction ─────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Dir {
    Up,
    Down,
    Left,
    Right,
}

impl Dir {
    fn opp(self) -> Self {
        match self {
            Dir::Up => Dir::Down,
            Dir::Down => Dir::Up,
            Dir::Left => Dir::Right,
            Dir::Right => Dir::Left,
        }
    }

    fn delta(self) -> IVec2 {
        match self {
            Dir::Up => IVec2::Y,
            Dir::Down => IVec2::NEG_Y,
            Dir::Left => IVec2::NEG_X,
            Dir::Right => IVec2::X,
        }
    }
}

// ── Components & Resources ────────────────────────────────────────────────────

#[derive(Component)]
struct SnakeHead {
    dir: Dir,
    next: Dir,
}

#[derive(Component)]
struct SnakeSegment;

#[derive(Component)]
struct Food;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
struct GridPos(IVec2);

/// Owns the ordered entity list (index 0 = head) and matching grid positions.
/// Keeping positions here avoids ECS query conflicts during movement.
#[derive(Resource, Default)]
struct Snake {
    ents: Vec<Entity>,
    pos: Vec<IVec2>,
}

#[derive(Resource)]
struct MoveTimer(Timer);

#[derive(Resource, Default)]
struct Score(u32);

#[derive(Component)]
struct ScoreText;

#[derive(Component)]
struct GameOverRoot;

/// Marker on each of the four directional buttons.
#[derive(Component, Clone, Copy)]
struct DpadButton(Dir);

/// Marker on the D-pad container so cleanup can find and ignore it.
#[derive(Component)]
struct DpadRoot;

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct SnakePlugin;

impl Plugin for SnakePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>()
            .init_resource::<Snake>()
            .init_resource::<Score>()
            .insert_resource(MoveTimer(Timer::from_seconds(MOVE_SECS, TimerMode::Repeating)))
            .add_systems(Startup, (setup_camera, setup_bg, setup_score_ui, setup_dpad))
            .add_systems(
                OnEnter(GameState::Playing),
                (reset_game, spawn_snake, spawn_first_food).chain(),
            )
            // input → tick_move ordering ensures direction is committed before the snake steps
            .add_systems(
                Update,
                (input_kb, input_touch, input_dpad, tick_move)
                    .chain()
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(OnEnter(GameState::GameOver), show_gameover)
            .add_systems(
                Update,
                restart_watch.run_if(in_state(GameState::GameOver)),
            )
            .add_systems(OnExit(GameState::GameOver), cleanup_round);
    }
}

// ── Setup ─────────────────────────────────────────────────────────────────────

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        // AutoMin guarantees the whole grid is visible at any aspect ratio, and the
        // extra height reserves a band below the grid for the on-screen D-pad.
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::AutoMin {
                min_width: GRID_W as f32 * CELL + 40.0,
                min_height: GRID_H as f32 * CELL + 300.0,
            },
            ..OrthographicProjection::default_2d()
        }),
        // Bias the view downward so the grid sits in the upper area, keeping the
        // bottom of the screen clear for the controls.
        Transform::from_xyz(0.0, -120.0, 0.0),
    ));
}

fn setup_bg(mut commands: Commands) {
    // Dark outer border
    commands.spawn((
        Sprite {
            color: Color::srgb(0.06, 0.06, 0.06),
            custom_size: Some(Vec2::new(
                GRID_W as f32 * CELL + 8.0,
                GRID_H as f32 * CELL + 8.0,
            )),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, -2.0),
    ));

    // Checkerboard grid cells
    for y in 0..GRID_H {
        for x in 0..GRID_W {
            let v: f32 = if (x + y) % 2 == 0 { 0.11 } else { 0.14 };
            commands.spawn((
                Sprite {
                    color: Color::srgb(v, v, v),
                    custom_size: Some(Vec2::splat(CELL - 1.0)),
                    ..default()
                },
                Transform::from_translation(to_world(IVec2::new(x, y)).with_z(-1.0)),
            ));
        }
    }
}

fn setup_score_ui(mut commands: Commands) {
    commands.spawn((
        Text::new("Score: 0"),
        TextFont {
            font_size: 22.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(8.0),
            left: Val::Px(12.0),
            ..default()
        },
        ScoreText,
    ));
}

fn setup_dpad(mut commands: Commands) {
    // Full-width strip pinned to the bottom of the screen; its children are
    // centred horizontally. Sizes use Vh so the pad scales with the viewport
    // and stays inside the band the camera reserves below the grid.
    commands
        .spawn((
            DpadRoot,
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Vh(3.0),
                left: Val::Px(0.0),
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Vh(1.5),
                ..default()
            },
        ))
        .with_children(|col| {
            // Up
            spawn_dpad_btn(col, Dir::Up, "^");

            // Left + Right on the same row
            col.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Vh(8.0),
                ..default()
            })
            .with_children(|row| {
                spawn_dpad_btn(row, Dir::Left, "<");
                spawn_dpad_btn(row, Dir::Right, ">");
            });

            // Down
            spawn_dpad_btn(col, Dir::Down, "v");
        });
}

fn spawn_dpad_btn(parent: &mut ChildSpawnerCommands, dir: Dir, label: &'static str) {
    parent
        .spawn((
            DpadButton(dir),
            Button,
            Node {
                width: Val::Vh(8.0),
                height: Val::Vh(8.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(Val::Px(2.0)),
                border_radius: BorderRadius::all(Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.45)),
            BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.4)),
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(label),
                TextFont {
                    font_size: 28.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

// ── Playing-state entry ───────────────────────────────────────────────────────

fn reset_game(
    mut score: ResMut<Score>,
    mut timer: ResMut<MoveTimer>,
    mut text: Single<&mut Text, With<ScoreText>>,
) {
    score.0 = 0;
    timer.0.reset();
    **text = Text::new("Score: 0");
}

fn spawn_snake(mut commands: Commands, mut snake: ResMut<Snake>) {
    let cx = GRID_W / 2;
    let cy = GRID_H / 2;

    let head = commands
        .spawn((
            SnakeHead {
                dir: Dir::Right,
                next: Dir::Right,
            },
            SnakeSegment,
            Sprite {
                color: Color::srgb(0.2, 0.92, 0.2),
                custom_size: Some(Vec2::splat(CELL - 4.0)),
                ..default()
            },
            Transform::from_translation(to_world(IVec2::new(cx, cy)).with_z(1.0)),
        ))
        .id();

    let b1 = new_body_seg(&mut commands, IVec2::new(cx - 1, cy));
    let b2 = new_body_seg(&mut commands, IVec2::new(cx - 2, cy));

    snake.ents = vec![head, b1, b2];
    snake.pos = vec![
        IVec2::new(cx, cy),
        IVec2::new(cx - 1, cy),
        IVec2::new(cx - 2, cy),
    ];
}

fn new_body_seg(commands: &mut Commands, p: IVec2) -> Entity {
    commands
        .spawn((
            SnakeSegment,
            Sprite {
                color: Color::srgb(0.1, 0.72, 0.1),
                custom_size: Some(Vec2::splat(CELL - 4.0)),
                ..default()
            },
            Transform::from_translation(to_world(p).with_z(0.8)),
        ))
        .id()
}

fn spawn_first_food(mut commands: Commands, snake: Res<Snake>) {
    do_spawn_food(&mut commands, &snake.pos);
}

fn do_spawn_food(commands: &mut Commands, occupied: &[IVec2]) {
    let mut rng = rand::thread_rng();
    let free: Vec<IVec2> = (0..GRID_W)
        .flat_map(|x| (0..GRID_H).map(move |y| IVec2::new(x, y)))
        .filter(|p| !occupied.contains(p))
        .collect();

    if free.is_empty() {
        return;
    }

    let p = free[rng.gen_range(0..free.len())];
    commands.spawn((
        Food,
        GridPos(p),
        Sprite {
            color: Color::srgb(0.95, 0.22, 0.22),
            custom_size: Some(Vec2::splat(CELL - 8.0)),
            ..default()
        },
        Transform::from_translation(to_world(p).with_z(0.9)),
    ));
}

// ── Input ─────────────────────────────────────────────────────────────────────

fn input_kb(kb: Res<ButtonInput<KeyCode>>, mut head: Single<&mut SnakeHead>) {
    let d = if kb.any_just_pressed([KeyCode::ArrowUp, KeyCode::KeyW]) {
        Some(Dir::Up)
    } else if kb.any_just_pressed([KeyCode::ArrowDown, KeyCode::KeyS]) {
        Some(Dir::Down)
    } else if kb.any_just_pressed([KeyCode::ArrowLeft, KeyCode::KeyA]) {
        Some(Dir::Left)
    } else if kb.any_just_pressed([KeyCode::ArrowRight, KeyCode::KeyD]) {
        Some(Dir::Right)
    } else {
        None
    };

    if let Some(d) = d {
        if d != head.dir.opp() {
            head.next = d;
        }
    }
}

fn input_touch(
    mut evts: MessageReader<TouchInput>,
    mut start: Local<Option<Vec2>>,
    mut head: Single<&mut SnakeHead>,
) {
    for ev in evts.read() {
        match ev.phase {
            TouchPhase::Started => *start = Some(ev.position),
            // "Canceled" is the American spelling Bevy uses
            TouchPhase::Ended | TouchPhase::Canceled => {
                if let Some(s) = start.take() {
                    let delta = ev.position - s;
                    if delta.length() < 15.0 {
                        continue;
                    }
                    // Screen Y increases downward, so +Y delta = swipe toward bottom = Down
                    let d = if delta.x.abs() > delta.y.abs() {
                        if delta.x > 0.0 { Dir::Right } else { Dir::Left }
                    } else if delta.y > 0.0 {
                        Dir::Down
                    } else {
                        Dir::Up
                    };
                    if d != head.dir.opp() {
                        head.next = d;
                    }
                }
            }
            _ => {}
        }
    }
}

fn input_dpad(
    interactions: Query<(&Interaction, &DpadButton), Changed<Interaction>>,
    mut head_q: Query<&mut SnakeHead>,
) {
    for (interaction, btn) in &interactions {
        if *interaction == Interaction::Pressed {
            if let Ok(mut head) = head_q.single_mut() {
                let d = btn.0;
                if d != head.dir.opp() {
                    head.next = d;
                }
            }
        }
    }
}

// ── Movement ──────────────────────────────────────────────────────────────────

fn tick_move(
    mut commands: Commands,
    time: Res<Time>,
    mut timer: ResMut<MoveTimer>,
    mut snake: ResMut<Snake>,
    mut transforms: Query<&mut Transform>,
    mut head_q: Query<&mut SnakeHead>,
    food_q: Query<(Entity, &GridPos), With<Food>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut score: ResMut<Score>,
    mut score_text: Query<&mut Text, With<ScoreText>>,
) {
    if !timer.0.tick(time.delta()).just_finished() {
        return;
    }

    let Ok(mut head) = head_q.single_mut() else {
        return;
    };
    let dir = head.next;
    head.dir = dir;

    if snake.pos.is_empty() {
        return;
    }

    let new_head = snake.pos[0] + dir.delta();

    // Wrap around grid edges
    let new_head = IVec2::new(
        new_head.x.rem_euclid(GRID_W),
        new_head.y.rem_euclid(GRID_H),
    );

    // Self-collision: skip the tail — it moves away this tick
    let body_without_tail = &snake.pos[..snake.pos.len() - 1];
    if body_without_tail.contains(&new_head) {
        next_state.set(GameState::GameOver);
        return;
    }

    // Check food
    let ate = food_q
        .iter()
        .find(|(_, fp)| fp.0 == new_head)
        .map(|(e, _)| e);

    let old_tail = *snake.pos.last().unwrap();

    // Shift positions and transforms (back → front, then head)
    for i in (1..snake.ents.len()).rev() {
        let p = snake.pos[i - 1];
        snake.pos[i] = p;
        if let Ok(mut tf) = transforms.get_mut(snake.ents[i]) {
            tf.translation = to_world(p).with_z(0.8);
        }
    }
    snake.pos[0] = new_head;
    if let Ok(mut tf) = transforms.get_mut(snake.ents[0]) {
        tf.translation = to_world(new_head).with_z(1.0);
    }

    // Grow + respawn food
    if let Some(food_e) = ate {
        commands.entity(food_e).despawn();
        score.0 += 1;
        if let Ok(mut t) = score_text.single_mut() {
            *t = Text::new(format!("Score: {}", score.0));
        }

        // New segment at old tail (the old tail entity shifted forward, so this slot is free)
        let seg = new_body_seg(&mut commands, old_tail);
        snake.ents.push(seg);
        snake.pos.push(old_tail);

        do_spawn_food(&mut commands, &snake.pos);
    }
}

// ── Game Over ─────────────────────────────────────────────────────────────────

fn show_gameover(mut commands: Commands, score: Res<Score>) {
    commands
        .spawn((
            GameOverRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(16.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.78)),
        ))
        .with_children(|p| {
            p.spawn((
                Text::new("GAME OVER"),
                TextFont {
                    font_size: 52.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.3, 0.3)),
            ));
            p.spawn((
                Text::new(format!("Score: {}", score.0)),
                TextFont {
                    font_size: 30.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
            p.spawn((
                Text::new("Tap  /  Press R or Space to restart"),
                TextFont {
                    font_size: 17.0,
                    ..default()
                },
                TextColor(Color::srgb(0.55, 0.55, 0.55)),
            ));
        });
}

fn restart_watch(
    kb: Res<ButtonInput<KeyCode>>,
    touches: Res<Touches>,
    mut next: ResMut<NextState<GameState>>,
) {
    if kb.any_just_pressed([KeyCode::KeyR, KeyCode::Space, KeyCode::Enter])
        || touches.any_just_pressed()
    {
        next.set(GameState::Playing);
    }
}

fn cleanup_round(
    mut commands: Commands,
    segs: Query<Entity, With<SnakeSegment>>,
    foods: Query<Entity, With<Food>>,
    overlay: Query<Entity, With<GameOverRoot>>,
    mut snake: ResMut<Snake>,
) {
    for e in &segs {
        commands.entity(e).despawn();
    }
    for e in &foods {
        commands.entity(e).despawn();
    }
    for e in &overlay {
        commands.entity(e).despawn();
    }
    snake.ents.clear();
    snake.pos.clear();
}

// ── App entry ─────────────────────────────────────────────────────────────────

pub fn run() {
    App::new()
        .add_plugins(
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Bevy Snake".into(),
                    fit_canvas_to_parent: true,
                    prevent_default_event_handling: false,
                    ..default()
                }),
                ..default()
            }),
        )
        .add_plugins(SnakePlugin)
        .run();
}

/// `#[bevy_main]` is a no-op on desktop/web and generates `android_main` on Android.
#[bevy_main]
pub fn main() {
    run();
}
