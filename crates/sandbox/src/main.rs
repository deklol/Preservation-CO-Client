// Source: @digitalm1nd on x.com / _dek on Discord — Preservation Conquer project — https://discord.gg/CvKPXEHYRY
mod action;
mod assets;
mod character;
mod labels;
mod map;
mod minimap;
mod motion_math;
mod movement;
mod renderer;
mod validation;
use action::Action;
use assets::{Assets, Result};
use std::{path::PathBuf, sync::Arc, time::Instant};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};
struct App {
    assets: Assets,
    map: map::Map,
    character: character::Character,
    player: movement::Player,
    window: Option<Arc<Window>>,
    renderer: Option<renderer::Renderer>,
    cursor: [f32; 2],
    started: Instant,
    last: Instant,
    error: Option<String>,
    modifiers: winit::keyboard::ModifiersState,
    held: bool,
    queued_jump: Option<[i32; 2]>,
}
impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let result = (|| -> Result<_> {
            let window = Arc::new(
                event_loop.create_window(
                    Window::default_attributes()
                        .with_title("dek — offline Twin City sandbox")
                        .with_inner_size(winit::dpi::PhysicalSize::new(1366, 768)),
                )?,
            );
            let renderer = pollster::block_on(renderer::Renderer::new(window.clone()))?;
            Ok((window, renderer))
        })();
        match result {
            Ok((window, renderer)) => {
                self.window = Some(window);
                self.renderer = Some(renderer);
                self.last = Instant::now();
            }
            Err(e) => {
                self.error = Some(e.to_string());
                event_loop.exit();
            }
        }
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size.width, size.height)
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = [position.x as f32, position.y as f32]
            }
            WindowEvent::ModifiersChanged(modifiers) => self.modifiers = modifiers.state(),
            WindowEvent::Focused(false) => {
                self.held = false;
                self.queued_jump = None;
                self.player.stop();
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => {
                if self.renderer.as_mut().is_some_and(|renderer| {
                    renderer.minimap_pointer(self.cursor, state == ElementState::Pressed)
                }) {
                    self.held = false;
                    return;
                }
                self.held = state == ElementState::Pressed;
                if self.held {
                    self.move_cursor();
                }
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                match event.physical_key {
                    PhysicalKey::Code(KeyCode::Escape) => event_loop.exit(),
                    PhysicalKey::Code(KeyCode::Space) => self.player.stop(),
                    PhysicalKey::Code(KeyCode::Slash) => self.player.running = !self.player.running,
                    _ => {}
                }
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                self.player
                    .tick(now.duration_since(self.last).as_secs_f32());
                self.last = now;
                if !self.player.moving()
                    && let Some(target) = self.queued_jump.take()
                {
                    self.player
                        .jump(&self.map, target, self.character.duration(Action::Jump));
                } else if self.held && !self.player.moving() {
                    self.move_cursor();
                }
                if let Some(renderer) = &mut self.renderer
                    && let Err(error) = renderer.draw(
                        &mut self.assets,
                        &self.map,
                        &self.character,
                        &self.player,
                        self.started.elapsed().as_secs_f32(),
                    )
                {
                    self.error = Some(error.to_string());
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }
    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}
impl App {
    fn move_cursor(&mut self) {
        if let Some(renderer) = &self.renderer {
            let camera = renderer.camera(&self.map, &self.player);
            let target = self
                .map
                .unproject([self.cursor[0] + camera[0], self.cursor[1] + camera[1]])
                .map(|v| v.round() as i32);
            if self.modifiers.control_key() {
                self.player.stop();
                if self.player.moving() {
                    self.queued_jump = Some(target);
                    return;
                }
                self.player
                    .jump(&self.map, target, self.character.duration(Action::Jump));
            } else if self.modifiers.shift_key() {
                self.player.face(target);
            } else {
                self.player.go(&self.map, target);
            }
        }
    }
}
fn run() -> Result<()> {
    let mut args = std::env::args_os().skip(1);
    let mut root = None;
    let mut map_id = 1002;
    let mut body =
        sandbox_data::ini::SectionCatalog::parse(include_bytes!("../../../character.ini"))
            .section("Character")
            .and_then(|section| section.first("Body"))
            .unwrap_or("3")
            .parse::<u32>()?;
    let mut spawn = None;
    let mut check = false;
    while let Some(arg) = args.next() {
        match arg.to_str().ok_or("Invalid argument encoding")? {
            "--assets" => {
                root = Some(PathBuf::from(
                    args.next().ok_or("--assets requires a directory")?,
                ))
            }
            "--map" => {
                map_id = args
                    .next()
                    .ok_or("--map requires an ID")?
                    .to_str()
                    .ok_or("Invalid map")?
                    .parse()?
            }
            "--body" => {
                body = args
                    .next()
                    .ok_or("--body requires 1..4")?
                    .to_str()
                    .ok_or("Invalid body")?
                    .parse()?
            }
            "--spawn" => {
                let x = args
                    .next()
                    .ok_or("--spawn requires X Y")?
                    .to_str()
                    .ok_or("Invalid X")?
                    .parse()?;
                let y = args
                    .next()
                    .ok_or("--spawn requires X Y")?
                    .to_str()
                    .ok_or("Invalid Y")?
                    .parse()?;
                spawn = Some([x, y]);
            }
            "--check-assets" => check = true,
            "--help" | "-h" => {
                println!(
                    "You must supply your own Conquer Online 5065 installation.\nconquer-sandbox --assets <installation> [--map 1002] [--spawn X Y] [--body 1..4] [--check-assets]\nOffline only. Click to run; Space stops; Escape exits. No bundled assets."
                );
                return Ok(());
            }
            _ => return Err("Unknown argument; use --help".into()),
        }
    }
    let mut assets = Assets::open(
        &root.ok_or("You must supply your own 5065 installation: --assets <directory>")?,
    )?;
    let map = map::Map::load(&mut assets, map_id)?;
    let character = character::Character::load(&mut assets, body)?;
    let spawn = spawn.or(if map_id == 1002 {
        Some([430, 380])
    } else {
        None
    });
    let player = movement::Player::new(&map, spawn)?;
    if check {
        for part in &character.parts {
            assets.image(&part.texture)?;
        }
        validation::movement(&map, &character, player.cell())?;
        if let Some(minimap) = minimap::Minimap::load(&mut assets, map.id)? {
            let center = map.project(player.position);
            let quads = minimap.quads(
                &map,
                player.position,
                [center[0] - 683.0, center[1] - 384.0],
                [1366.0, 768.0],
            );
            if quads.len() != 4 {
                return Err("Minimap crop/hero/control check failed".into());
            }
            println!("Minimap OK: original assets, projected hero, crop and controls");
        }
        println!(
            "Assets OK: map {map_id}, {}x{}, {} sprites; character dek at {:?}",
            map.data.width,
            map.data.height,
            map.sprites.len(),
            player.cell()
        );
        return Ok(());
    }
    let mut app = App {
        assets,
        map,
        character,
        player,
        window: None,
        renderer: None,
        cursor: [0.0; 2],
        started: Instant::now(),
        last: Instant::now(),
        error: None,
        modifiers: Default::default(),
        held: false,
        queued_jump: None,
    };
    EventLoop::new()?.run_app(&mut app)?;
    if let Some(error) = app.error {
        return Err(error.into());
    }
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("Sandbox: {error}");
        std::process::exit(1);
    }
}
