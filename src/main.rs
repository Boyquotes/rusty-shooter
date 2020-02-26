#![deny(unsafe_code)]
#![deny(unused_must_use)]

extern crate rg3d;
extern crate rand;

mod actor;
mod level;
mod player;
mod weapon;
mod bot;
mod projectile;
mod menu;
mod effects;
mod character;
mod hud;
mod jump_pad;
mod item;
mod control_scheme;
mod match_menu;
mod message;
mod options_menu;
mod gui;

use crate::{
    character::AsCharacter,
    level::{
        Level,
    },
    message::Message,
    menu::Menu,
    hud::Hud,
    actor::Actor,
    control_scheme::ControlScheme,
    gui::{CustomUiMessage, DummyUiNode}
};
use std::{
    sync::mpsc::{
        Receiver,
        Sender,
        self,
    },
    rc::Rc,
    fs::File,
    path::Path,
    time::{
        Instant,
        self,
        Duration,
    },
    io::Write,
    thread,
    cell::RefCell,
};
use rg3d::{
    utils::translate_event,
    core::{
        pool::Handle,
        visitor::{
            Visitor,
            VisitResult,
            Visit,
        },
        color::Color,
    },
    sound::{
        source::{
            Status,
            SoundSource,
            generic::GenericSourceBuilder,
        },
    },
    gui::{
        widget::WidgetBuilder,
        node::UINode,
        text::TextBuilder,
        UserInterface,
        message::UiMessage,
    },
    event::{DeviceEvent, WindowEvent, ElementState, VirtualKeyCode, Event},
    event_loop::{EventLoop, ControlFlow},
    engine::Engine,
};

// Define type aliases for engine structs.
pub type UINodeHandle = Handle<UINode<CustomUiMessage, DummyUiNode>>;
pub type GameEngine = Engine<CustomUiMessage, DummyUiNode>;
pub type Gui = UserInterface<CustomUiMessage, DummyUiNode>;
pub type GuiMessage = UiMessage<CustomUiMessage, DummyUiNode>;

pub struct Game {
    menu: Menu,
    hud: Hud,
    engine: GameEngine,
    level: Option<Level>,
    debug_text: UINodeHandle,
    debug_string: String,
    last_tick_time: time::Instant,
    music: Handle<SoundSource>,
    running: bool,
    control_scheme: Rc<RefCell<ControlScheme>>,
    time: GameTime,
    events_receiver: Receiver<Message>,
    events_sender: Sender<Message>,
}

#[derive(Copy, Clone)]
pub struct GameTime {
    clock: time::Instant,
    elapsed: f64,
    delta: f32,
}

pub enum CollisionGroups {
    Generic = 1 << 0,
    Projectile = 1 << 1,
    Actor = 1 << 2,
    All = std::isize::MAX,
}

#[derive(Copy, Clone, Debug)]
pub struct DeathMatch {
    pub time_limit_secs: f32,
    pub frag_limit: u32,
}

impl Default for DeathMatch {
    fn default() -> Self {
        Self {
            time_limit_secs: Default::default(),
            frag_limit: 0,
        }
    }
}

impl Visit for DeathMatch {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.time_limit_secs.visit("TimeLimit", visitor)?;
        self.frag_limit.visit("FragLimit", visitor)?;

        visitor.leave_region()
    }
}

#[derive(Copy, Clone, Debug)]
pub struct TeamDeathMatch {
    pub time_limit_secs: f32,
    pub team_frag_limit: u32,
}

impl Default for TeamDeathMatch {
    fn default() -> Self {
        Self {
            time_limit_secs: Default::default(),
            team_frag_limit: 0,
        }
    }
}

impl Visit for TeamDeathMatch {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.time_limit_secs.visit("TimeLimit", visitor)?;
        self.team_frag_limit.visit("TeamFragLimit", visitor)?;

        visitor.leave_region()
    }
}

#[derive(Copy, Clone, Debug)]
pub struct CaptureTheFlag {
    pub time_limit_secs: f32,
    pub flag_limit: u32,
}

impl Default for CaptureTheFlag {
    fn default() -> Self {
        Self {
            time_limit_secs: Default::default(),
            flag_limit: 0,
        }
    }
}

impl Visit for CaptureTheFlag {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.time_limit_secs.visit("TimeLimit", visitor)?;
        self.flag_limit.visit("FlagLimit", visitor)?;

        visitor.leave_region()
    }
}

#[derive(Copy, Clone, Debug)]
pub enum MatchOptions {
    DeathMatch(DeathMatch),
    TeamDeathMatch(TeamDeathMatch),
    CaptureTheFlag(CaptureTheFlag),
}

impl MatchOptions {
    fn from_id(id: u32) -> Result<Self, String> {
        match id {
            0 => Ok(MatchOptions::DeathMatch(Default::default())),
            1 => Ok(MatchOptions::TeamDeathMatch(Default::default())),
            2 => Ok(MatchOptions::CaptureTheFlag(Default::default())),
            _ => Err(format!("Invalid match options {}", id))
        }
    }

    fn id(&self) -> u32 {
        match self {
            MatchOptions::DeathMatch(_) => 0,
            MatchOptions::TeamDeathMatch(_) => 1,
            MatchOptions::CaptureTheFlag(_) => 2,
        }
    }
}

impl Default for MatchOptions {
    fn default() -> Self {
        MatchOptions::DeathMatch(Default::default())
    }
}

impl Visit for MatchOptions {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        let mut id = self.id();
        id.visit("Id", visitor)?;
        if visitor.is_reading() {
            *self = Self::from_id(id)?;
        }
        match self {
            MatchOptions::DeathMatch(o) => o.visit("Data", visitor)?,
            MatchOptions::TeamDeathMatch(o) => o.visit("Data", visitor)?,
            MatchOptions::CaptureTheFlag(o) => o.visit("Data", visitor)?,
        }

        visitor.leave_region()
    }
}

impl Game {
    pub fn run() {
        let events_loop = EventLoop::<()>::new();

        let primary_monitor = events_loop.primary_monitor();
        let mut monitor_dimensions = primary_monitor.size();
        monitor_dimensions.height = (monitor_dimensions.height as f32 * 0.7) as u32;
        monitor_dimensions.width = (monitor_dimensions.width as f32 * 0.7) as u32;
        let inner_size = monitor_dimensions.to_logical::<f32>(primary_monitor.scale_factor());

        let window_builder = rg3d::window::WindowBuilder::new()
            .with_title("Rusty Shooter")
            .with_inner_size(inner_size)
            .with_resizable(true);

        let mut engine = GameEngine::new(window_builder, &events_loop).unwrap();
        let hrtf_sphere = rg3d::sound::hrtf::HrtfSphere::new("data/sounds/IRC_1040_C.bin").unwrap();
        engine.sound_context
            .lock()
            .unwrap()
            .set_renderer(rg3d::sound::renderer::Renderer::HrtfRenderer(rg3d::sound::hrtf::HrtfRenderer::new(hrtf_sphere)));

        effects::register_custom_emitter_factory();

        engine.renderer.set_ambient_color(Color::opaque(60, 60, 60));

        let buffer = engine.resource_manager.request_sound_buffer("data/sounds/Antonio_Bizarro_Berzerker.ogg", true).unwrap();
        let music = engine.sound_context
            .lock()
            .unwrap()
            .add_source(GenericSourceBuilder::new(buffer)
                .with_looping(true)
                .with_status(Status::Playing)
                .with_gain(0.25)
                .build_source()
                .unwrap());

        let mut reverb = rg3d::sound::effects::reverb::Reverb::new();
        reverb.set_decay_time(Duration::from_secs_f32(5.0));
        engine.sound_context
            .lock()
            .unwrap()
            .add_effect(rg3d::sound::effects::Effect::Reverb(reverb));

        let control_scheme = Rc::new(RefCell::new(ControlScheme::default()));

        let fixed_fps = 60.0;
        let fixed_timestep = 1.0 / fixed_fps;

        let time = GameTime {
            clock: Instant::now(),
            elapsed: 0.0,
            delta: fixed_timestep,
        };

        let (tx, rx) = mpsc::channel();

        let mut game = Game {
            hud: Hud::new(&mut engine),
            running: true,
            menu: Menu::new(&mut engine, control_scheme.clone(), tx.clone()),
            control_scheme,
            debug_text: Handle::NONE,
            engine,
            level: None,
            debug_string: String::new(),
            last_tick_time: time::Instant::now(),
            music,
            time,
            events_receiver: rx,
            events_sender: tx,
        };

        game.create_debug_ui();

        events_loop.run(move |event, _, control_flow| {
            game.process_input_event(&event);

            match event {
                Event::MainEventsCleared => {
                    let mut dt = game.time.clock.elapsed().as_secs_f64() - game.time.elapsed;
                    while dt >= fixed_timestep as f64 {
                        dt -= fixed_timestep as f64;
                        game.time.elapsed += fixed_timestep as f64;

                        while let Some(mut ui_event) = game.engine.get_ui_mut().poll_message() {
                            game.menu.handle_ui_event(&mut game.engine, &mut ui_event);
                        }

                        game.update(game.time);
                    }
                    if !game.running {
                        *control_flow = ControlFlow::Exit;
                    }
                    game.engine.get_window().request_redraw();
                }
                Event::RedrawRequested(_) => {
                    game.update_statistics(game.time.elapsed);

                    // <<<<< ENABLE THIS TO SHOW DEBUG GEOMETRY >>>>>
                    if false {
                        game.debug_render();
                    }

                    // Render at max speed
                    game.engine.render().unwrap();
                    // Make sure to cap update rate to 60 FPS.
                    game.limit_fps(fixed_fps as f64);
                }
                Event::WindowEvent { event, .. } => {
                    match event {
                        WindowEvent::CloseRequested => {
                            game.destroy_level();
                            *control_flow = ControlFlow::Exit
                        }
                        WindowEvent::Resized(new_size) => {
                            game.engine
                                .renderer
                                .set_frame_size(new_size.into())
                                .unwrap();
                        }
                        _ => ()
                    }
                }
                _ => *control_flow = ControlFlow::Poll,
            }
        });
    }

    fn debug_render(&mut self) {
        self.engine.renderer.debug_renderer.clear_lines();

        if let Some(level) = self.level.as_mut() {
            level.debug_draw(&mut self.engine);
        }
    }

    pub fn create_debug_ui(&mut self) {
        self.debug_text = TextBuilder::new(WidgetBuilder::new()
            .with_width(400.0))
            .build(&mut self.engine.user_interface);
    }

    pub fn save_game(&mut self) -> VisitResult {
        let mut visitor = Visitor::new();

        // Visit engine state first.
        self.engine.visit("GameEngine", &mut visitor)?;

        self.level.visit("Level", &mut visitor)?;

        // Debug output
        if let Ok(mut file) = File::create(Path::new("save.txt")) {
            file.write_all(visitor.save_text().as_bytes()).unwrap();
        }

        visitor.save_binary(Path::new("save.bin"))
    }

    pub fn load_game(&mut self) {
        println!("Attempting load a save...");
        match Visitor::load_binary(Path::new("save.bin")) {
            Ok(mut visitor) => {
                // Clean up.
                self.destroy_level();

                // Load engine state first
                println!("Trying to load engine state...");
                match self.engine.visit("GameEngine", &mut visitor) {
                    Ok(_) => {
                        println!("GameEngine state successfully loaded!");

                        // Then load game state.
                        match self.level.visit("Level", &mut visitor) {
                            Ok(_) => {
                                println!("Game state successfully loaded!");

                                // Hide menu only of we successfully loaded a save.
                                self.set_menu_visible(false);

                                // Set control scheme for player.
                                if let Some(level) = &mut self.level {
                                    level.set_message_sender(self.events_sender.clone());
                                    level.build_navmesh(&mut self.engine);
                                    level.control_scheme = Some(self.control_scheme.clone());
                                    let player = level.get_player();
                                    if let Actor::Player(player) = level.get_actors_mut().get_mut(player) {
                                        player.set_control_scheme(self.control_scheme.clone());
                                    }
                                }

                                self.time.elapsed = self.time.clock.elapsed().as_secs_f64();
                            }
                            Err(e) => println!("Failed to load game state! Reason: {}", e)
                        }
                    }
                    Err(e) => println!("Failed to load engine state! Reason: {}", e)
                }
            }
            Err(e) => {
                println!("failed to load a save, reason: {}", e);
            }
        }
    }

    fn destroy_level(&mut self) {
        if let Some(ref mut level) = self.level.take() {
            level.destroy(&mut self.engine);
            println!("Current level destroyed!");
        }
    }

    pub fn start_new_game(&mut self, options: MatchOptions) {
        self.destroy_level();
        self.level = Some(Level::new(
            &mut self.engine,
            self.control_scheme.clone(),
            self.events_sender.clone(),
            options,
        ));
        self.set_menu_visible(false);
    }

    pub fn set_menu_visible(&mut self, visible: bool) {
        let ui = &mut self.engine.user_interface;
        self.menu.set_visible(ui, visible);
        self.hud.set_visible(ui, !visible);
    }

    pub fn is_menu_visible(&self) -> bool {
        self.menu.is_visible(&self.engine.user_interface)
    }

    pub fn update(&mut self, time: GameTime) {
        let window = self.engine.get_window();
        window.set_cursor_visible(self.is_menu_visible());
        let _ = window.set_cursor_grab(!self.is_menu_visible());

        self.engine.update(time.delta);

        if let Some(ref mut level) = self.level {
            level.update(&mut self.engine, time);
            let ui = &mut self.engine.user_interface;
            self.hud.set_time(ui, level.time());
            let player = level.get_player();
            if player.is_some() {
                // Sync hud with player state.
                let player = level.get_actors().get(player);
                self.hud.set_health(ui, player.character().get_health());
                self.hud.set_armor(ui, player.character().get_armor());
                let current_weapon = player.character().get_current_weapon();
                if current_weapon.is_some() {
                    let current_weapon = level.get_weapons().get(current_weapon);
                    self.hud.set_ammo(ui, current_weapon.get_ammo());
                }
            }
        }

        self.handle_messages(time);

        self.hud.update(&mut self.engine.user_interface, &self.time);
    }

    fn handle_messages(&mut self, time: GameTime) {
        while let Ok(message) = self.events_receiver.try_recv() {
            match &message {
                Message::StartNewGame { options } => {
                    self.start_new_game(*options);
                }
                Message::SaveGame => {
                    match self.save_game() {
                        Ok(_) => println!("successfully saved"),
                        Err(e) => println!("failed to make a save, reason: {}", e),
                    }
                }
                Message::LoadGame => {
                    self.load_game();
                }
                Message::QuitGame => {
                    self.destroy_level();
                    self.running = false;
                }
                Message::SetMusicVolume { volume } => {
                    self.engine
                        .sound_context
                        .lock()
                        .unwrap()
                        .source_mut(self.music)
                        .generic_mut()
                        .set_gain(*volume);
                }
                _ => ()
            }

            if let Some(ref mut level) = self.level {
                level.handle_message(&mut self.engine, &message, time);

                self.hud.handle_message(&message, &mut self.engine.user_interface, &level.leader_board, &level.options);
            }
        }
    }

    pub fn update_statistics(&mut self, elapsed: f64) {
        self.debug_string.clear();
        use std::fmt::Write;
        let statistics = self.engine.renderer.get_statistics();
        write!(self.debug_string,
               "Pure frame time: {:.2} ms\n\
               Capped frame time: {:.2} ms\n\
               FPS: {}\n\
               Triangles: {}\n\
               Draw calls: {}\n\
               Up time: {:.2} s\n\
               Sound render time: {:?}\n\
               UI Time: {:?}",
               statistics.pure_frame_time * 1000.0,
               statistics.capped_frame_time * 1000.0,
               statistics.frames_per_second,
               statistics.geometry.triangles_rendered,
               statistics.geometry.draw_calls,
               elapsed,
               self.engine.sound_context.lock().unwrap().full_render_duration(),
               self.engine.ui_time
        ).unwrap();

        if let UINode::Text(text) = self.engine.user_interface.node_mut(self.debug_text) {
            text.set_text(self.debug_string.as_str());
        }
    }

    pub fn limit_fps(&mut self, value: f64) {
        let current_time = time::Instant::now();
        let render_call_duration = current_time.duration_since(self.last_tick_time).as_secs_f64();
        self.last_tick_time = current_time;
        let desired_frame_time = 1.0 / value;
        if render_call_duration < desired_frame_time {
            thread::sleep(Duration::from_secs_f64(desired_frame_time - render_call_duration));
        }
    }

    fn process_dispatched_event(&mut self, event: &Event<()>) {
        if let Event::WindowEvent { event, .. } = event {
            if let Some(event) = translate_event(event) {
                self.engine
                    .user_interface
                    .process_os_event(&event);
            }
        }

        if !self.is_menu_visible() {
            if let Some(ref mut level) = self.level {
                level.process_input_event(event);
            }
        }
    }

    pub fn process_input_event(&mut self, event: &Event<()>) {
        self.process_dispatched_event(event);

        if let Event::DeviceEvent { event, .. } = event {
            if let DeviceEvent::Key(input) = event {
                if let ElementState::Pressed = input.state {
                    if let Some(key) = input.virtual_keycode {
                        if key == VirtualKeyCode::Escape {
                            self.set_menu_visible(!self.is_menu_visible());
                        }
                    }
                }
            }
        }

        self.menu.process_input_event(&mut self.engine, &event);
        self.hud.process_input_event(&mut self.engine, &event);
    }
}

fn main() {
    Game::run();
}