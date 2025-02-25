//#![windows_subsystem = "windows"]

use bevy::prelude::Srgba;
use bevy::prelude::*;
use bevy::window::{PresentMode, PrimaryWindow, Window, WindowMode, WindowResized};
use bevy_egui::{egui, EguiContextSettings, EguiContexts, EguiPlugin};
use bevy_prototype_lyon::draw::Fill;
use bevy_prototype_lyon::prelude::*;
use bevy_prototype_lyon::shapes;
use rand::prelude::*;
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
enum AppState {
    Memorizing,
    Solving,
    #[default]
    Results,
}

#[derive(Resource)]
struct SetonGame {
    board_size: usize,
    time_seconds: usize,
    time_started: f64,
    n_black_stones: usize,
    n_white_stones: usize,
    last_score: (usize, usize, usize, f32),
    cursor: Vec2,
    position: [ndarray::Array2<i8>; 2],
    games_played: usize,
}

#[derive(Component)]
struct Piece;

#[derive(Default, Resource)]
struct Board {
    vertical: bool,
    window: Vec2,
    board_side: f32,
    square_side: f32,
    stone_radius: f32,
    origin: Vec2,
}

fn sample_stone_coords(
    n_black_stones: usize,
    n_white_stones: usize,
    board_size: usize,
) -> ndarray::Array2<i8> {
    let mut squares: Vec<(usize, usize)> = vec![];
    for i in 0..board_size {
        for j in 0..board_size {
            squares.push((i, j));
        }
    }
    let mut ret = ndarray::Array2::zeros([board_size, board_size]);
    //let mut ret: Vec<(u32, u32)> = vec![];
    for (k, (i, j)) in squares
        .choose_multiple(
            &mut rand::rng(),
            usize::min(n_black_stones + n_white_stones as usize, squares.len()),
        )
        .enumerate()
    {
        ret[[*i, *j]] = if k < n_black_stones { 1_i8 } else { -1_i8 };
    }
    ret
}
impl Default for SetonGame {
    fn default() -> Self {
        let n_black_stones: usize = 5;
        let n_white_stones: usize = 5;
        let board_size: usize = 5;
        let time_seconds = 30;
        Self {
            board_size,
            time_started: 0.0,
            time_seconds,
            n_black_stones,
            n_white_stones,
            last_score: (0, 0, n_black_stones + n_white_stones, 0.0),
            cursor: Vec2::new(0.0, 0.0),
            position: [
                ndarray::Array2::zeros([board_size, board_size]),
                ndarray::Array2::zeros([board_size, board_size]),
            ],
            games_played: 0,
        }
    }
}

fn main() {
    App::new()
.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Setonova hra".to_string(),
                present_mode: PresentMode::AutoVsync,
                resizable: true,
                mode: WindowMode::Windowed,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(ShapePlugin)
        .add_plugins(EguiPlugin)
        .init_state::<AppState>()
        .insert_resource(ClearColor(Srgba::gray(0.25).into()))
        .insert_resource(SetonGame::default())
        .insert_resource(Board::default())
        .add_systems(Startup, setup)
        .add_systems(Update, resized_redraw_system)
        .add_systems(OnEnter(AppState::Memorizing), redraw_system)
        .add_systems(OnEnter(AppState::Solving), redraw_system)
        .add_systems(OnEnter(AppState::Results), redraw_system)
        .add_systems(Update, mouse_move.run_if(in_state(AppState::Solving)))
        .add_systems(Update, egui_settings)
        .run();
}

fn egui_settings(
    mut egui_context: EguiContexts,
    state: Res<State<AppState>>,
    mut contexts: Query<&mut EguiContextSettings>,
    mut next_state: ResMut<NextState<AppState>>,
    mut game: ResMut<SetonGame>,
    time: Res<Time>,
    mut window_query: Query<&mut Window, With<PrimaryWindow>>,
) {
    if let Ok(mut egui_settings) = contexts.get_single_mut() {
        if let Ok(window) = window_query.get_single_mut() {
            egui_settings.scale_factor = 1.75 / window.scale_factor();
        }
    }
    let mut progress_left = 1.0;
    egui::TopBottomPanel::top("top_bar").show(egui_context.ctx_mut(), |ui| {
        ui.horizontal_wrapped(|ui| {
            if ui.button(": :").clicked() {
                if let Ok(mut window) = window_query.get_single_mut() {
                    if window.mode == WindowMode::Windowed {
                        window.mode = WindowMode::BorderlessFullscreen(MonitorSelection::Primary);
                    } else {
                        window.mode = WindowMode::Windowed;
                    }
                }
            }
            ui.separator();
            ui.label("Setonova hra");
            ui.separator();
            if state.get() == &AppState::Results {
                if ui.button("Start").clicked() {
                    game.position[0] = sample_stone_coords(
                        game.n_black_stones,
                        game.n_white_stones,
                        game.board_size,
                    );
                    game.position[1] = ndarray::Array2::zeros([game.board_size, game.board_size]);
                    game.time_started = time.elapsed_secs_f64();
                    next_state.set(AppState::Memorizing);
                }
            } else if state.get() == &AppState::Memorizing {
                ui.label("Zapamatuj si...");
                progress_left = 1.0
                    - (time.elapsed_secs_f64() - game.time_started) / game.time_seconds as f64;
                ui.separator();
                if ui.button("Už vím").clicked() {
                    progress_left = 0.0;
                }
                if progress_left <= 0.0 {
                    next_state.set(AppState::Solving);
                }
            } else if state.get() == &AppState::Solving {
                ui.label("Rozestav kameny...");
                ui.separator();
                progress_left = 1.0
                    - (time.elapsed_secs_f64() - game.time_started) / game.time_seconds as f64;
                if ui.button("Hotovo").clicked() {
                    let (correct_2, correct_1) = {
                        let truth = &game.position[0];
                        let solution = &game.position[1];
                        (
                            (truth * solution).map(|x| i8::max(0, *x)).sum() as usize,
                            (truth * solution).map(|x| i8::max(0, -*x)).sum() as usize,
                        )
                    };
                    game.last_score.0 = correct_2;
                    game.last_score.1 = correct_1;
                    game.last_score.2 = game.n_black_stones + game.n_white_stones
                        - game.last_score.0
                        - game.last_score.1;
                    game.last_score.3 = correct_2 as f32
                        / (game.n_black_stones as f32 + game.n_white_stones as f32)
                        + 0.5 * correct_1 as f32
                            / (game.n_black_stones as f32 + game.n_white_stones as f32);
                    game.games_played += 1;
                    next_state.set(AppState::Results);
                }
            }
            if state.get() == &AppState::Results {
                ui.separator();
                ui.add(egui::Slider::new(&mut game.board_size, 5..=10).text("deska"));
                ui.separator();
                ui.add(egui::Slider::new::<usize>(&mut game.n_white_stones, 1..=10).text("bílých"));
                ui.separator();
                ui.add(
                    egui::Slider::new::<usize>(&mut game.n_black_stones, 1..=10).text("černých"),
                );
                ui.separator();
                ui.add(egui::Slider::new::<usize>(&mut game.time_seconds, 1..=120).text("čas"));
            }
        });
    });
    egui::TopBottomPanel::bottom("bottom_bar").show(egui_context.ctx_mut(), |ui| {
        ui.horizontal_centered(|ui| {
            if state.get() == &AppState::Memorizing {
                ui.add(egui::ProgressBar::new(progress_left as f32));
            } else if game.games_played > 0 && state.get() != &AppState::Solving {
                ui.label(format!(
                    "Hodnocení:  {} %",
                    f32::round(100.0 * game.last_score.3)
                ));
                ui.separator();
                ui.label(format!("Správně:  {}", game.last_score.0));
                ui.separator();
                ui.label(format!("Špatně barva:  {}", game.last_score.1));
                ui.separator();
                ui.label(format!("Špatně pozice:  {}", game.last_score.2));
            }
        });
    });
}

fn resized_redraw_system(
    commands: Commands,
    window_query: Query<&Window, With<PrimaryWindow>>,
    game: ResMut<SetonGame>,
    mut view: ResMut<Board>,
    mut event_resized: EventReader<WindowResized>,
    source: Query<Entity, With<Piece>>,
    state: ResMut<State<AppState>>,
) {
    let window = window_query.get_single().unwrap();
    let width = window.width();
    let height = window.height();

    view.window.x = width;
    view.window.y = height;

    let mut count = 0;
    for _event in event_resized.read() {
        count += 1;
    }
    if count > 0 {
        redraw_system(commands, game, view, source, state);
    }
}

fn redraw_system(
    mut commands: Commands,
    game: ResMut<SetonGame>,
    view: ResMut<Board>,
    source: Query<Entity, With<Piece>>,
    state: ResMut<State<AppState>>,
) {
    source.iter().for_each(|id| commands.entity(id).despawn());
    spawn_source(commands, game, view, state);
}

fn mouse_move(
    mut motion: EventReader<CursorMoved>,
    button: Res<ButtonInput<MouseButton>>,
    mut game: ResMut<SetonGame>,
    view: ResMut<Board>,
    source: Query<Entity, With<Piece>>,
    state: ResMut<State<AppState>>,
    commands: Commands,
) {
    if let Some(e) = motion.read().last() {
        game.cursor.x = e.position.x;
        game.cursor.y = e.position.y;
    }
    if button.just_pressed(MouseButton::Left) || button.just_pressed(MouseButton::Right) {
        let p = game.cursor;
        let i = game.board_size as f32
            * (0.5 + (p.x - 0.5 * view.window.x - view.origin.x) / view.board_side);
        let j = game.board_size as f32
            * (0.5 + (p.y - 0.5 * view.window.y - view.origin.y) / view.board_side);
        if i >= 0.0 && i < game.board_size as f32 && j >= 0.0 && j < game.board_size as f32 {
            let i = f32::floor(i) as usize;
            let j = game.board_size - f32::floor(j) as usize - 1;
            let stone = game.position[1][[i, j]];
            let sign = if button.just_pressed(MouseButton::Left) {
                1
            } else {
                -1
            };
            let mut new_stone = if stone == 0 {
                1 * sign
            } else if stone == 1 * sign {
                -1 * sign
            } else {
                0
            };
            let black_stones_placed = game.position[1].map(|&x| if x == 1 { 1 } else { 0 }).sum();
            let white_stones_placed = game.position[1].map(|&x| if x == -1 { 1 } else { 0 }).sum();
            if new_stone == 1 && black_stones_placed >= game.n_black_stones {
                new_stone = -1;
            }
            if new_stone == -1 && white_stones_placed >= game.n_white_stones {
                new_stone = 1;
            }
            if new_stone == 1 && black_stones_placed >= game.n_black_stones {
                new_stone = 0;
            }
            if !((new_stone == 1 && black_stones_placed >= game.n_black_stones)
                || (new_stone == -1 && white_stones_placed >= game.n_white_stones))
            {
                game.position[1][[i, j]] = new_stone;
                redraw_system(commands, game, view, source, state);
                //if let Ok(mut window) = window_query.get_single_mut() {
                //    window.resolution.set(view.window.x, view.window.y);
                //}
            }
        }
    }
}
fn spawn_board(
    commands: &mut Commands,
    game: &ResMut<SetonGame>,
    view: &mut ResMut<Board>,
    origin: Vec2,
    board_side: f32,
    padding: f32,
    grid_color: Color,
    square_color: Color,
    solution: usize,
) {
    let board = shapes::Rectangle {
        extents: Vec2::new(board_side, board_side),
        origin: RectangleOrigin::Center,
        ..Default::default()
    };

    let n_half: f32 = 0.5 * (game.board_size - 1) as f32;
    let square_side = board_side / (game.board_size as f32 + padding) as f32;

    let square = shapes::Rectangle {
        extents: Vec2::new(square_side / (1.0 + padding), square_side / (1.0 + padding)),
        origin: RectangleOrigin::Center,
        ..Default::default()
    };

    let stone_radius = 0.5 * square_side / (1.0 + 4.0 * padding);
    let stone = shapes::Circle {
        radius: stone_radius,
        center: Default::default(),
    };

    commands
        .spawn((
            ShapeBundle {
                path: GeometryBuilder::build_as(&board),
                transform: Transform::default()
                        .with_translation(Vec3::new(origin.x, origin.y, 0.0)),
                ..default()
            },
            Fill::color(grid_color),
        ))
        .insert(Piece);

    for i in 0..game.board_size {
        for j in 0..game.board_size {
            commands
                .spawn((
                    ShapeBundle {
                        path: GeometryBuilder::build_as(&square),
                        transform: Transform::default().with_translation(Vec3::new(
                                origin.x + (i as f32 - n_half) * square_side,
                                origin.y + (j as f32 - n_half) * square_side,
                                0.5,
                            )),
                        ..default()
                    },
                    Fill::color(square_color),
                ))
                .with_children(|parent| {
                    let is_stone = game.position[solution][[i, j]];
                    if is_stone != 0 {
                        parent
                            .spawn((
                                ShapeBundle {
                                    path: GeometryBuilder::build_as(&stone),
                                    transform: Transform::default()
                                            .with_translation(Vec3::new(0.0, 0.0, 0.5)),
                                    ..default()
                                },
                                Fill::color(if is_stone > 0 {
                                    Color::BLACK
                                } else {
                                    Color::WHITE
                                }),
                            ))
                            .insert(Piece);
                    }
                })
                .insert(Piece);
        }
    }
    view.board_side = board_side;
    view.square_side = square_side;
    view.origin = origin;
    view.stone_radius = stone_radius;
}

fn setup(mut commands: Commands)
{
    commands.spawn(Camera2d::default());
}

fn spawn_source(
    mut commands: Commands,
    game: ResMut<SetonGame>,
    mut view: ResMut<Board>,
    state: ResMut<State<AppState>>,
) {
    const PADDING: f32 = 0.05;
    let height_minus_gui = 0.9;
    let vertical = height_minus_gui * view.window.y > view.window.x;
    let board_side = (1.0 - PADDING)
        * if vertical {
            f32::min(view.window.x, height_minus_gui * 0.5 * view.window.y)
        } else {
            f32::min(height_minus_gui * view.window.y, 0.5 * view.window.x)
        };
    let shift = 0.5 * board_side * (1.0 + PADDING);
    let (origin_source, origin_target) = if vertical {
        (Vec2::new(0.0, shift), Vec2::new(0.0, -shift))
    } else {
        (Vec2::new(-shift, 0.0), Vec2::new(shift, 0.0))
    };

    const GRID_COLOR: Color = Color::srgb(192.0 / 255., 192. / 255., 192. / 255.);
    const SQUARE_COLOR: Color = Color::srgb(0.5, 0.5, 0.5);

    view.vertical = vertical;
    let results = state.get() == &AppState::Results && game.games_played > 0;
    if state.get() == &AppState::Memorizing || results {
        spawn_board(
            &mut commands,
            &game,
            &mut view,
            origin_source,
            board_side,
            PADDING,
            GRID_COLOR,
            SQUARE_COLOR,
            0,
        );
    }
    if state.get() == &AppState::Solving || results {
        spawn_board(
            &mut commands,
            &game,
            &mut view,
            origin_target,
            board_side,
            PADDING,
            GRID_COLOR,
            SQUARE_COLOR,
            1,
        );
    }
}
