use bevy::prelude::*;
use mobile_entry_point::mobile_entry_point;

#[mobile_entry_point]
fn main() {
    App::build()
        .add_resource(WindowDescriptor {
            title: "Meh2".to_string(),
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup.system())
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let texture_handle = asset_server.load("branding/icon.png");
    commands
        .spawn(Camera2dComponents::default())
        .spawn(SpriteComponents {
            material: materials.add(texture_handle.into()),
            ..Default::default()
        });
}
