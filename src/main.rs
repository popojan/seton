use bevy::prelude::*;
use bevy_egui::{egui, EguiContext, EguiPlugin, EguiSettings};
use bevy_prototype_lyon::prelude::*;
use bevy::window::{WindowId, WindowMode, WindowResized};
use bevy::window::exit_on_all_closed;
use bevy::window::close_on_esc;
use bevy_prototype_lyon::shapes;
use rand::seq::SliceRandom;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum AppState {
    Setting,
    Memorizing,
    Solving,
    Results,
}

struct SetonGame {
  board_size: usize,
  time_seconds: usize,
  time_started: f64,
  n_black_stones: usize,
  n_white_stones: usize,
  last_score: (usize, usize, usize, f32),
  cursor: Vec2,
  position: [ndarray::Array2<i8>; 2],
  games_played: usize
}

#[derive(Component)]
struct Piece;

#[derive(Default)]
struct Board {
    vertical: bool,
    window_id: WindowId,
    window: Vec2,
    board_side: f32,
    square_side: f32,
    stone_radius: f32,
    origin: Vec2,
}

fn sample_stone_coords(n_black_stones: usize, n_white_stones: usize, board_size: usize) -> ndarray::Array2<i8> {
    let mut squares: Vec<(usize, usize)> = vec![];
    for i in 0..board_size {
        for j in 0..board_size {
            squares.push((i, j));
        }
    }
    let mut ret = ndarray::Array2::zeros([board_size, board_size]);
    //let mut ret: Vec<(u32, u32)> = vec![];
    for (k, (i, j)) in squares.choose_multiple(
        &mut rand::thread_rng(),
        usize::min(n_black_stones + n_white_stones as usize, squares.len())
    ).enumerate() {
        ret[[*i, *j]] = if k < n_black_stones {1_i8} else {-1_i8};
    };
    ret
}
impl Default for SetonGame {
    fn default() -> Self {
        let n_black_stones: usize = 8;
        let n_white_stones: usize = 8;
        let board_size: usize =     8;
        let time_seconds = 30;
        Self {
            board_size,
            time_started: 0.0,
            time_seconds,
            n_black_stones,
            n_white_stones,
            last_score: (0, 0, n_black_stones + n_white_stones, 0.0),
            cursor: Vec2::new(0.0,0.0),
            position: [
                ndarray::Array2::zeros([board_size, board_size]),
                ndarray::Array2::zeros([board_size, board_size])
            ],
            games_played: 0,
        }
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(ShapePlugin)
        .add_plugin(EguiPlugin)
        .add_state(AppState::Setting)
        .insert_resource(ClearColor(Color::GRAY))
        .insert_resource(Msaa { samples: 4 })
        .insert_resource(SetonGame::default())
        .insert_resource(Board::default())
        .add_system(close_on_esc)
        .add_system(exit_on_all_closed)
        .add_startup_system(setup)
        .add_system(resized_redraw_system)
        .add_system(egui_settings)
        .add_system_set(SystemSet::on_enter(AppState::Memorizing)
            .with_system(redraw_system)
        )
        .add_system_set(SystemSet::on_update(AppState::Solving)
            .with_system(redraw_system)
            .with_system(mouse_move)
        )
        .add_system_set(SystemSet::on_enter(AppState::Results)
            .with_system(redraw_system)
        )
        .run();
}

fn egui_settings(mut egui_context: ResMut<EguiContext>,
                 mut egui_settings: ResMut<EguiSettings>,
                 mut state: ResMut<State<AppState>>,
                mut game: ResMut<SetonGame>,
                time: Res<Time>,
                mut windows: ResMut<Windows>,
) {
    egui_settings.scale_factor = 1.75;
    let mut progress_left = 1.0;
    egui::TopBottomPanel::top("top_bar")
    .show(egui_context.ctx_mut(), |ui| {
        ui.horizontal_wrapped(|ui| {
            if ui.button(": :").clicked() {
                if let Some(window) = windows.get_primary_mut() {
                    if window.mode() != WindowMode::BorderlessFullscreen {
                        window.set_mode(WindowMode::BorderlessFullscreen);
                    } else {
                        window.set_mode(WindowMode::Windowed);
                    }
                }
            }
            ui.separator();
            ui.label("Setonova hra");
            ui.separator();
            if state.current() == &AppState::Setting {
                if ui.button("Start").clicked() {
                    game.position[0] = sample_stone_coords(game.n_black_stones, game.n_white_stones, game.board_size);
                    game.position[1] = ndarray::Array2::zeros([game.board_size, game.board_size]);
                    game.time_started = time.seconds_since_startup();
                    state.set(AppState::Memorizing).unwrap();
                }
            } else if state.current() == &AppState::Memorizing {
                ui.label("Zapamatuj si...");
                progress_left = 1.0 - (time.seconds_since_startup() - game.time_started)/game.time_seconds as f64;
                ui.separator();
                if ui.button("Už vím").clicked() {
                    progress_left = 0.0;
                }
                if progress_left <= 0.0 {
                    state.set(AppState::Solving).unwrap();
                }
            } else if state.current() == &AppState::Solving {
                ui.label("Rozestav kameny...");
                ui.separator();
                progress_left = 1.0 - (time.seconds_since_startup() - game.time_started)/game.time_seconds as f64;
                if ui.button("Hotovo").clicked() {
                    let (correct_2, correct_1) = {
                        let truth = &game.position[0];
                        let solution = &game.position[1];
                        (
                            (truth * solution).map(|x| i8::max(0, *x)).sum() as usize,
                            (truth * solution).map(|x| i8::max(0, -*x)).sum() as usize
                        )
                    };
                    game.last_score.0 = correct_2;
                    game.last_score.1 = correct_1;
                    game.last_score.2 = game.n_black_stones + game.n_white_stones
                        - game.last_score.0 - game.last_score.1;
                    game.last_score.3 = correct_2 as f32/(game.n_black_stones as f32 + game.n_white_stones as f32)
                        + 0.5 * correct_1 as f32/(game.n_black_stones as f32 + game.n_white_stones as f32);
                    game.games_played += 1;
                    state.set(AppState::Results).unwrap();
                }
            } else {
                state.set(AppState::Setting).unwrap();
                /*if ui.button("Skrýt").clicked() {
                    state.set(AppState::Setting).unwrap();
                }*/
            }
            if state.current() == &AppState::Setting {
                ui.separator();
                ui.label("deska");
                ui.add(egui::Slider::new::<usize>(&mut game.board_size, 6..=12));
                ui.separator();
                ui.label("bílých");
                ui.add(egui::Slider::new::<usize>(&mut game.n_white_stones, 1..=16));
                ui.separator();
                ui.label("černých");
                ui.add(egui::Slider::new::<usize>(&mut game.n_black_stones, 1..=16));
                ui.separator();
                ui.label("čas");
                ui.add(egui::Slider::new::<usize>(&mut game.time_seconds, 10..=120));
            }
        });
    });
    egui::TopBottomPanel::bottom("bottom_bar")
    .show(egui_context.ctx_mut(), |ui| {
        ui.horizontal_centered(|ui| {
            if state.current() == &AppState::Memorizing {
                ui.add(egui::ProgressBar::new(progress_left as f32));
            } else if game.games_played > 0  && state.current() != &AppState::Solving {
                ui.label(format!("Hodnocení: {} %", f32::round(100.0 * game.last_score.3)));
                ui.separator();
                ui.label(format!("Správně: {}", game.last_score.0));
                ui.separator();
                ui.label(format!("Špatně barva: {}", game.last_score.1));
                ui.separator();
                ui.label(format!("Špatně pozice: {}", game.last_score.2));
            }
        });
    });

}

fn resized_redraw_system(
    commands: Commands,
    mut windows: ResMut<Windows>,
    game: Res<SetonGame>,
    mut view: ResMut<Board>,
    mut event_resized: EventReader<WindowResized>,
    source: Query<Entity, With<Piece>>,
    state: Res<State<AppState>>
) {
    let window = windows.get_primary_mut().unwrap();
    window.set_title("Setonova hra".to_string());
    let width = window.width();
    let height = window.height();

    view.window.x = width;
    view.window.y = height;
    view.window_id = window.id();

    if state.current() == &AppState::Memorizing {
        if let Some(_event) = event_resized.iter().last() {
            redraw_system(commands, game, view, source, state);
        }
    }
}

fn redraw_system(
    mut commands: Commands,
    game: Res<SetonGame>,
    view: ResMut<Board>,
    source: Query<Entity, With<Piece>>,
    state: Res<State<AppState>>,
) {
    source.iter().for_each(|id| commands.entity(id).despawn());
    spawn_source(commands, game, view, state);
}

fn mouse_move(
    mut motion : EventReader<CursorMoved>,
    button: Res<Input<MouseButton>>,
    mut game: ResMut<SetonGame>,
    view: Res<Board>,
    mut redraw: EventWriter<WindowResized>,
) {
    for e in motion.iter().last() {
        game.cursor.x = e.position.x;
        game.cursor.y = e.position.y;
    }
    if button.just_pressed(MouseButton::Left) || button.just_pressed(MouseButton::Right) {
        let p = game.cursor;
        let i = game.board_size as f32 *(0.5 + (p.x - 0.5*view.window.x - view.origin.x) / view.board_side);
        let j = game.board_size as f32 *(0.5 + (p.y - 0.5*view.window.y - view.origin.y) / view.board_side);
        if i >= 0.0 && i< game.board_size as f32 && j >=0.0 && j<game.board_size as f32 {
            let i = f32::floor(i) as usize;
            let j = f32::floor(j) as usize;
            let stone = game.position[1][[i, j]];
            let sign = if button.just_pressed(MouseButton::Left) { 1 } else {-1};
            let new_stone = if stone == 0 {
                1 * sign
            } else if stone == 1 * sign {
                -1 * sign
            } else {
                0
            };
            let stones_placed = game.position[1].map(|x| if x == &new_stone {1} else {0}).sum();
            if !(
                   (new_stone == 1 && stones_placed >= game.n_black_stones)
                || (new_stone == -1 && stones_placed >= game.n_white_stones)
            ) {
                game.position[1][[i, j]] = new_stone;
                redraw.send(WindowResized {
                    id: view.window_id,
                    width: view.window.x,
                    height: view.window.y
                })
            }
        }
    }
}
fn spawn_board(commands: &mut Commands, game: &Res<SetonGame>, view: &mut ResMut<Board>, origin: Vec2, board_side: f32, padding: f32,
               grid_color: Color, square_color: Color, solution: usize)
{
    let board = shapes::Rectangle {
        extents: Vec2::new(board_side, board_side),
        origin: RectangleOrigin::Center,
    };

    let n_half: f32 = 0.5 * (game.board_size-1) as f32;
    let square_side = board_side / (game.board_size as f32 + padding) as f32;

    let square = shapes::Rectangle {
        extents: Vec2::new(square_side / (1.0 + padding), square_side/ (1.0 + padding)),
        origin: RectangleOrigin::Center,
    };

    let stone_radius = 0.5 * square_side / (1.0 + 4.0 * padding);
    let stone = shapes::Circle {
        radius: stone_radius,
        center: Default::default()
    };

    commands.spawn_bundle(GeometryBuilder::build_as(
        &board,
        DrawMode::Fill(FillMode::color(grid_color)),
        Transform::default().with_translation(Vec3::new(origin.x, origin.y, 0.0)),
    )).insert(Piece);

    for i in 0..game.board_size {
        for j in 0..game.board_size {
            commands.spawn_bundle(GeometryBuilder::build_as(
                &square,
                DrawMode::Fill(FillMode::color(square_color)),
                Transform::default().with_translation(Vec3::new(
                    origin.x+ (i as f32-n_half)*square_side, origin.y+(j as f32 - n_half)*square_side, 0.0)),
            )).insert(Piece);
            let is_stone = game.position[solution][[i, j]];
            if is_stone != 0 {
                commands.spawn_bundle(GeometryBuilder::build_as(
                    &stone,
                    DrawMode::Fill(FillMode::color(if is_stone > 0  {Color::BLACK} else {Color::WHITE})),
                    Transform::default().with_translation(Vec3::new(
                        origin.x+ (i as f32-n_half)*square_side, origin.y+(j as f32 - n_half)*square_side, 0.0)),
                )).insert(Piece);
            }
        }
    }
    view.board_side = board_side;
    view.square_side = square_side;
    view.origin = origin;
    view.stone_radius = stone_radius;
}

fn setup(mut commands: Commands) {
    // kamera pro 2D scénu
    commands.spawn_bundle(Camera2dBundle::default());
}

fn spawn_source(
    mut commands: Commands,
    game: Res<SetonGame>,
    mut view: ResMut<Board>,
    state: Res<State<AppState>>,
) {
    const PADDING: f32 = 0.05;
    let vertical = view.window.y > view.window.x;
    let board_side = (1.0 - PADDING) * if vertical {
        f32::min(view.window.x, 0.5 * view.window.y)
    } else {
        f32::min(view.window.y, 0.5 * view.window.x)
    };
    let shift = 0.5 * board_side * (1.0 + PADDING);
    let (origin_source, origin_target) = if vertical {
        (
            Vec2::new(0.0, shift),
            Vec2::new(0.0, -shift)
        )
    } else {
        (
            Vec2::new(-shift,0.0),
            Vec2::new( shift,0.0)
        )
    };

    const GRID_COLOR: Color = Color::rgb(192.0/255.,192./255.,192./255.);
    const SQUARE_COLOR: Color = Color::GRAY;

    view.vertical = vertical;
    if state.current() == &AppState::Memorizing || state.current() == &AppState::Results {
        spawn_board(&mut commands, &game, &mut view, origin_source, board_side, PADDING, GRID_COLOR, SQUARE_COLOR, 0);
    }
    if state.current() == &AppState::Solving || state.current() == &AppState::Results {
        spawn_board(&mut commands, &game, &mut view, origin_target, board_side, PADDING, GRID_COLOR, SQUARE_COLOR, 1);
    }
}