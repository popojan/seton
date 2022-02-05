use bevy::input::system::exit_on_esc_system;
use bevy::prelude::*;
use bevy::window::exit_on_window_close_system;
use bevy_prototype_lyon::prelude::*;

fn main() {
    App::new()
        .insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins)
        .add_plugin(ShapePlugin)
        .insert_resource(ClearColor(Color::GRAY)) //barva pozadí
        .add_startup_system(setup_system)
        .add_system( exit_on_window_close_system)
        .add_system(exit_on_esc_system)
        .run();
}

fn setup_system(mut commands: Commands) {

    // kamera pro 2D scénu
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());

    // vyplněné kolečko

    let circle = shapes::Circle {
        radius: 100.0,
        center: Vec2::ZERO
    };
    commands.spawn_bundle(GeometryBuilder::build_as(
        &circle,
        DrawMode::Fill(FillMode::color(Color::WHITE)),
        Transform::default(),
    ));

    // cesta s jedním segmentem (úsečkou)

    let mut path_builder = PathBuilder::new();
    path_builder.move_to(Vec2::ZERO);
    path_builder.line_to(95.0 / f32::sqrt(2.0) * Vec2::ONE);
    let line = path_builder.build();

    commands.spawn_bundle(GeometryBuilder::build_as(
        &line,
        DrawMode::Stroke(StrokeMode::new(Color::ORANGE_RED, 10.0)),
        Transform::default(),
    ));
}