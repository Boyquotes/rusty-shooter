use std::{
    path::Path,
    sync::{Arc, Mutex},
    collections::VecDeque,
};
use rg3d::{
    core::{
        pool::Handle,
        color::Color,
    },
    engine::{
        Engine,
        resource_manager::ResourceManager,
    },
    resource::{
        texture::TextureKind,
    },
    event::{Event, WindowEvent},
    utils,
    gui::{
        ttf::Font,
        HorizontalAlignment,
        UINode,
        grid::{GridBuilder, Column, Row},
        UserInterface,
        widget::WidgetBuilder,
        text::TextBuilder,
        stack_panel::StackPanelBuilder,
        image::ImageBuilder,
        scroll_bar::Orientation,
        VerticalAlignment,
        Thickness,
        Visibility,
        text::Text,
        Builder,
        UINodeContainer,
    },
};
use crate::{
    GameTime,
    level::GameEvent
};

pub struct Hud {
    root: Handle<UINode>,
    health: Handle<UINode>,
    armor: Handle<UINode>,
    ammo: Handle<UINode>,
    message: Handle<UINode>,
    message_queue: VecDeque<String>,
    message_timeout: f32,
}

impl Hud {
    pub fn new(ui: &mut UserInterface, resource_manager: &mut ResourceManager, frame_size: (u32, u32)) -> Self {
        let font = Font::from_file(
            Path::new("data/ui/SquaresBold.ttf"),
            35.0,
            Font::default_char_set()).unwrap();
        let font = Arc::new(Mutex::new(font));

        let health;
        let armor;
        let ammo;
        let message;
        let root = GridBuilder::new(WidgetBuilder::new()
            .with_width(frame_size.0 as f32)
            .with_height(frame_size.1 as f32)
            .with_visibility(Visibility::Collapsed)
            .with_child(ImageBuilder::new(WidgetBuilder::new()
                .with_horizontal_alignment(HorizontalAlignment::Center)
                .with_vertical_alignment(VerticalAlignment::Center)
                .with_width(33.0)
                .with_height(33.0)
                .on_row(0)
                .on_column(1))
                .with_opt_texture(utils::into_any_arc(resource_manager.request_texture(Path::new("data/ui/crosshair.tga"), TextureKind::RGBA8)))
                .build(ui))
            .with_child(StackPanelBuilder::new(WidgetBuilder::new()
                .with_margin(Thickness::bottom(10.0))
                .on_column(0)
                .with_vertical_alignment(VerticalAlignment::Bottom)
                .with_horizontal_alignment(HorizontalAlignment::Center)
                .with_child(ImageBuilder::new(WidgetBuilder::new()
                    .with_width(35.0)
                    .with_height(35.0))
                    .with_opt_texture(utils::into_any_arc(resource_manager.request_texture(Path::new("data/ui/health_icon.png"), TextureKind::RGBA8)))
                    .build(ui))
                .with_child(TextBuilder::new(WidgetBuilder::new()
                    .with_width(170.0)
                    .with_height(35.0))
                    .with_text("Health:")
                    .with_font(font.clone())
                    .build(ui))
                .with_child({
                    health = TextBuilder::new(WidgetBuilder::new()
                        .with_foreground(Color::opaque(180, 14, 22))
                        .with_width(170.0)
                        .with_height(35.0))
                        .with_text("100")
                        .with_font(font.clone())
                        .build(ui);
                    health
                }))
                .with_orientation(Orientation::Horizontal)
                .build(ui))
            .with_child(StackPanelBuilder::new(WidgetBuilder::new()
                .with_margin(Thickness::bottom(10.0))
                .on_column(1)
                .with_vertical_alignment(VerticalAlignment::Bottom)
                .with_horizontal_alignment(HorizontalAlignment::Center)
                .with_child(ImageBuilder::new(WidgetBuilder::new()
                    .with_width(35.0)
                    .with_height(35.0))
                    .with_opt_texture(utils::into_any_arc(resource_manager.request_texture(Path::new("data/ui/ammo_icon.png"), TextureKind::RGBA8)))
                    .build(ui))
                .with_child(TextBuilder::new(WidgetBuilder::new()
                    .with_width(170.0)
                    .with_height(35.0))
                    .with_font(font.clone())
                    .with_text("Ammo:")
                    .build(ui)
                )
                .with_child({
                    ammo = TextBuilder::new(WidgetBuilder::new()
                        .with_foreground(Color::opaque(79, 79, 255))
                        .with_width(170.0)
                        .with_height(35.0))
                        .with_font(font.clone())
                        .with_text("40")
                        .build(ui);
                    ammo
                }))
                .with_orientation(Orientation::Horizontal)
                .build(ui))
            .with_child(StackPanelBuilder::new(WidgetBuilder::new()
                .with_margin(Thickness::bottom(10.0))
                .on_column(2)
                .with_vertical_alignment(VerticalAlignment::Bottom)
                .with_horizontal_alignment(HorizontalAlignment::Center)
                .with_child(ImageBuilder::new(WidgetBuilder::new()
                    .with_width(35.0)
                    .with_height(35.0))
                    .with_opt_texture(utils::into_any_arc(resource_manager.request_texture(Path::new("data/ui/shield_icon.png"), TextureKind::RGBA8)))
                    .build(ui))
                .with_child(TextBuilder::new(WidgetBuilder::new()
                    .with_width(170.0)
                    .with_height(35.0))
                    .with_font(font.clone())
                    .with_text("Armor:")
                    .build(ui))
                .with_child({
                    armor = TextBuilder::new(WidgetBuilder::new()
                        .with_foreground(Color::opaque(255, 100, 26))
                        .with_width(170.0)
                        .with_height(35.0))
                        .with_font(font.clone())
                        .with_text("100")
                        .build(ui);
                    armor
                }))
                .with_orientation(Orientation::Horizontal)
                .build(ui))
            .with_child({
                message = TextBuilder::new(WidgetBuilder::new()
                    .on_row(0)
                    .on_column(0)
                    .with_vertical_alignment(VerticalAlignment::Center)
                    .with_horizontal_alignment(HorizontalAlignment::Left)
                    .with_margin(Thickness {
                        left: 45.0,
                        top: 30.0,
                        right: 0.0,
                        bottom: 0.0,
                    })
                    .with_height(40.0)
                    .with_width(400.0))
                    .with_text("FOOBAR")
                    .build(ui);
                message
            }))
            .add_column(Column::stretch())
            .add_column(Column::stretch())
            .add_column(Column::stretch())
            .add_row(Row::stretch())
            .build(ui);

        Self {
            root,
            health,
            armor,
            ammo,
            message,
            message_timeout: 0.0,
            message_queue: Default::default(),
        }
    }

    pub fn set_health(&mut self, ui: &mut UserInterface, health: f32) {
        ui.node_mut(self.health)
            .downcast_mut::<Text>()
            .unwrap()
            .set_text(format!("{}", health));
    }

    pub fn set_armor(&mut self, ui: &mut UserInterface, armor: f32) {
        ui.node_mut(self.armor)
            .downcast_mut::<Text>()
            .unwrap()
            .set_text(format!("{}", armor));
    }

    pub fn set_ammo(&mut self, ui: &mut UserInterface, ammo: u32) {
        ui.node_mut(self.ammo)
            .downcast_mut::<Text>()
            .unwrap()
            .set_text(format!("{}", ammo));
    }

    pub fn set_visible(&mut self, ui: &mut UserInterface, visible: bool) {
        ui.node_mut(self.root)
            .widget_mut()
            .set_visibility(if visible {
                Visibility::Visible
            } else {
                Visibility::Collapsed
            });
    }

    pub fn add_message<P: AsRef<str>>(&mut self, message: P) {
        self.message_queue
            .push_back(message.as_ref().to_owned())
    }

    pub fn process_input_event(&mut self, engine: &mut Engine, event: &Event<()>) {
        if let Event::WindowEvent { event, .. } = event {
            if let WindowEvent::Resized(new_size) = event {
                engine.renderer
                    .set_frame_size((*new_size).into())
                    .unwrap();

                engine.user_interface
                    .node_mut(self.root)
                    .widget_mut()
                    .set_width(new_size.width as f32)
                    .set_height(new_size.height as f32);
            }
        }
    }

    pub fn update(&mut self, ui: &mut UserInterface, time: &GameTime) {
        self.message_timeout -= time.delta;

        if self.message_timeout <= 0.0 {
            if let Some(message) = self.message_queue.pop_front() {
                ui.node_mut(self.message)
                    .downcast_mut::<Text>()
                    .unwrap()
                    .set_text(message);

                self.message_timeout = 1.25;
            } else {
                ui.node_mut(self.message)
                    .downcast_mut::<Text>()
                    .unwrap()
                    .set_text("");
            }
        }
    }

    pub fn handle_game_event(&mut self, event: &GameEvent) {
        if let GameEvent::AddNotification { text } = event {
            self.add_message(text)
        }
    }
}