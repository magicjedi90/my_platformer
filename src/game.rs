use engine_core::prelude::*;
use std::path::Path;

pub struct PlatformerGame {
    physics: Option<PhysicsSystem>,
    behaviors: BehaviorRunner,
    scene_instance: Option<SceneInstance>,
    transform_hierarchy: TransformHierarchySystem,
    jump_sound: Option<SoundHandle>,
    music_playing: bool,
    volume: f32,
    show_ui: bool,
    font_loaded: bool,
}

impl PlatformerGame {
    pub fn new() -> Self {
        Self {
            physics: None,
            behaviors: BehaviorRunner::new(),
            scene_instance: None,
            transform_hierarchy: TransformHierarchySystem::new(),
            jump_sound: None,
            music_playing: false,
            volume: 1.0,
            show_ui: true,
            font_loaded: false,
        }
    }

    fn reset_player(&mut self, ctx: &mut GameContext) {
        let player = self.scene_instance.as_ref()
            .and_then(|scene| scene.get_entity("player"));

        if let Some(player) = player {
            if let Some(transform) = ctx.world.get_mut::<Transform2D>(player) {
                transform.position = Vec2::new(-200.0, 100.0);
            }
            if let Some(body) = ctx.world.get_mut::<RigidBody>(player) {
                body.velocity = Vec2::ZERO;
            }
            if let Some(physics) = &mut self.physics {
                physics.physics_world_mut().set_body_transform(
                    player, Vec2::new(-200.0, 100.0), 0.0,
                );
                physics.set_velocity(player, Vec2::ZERO, 0.0);
            }
        }
    }
}

impl Game for PlatformerGame {
    fn init(&mut self, ctx: &mut GameContext) {
        // Default base path is "assets" — correct for standalone projects
        let scene_path = Path::new("assets/scenes/level1.scene.ron");

        match SceneLoader::load_and_instantiate(scene_path, ctx.world, ctx.assets) {
            Ok(instance) => {
                log::info!("Loaded scene '{}' with {} entities", instance.name, instance.entity_count);
                self.behaviors.set_named_entities(instance.named_entities.clone());

                let physics_config = if let Some(settings) = &instance.physics {
                    PhysicsConfig::new(Vec2::new(settings.gravity.0, settings.gravity.1))
                        .with_scale(settings.pixels_per_meter)
                } else {
                    PhysicsConfig::platformer()
                };

                self.physics = Some(PhysicsSystem::with_config(physics_config));
                self.scene_instance = Some(instance);
            }
            Err(e) => {
                log::warn!("Failed to load scene: {}", e);
                log::info!("Creating entities programmatically as fallback...");

                let player = ctx.world.create_entity();
                ctx.world.add_component(&player, Transform2D::new(Vec2::new(-200.0, 100.0))).ok();
                ctx.world.add_component(&player, Sprite::new(0).with_color(Vec4::new(0.2, 0.4, 1.0, 1.0))).ok();
                ctx.world.add_component(&player, RigidBody::player_platformer()).ok();
                ctx.world.add_component(&player, Collider::player_box()).ok();
                ctx.world.add_component(&player, Behavior::PlayerPlatformer {
                    move_speed: 120.0,
                    jump_impulse: 420.0,
                    jump_cooldown: 0.3,
                    tag: "player".to_string(),
                }).ok();

                let ground = ctx.world.create_entity();
                ctx.world.add_component(&ground,
                    Transform2D::new(Vec2::new(0.0, -250.0))
                        .with_scale(Vec2::new(10.0, 0.5))
                ).ok();
                ctx.world.add_component(&ground,
                    Sprite::new(0).with_color(Vec4::new(0.3, 0.3, 0.3, 1.0))
                ).ok();
                ctx.world.add_component(&ground, RigidBody::new_static()).ok();
                ctx.world.add_component(&ground, Collider::platform(800.0, 40.0)).ok();

                self.physics = Some(PhysicsSystem::with_config(PhysicsConfig::platformer()));
            }
        }

        // Initialize systems
        if let Some(physics) = &mut self.physics {
            physics.initialize(ctx.world).ok();
        }
        self.transform_hierarchy.initialize(ctx.world).ok();

        // Load sound effects
        match ctx.audio.load_sound("assets/sounds/snd_jump.wav") {
            Ok(handle) => {
                self.jump_sound = Some(handle);
                log::info!("Loaded jump sound effect");
            }
            Err(e) => {
                log::info!("No jump sound loaded ({})", e);
            }
        }

        // Try background music
        if ctx.audio.play_music("assets/sounds/music.ogg").is_ok() {
            self.music_playing = true;
        }

        // Load font
        match ctx.ui.load_font_file("assets/fonts/font.ttf") {
            Ok(_) => {
                self.font_loaded = true;
                log::info!("Font loaded");
            }
            Err(_) => {
                let font_paths = [
                    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
                    "/usr/share/fonts/TTF/DejaVuSans.ttf",
                    "/System/Library/Fonts/Helvetica.ttc",
                    "C:\\Windows\\Fonts\\arial.ttf",
                ];
                for path in font_paths {
                    if ctx.ui.load_font_file(path).is_ok() {
                        self.font_loaded = true;
                        log::info!("System font loaded from: {}", path);
                        break;
                    }
                }
            }
        }

        let total = ctx.world.entity_count();
        let roots = ctx.world.get_root_entities().len();
        log::info!("Game initialized: {} entities ({} roots, {} children)",
                   total, roots, total - roots);
    }

    fn on_play_stopped(&mut self, _ctx: &mut GameContext) {
        if let Some(physics) = &mut self.physics {
            physics.clear();
        }
    }

    fn update(&mut self, ctx: &mut GameContext) {
        // Jump sound
        if ctx.input.is_key_just_pressed(KeyCode::Space) {
            if let Some(jump_sound) = &self.jump_sound {
                let settings = SoundSettings::new().with_volume(0.8).with_speed(1.0);
                ctx.audio.play_with_settings(jump_sound, settings).ok();
            }
        }

        // Music toggle
        if ctx.input.is_key_just_pressed(KeyCode::KeyM) {
            if self.music_playing {
                ctx.audio.pause_music();
                self.music_playing = false;
            } else {
                ctx.audio.resume_music();
                self.music_playing = true;
            }
        }

        // Volume controls
        if ctx.input.is_key_just_pressed(KeyCode::Equal) {
            let v = (ctx.audio.master_volume() + 0.1).min(1.0);
            ctx.audio.set_master_volume(v);
        }
        if ctx.input.is_key_just_pressed(KeyCode::Minus) {
            let v = (ctx.audio.master_volume() - 0.1).max(0.0);
            ctx.audio.set_master_volume(v);
        }

        // Behaviours (player movement)
        self.behaviors.update(
            ctx.world,
            ctx.input,
            ctx.delta_time,
            self.physics.as_mut(),
        );

        // Reset
        if ctx.input.is_key_pressed(KeyCode::KeyR) {
            self.reset_player(ctx);
        }

        // Physics
        if let Some(physics) = &mut self.physics {
            physics.update(ctx.world, ctx.delta_time);
        }

        // Hierarchy propagation
        self.transform_hierarchy.update(ctx.world, ctx.delta_time);

        // UI
        if ctx.input.is_key_just_pressed(KeyCode::KeyH) {
            self.show_ui = !self.show_ui;
        }

        if self.show_ui {
            let panel_rect = UIRect::new(10.0, 10.0, 220.0, 200.0);
            ctx.ui.panel(panel_rect);

            ctx.ui.label("Controls", Vec2::new(20.0, 25.0));

            ctx.ui.label("Volume:", Vec2::new(20.0, 55.0));
            let slider_rect = UIRect::new(20.0, 70.0, 190.0, 20.0);
            let new_volume = ctx.ui.slider("volume_slider", self.volume, slider_rect);
            if new_volume != self.volume {
                self.volume = new_volume;
                ctx.audio.set_master_volume(self.volume);
            }

            let music_btn_rect = UIRect::new(20.0, 100.0, 90.0, 30.0);
            let music_label = if self.music_playing { "Pause" } else { "Play" };
            if ctx.ui.button("music_btn", music_label, music_btn_rect) {
                if self.music_playing {
                    ctx.audio.pause_music();
                    self.music_playing = false;
                } else {
                    ctx.audio.resume_music();
                    self.music_playing = true;
                }
            }

            let reset_btn_rect = UIRect::new(120.0, 100.0, 90.0, 30.0);
            if ctx.ui.button("reset_btn", "Reset", reset_btn_rect) {
                self.reset_player(ctx);
            }

            ctx.ui.label("Volume Bar:", Vec2::new(20.0, 145.0));
            let progress_rect = UIRect::new(20.0, 160.0, 190.0, 15.0);
            ctx.ui.progress_bar(self.volume, progress_rect);

            ctx.ui.label("H: Toggle UI", Vec2::new(20.0, 185.0));
            let font_status = if self.font_loaded { "Font: ON" } else { "Font: OFF" };
            ctx.ui.label(font_status, Vec2::new(140.0, 185.0));
        }
    }
}
