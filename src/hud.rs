use std::{
    rc::Rc,
    cell::RefCell,
    path::Path,
};
use rg3d::{
    core::{
        pool::Handle,
        color::Color
    },
    engine::{
        Engine,
        EngineInterfaceMut,
        resource_manager::ResourceManager
    },
    resource::{
        texture::TextureKind,
        ttf::Font
    },
    event::WindowEvent,
    gui::{
        HorizontalAlignment,
        node::UINode,
        grid::{GridBuilder, Column, Row},
        UserInterface,
        widget::WidgetBuilder,
        text::TextBuilder,
        stack_panel::StackPanelBuilder,
        image::ImageBuilder,
        scroll_bar::Orientation,
        VerticalAlignment,
        widget::AsWidget,
        Thickness,
        Visibility
    },
};

pub struct Hud {
    root: Handle<UINode>,
    health: Handle<UINode>,
    armor: Handle<UINode>,
    ammo: Handle<UINode>,
}

impl Hud {
    pub fn new(ui: &mut UserInterface, resource_manager: &mut ResourceManager, frame_size: (u32, u32)) -> Self {
        let font = Font::from_file(
            Path::new("data/ui/SquaresBold.ttf"),
            35.0,
            Font::default_char_set()).unwrap();
        let font = Rc::new(RefCell::new(font));

        let health;
        let armor;
        let ammo;

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
                .with_opt_texture(resource_manager.request_texture(Path::new("data/ui/crosshair.tga"), TextureKind::RGBA8))
                .build(ui))
            .with_child(StackPanelBuilder::new(WidgetBuilder::new()
                .with_margin(Thickness::bottom(10.0))
                .on_column(0)
                .with_vertical_alignment(VerticalAlignment::Bottom)
                .with_horizontal_alignment(HorizontalAlignment::Center)
                .with_child(ImageBuilder::new(WidgetBuilder::new()
                    .with_width(35.0)
                    .with_height(35.0))
                    .with_opt_texture(resource_manager.request_texture(Path::new("data/ui/health_icon.png"), TextureKind::RGBA8))
                    .build(ui))
                .with_child(TextBuilder::new(WidgetBuilder::new()
                    .with_width(170.0)
                    .with_height(35.0))
                    .with_text("Health:")
                    .with_font(font.clone())
                    .build(ui))
                .with_child({
                    health = TextBuilder::new(WidgetBuilder::new()
                        .with_color(Color::opaque(180, 14, 22))
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
                    .with_opt_texture(resource_manager.request_texture(Path::new("data/ui/ammo_icon.png"), TextureKind::RGBA8))
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
                        .with_color(Color::opaque(79, 79, 255))
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
                    .with_opt_texture(resource_manager.request_texture(Path::new("data/ui/shield_icon.png"), TextureKind::RGBA8))
                    .build(ui))
                .with_child(TextBuilder::new(WidgetBuilder::new()
                    .with_width(170.0)
                    .with_height(35.0))
                    .with_font(font.clone())
                    .with_text("Armor:")
                    .build(ui))
                .with_child({
                    armor = TextBuilder::new(WidgetBuilder::new()
                        .with_color(Color::opaque(255, 100, 26))
                        .with_width(170.0)
                        .with_height(35.0))
                        .with_font(font.clone())
                        .with_text("100")
                        .build(ui);
                    armor
                }))
                .with_orientation(Orientation::Horizontal)
                .build(ui)))
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
        }
    }

    pub fn set_health(&mut self, ui: &mut UserInterface, health: f32) {
        ui.get_node_mut(self.health).as_text_mut().set_text(format!("{}", health).as_str());
    }

    pub fn set_armor(&mut self, ui: &mut UserInterface, armor: f32) {
        ui.get_node_mut(self.armor).as_text_mut().set_text(format!("{}", armor).as_str());
    }

    pub fn set_ammo(&mut self, ui: &mut UserInterface, ammo: u32) {
        ui.get_node_mut(self.ammo).as_text_mut().set_text(format!("{}", ammo).as_str());
    }

    pub fn set_visible(&mut self, ui: &mut UserInterface, visible: bool) {
        ui.get_node_mut(self.root).widget_mut().set_visibility(
            if visible { Visibility::Visible } else { Visibility::Collapsed })
    }

    pub fn process_input_event(&mut self, engine: &mut Engine, event: &WindowEvent) {
        if let WindowEvent::Resized(new_size) = event {
            let EngineInterfaceMut { ui, renderer, .. } = engine.interface_mut();
            renderer.set_frame_size((*new_size).into()).unwrap();
            let root = ui.get_node_mut(self.root).widget_mut();
            root.set_width(new_size.width as f32);
            root.set_height(new_size.height as f32);
        }
    }
}