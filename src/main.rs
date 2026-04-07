use engine_core::prelude::*;

mod game;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let config = GameConfig::new("My Platformer")
        .with_size(800, 600)
        .with_clear_color(0.1, 0.1, 0.15, 1.0);

    run_game(game::PlatformerGame::new(), config)
}
