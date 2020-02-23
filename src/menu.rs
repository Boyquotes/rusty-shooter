use std::{
    path::Path,
    rc::Rc,
    sync::{
        Mutex,
        Arc,
        mpsc::Sender,
    },
    cell::RefCell,
};
use crate::{
    message::Message,
    match_menu::MatchMenu,
    options_menu::OptionsMenu,
    UINodeHandle,
    GameEngine,
    Gui,
    GuiMessage,
    control_scheme::ControlScheme
};
use rg3d::{
    resource::{
        texture::TextureKind,
    },
    event::{
        WindowEvent,
        Event,
    },
    gui::{
        ttf::Font,
        check_box::CheckBoxBuilder,
        Control,
        grid::{
            GridBuilder,
            Row,
            Column,
        },
        Thickness,
        VerticalAlignment,
        window::{
            WindowBuilder,
            WindowTitle,
        },
        scroll_bar::{ScrollBarBuilder, Orientation},
        button::ButtonBuilder,
        scroll_viewer::ScrollViewerBuilder,
        message::{
            UiMessage,
            UiMessageData,
            WindowMessage,
            ButtonMessage,
        },
        widget::{
            WidgetBuilder,
        },
        HorizontalAlignment,
        image::ImageBuilder,
    },
    utils,
};
use rg3d::engine::resource_manager::ResourceManager;

pub struct Menu {
    sender: Sender<Message>,
    root: UINodeHandle,
    btn_new_game: UINodeHandle,
    btn_save_game: UINodeHandle,
    btn_settings: UINodeHandle,
    btn_load_game: UINodeHandle,
    btn_quit_game: UINodeHandle,
    options_menu: OptionsMenu,
    match_menu: MatchMenu,
}

impl Menu {
    pub fn new(engine: &mut GameEngine, control_scheme: Rc<RefCell<ControlScheme>>, sender: Sender<Message>) -> Self {
        let frame_size = engine.renderer.get_frame_size();

        let font: Font = Font::from_file(
            Path::new("data/ui/SquaresBold.ttf"),
            30.0,
            Font::default_char_set()).unwrap();
        let font = Arc::new(Mutex::new(font));

        let ui = &mut engine.user_interface;

        let btn_new_game;
        let btn_settings;
        let btn_save_game;
        let btn_load_game;
        let btn_quit_game;
        let root: UINodeHandle = GridBuilder::new(WidgetBuilder::new()
            .with_width(frame_size.0 as f32)
            .with_height(frame_size.1 as f32)
            .with_child(WindowBuilder::new(WidgetBuilder::new()
                .on_row(1)
                .on_column(1))
                .can_minimize(false)
                .can_close(false)
                .with_title(WindowTitle::Text("Rusty Shooter"))
                .with_content(GridBuilder::new(WidgetBuilder::new()
                    .with_margin(Thickness::uniform(20.0))
                    .with_child({
                        btn_new_game = ButtonBuilder::new(WidgetBuilder::new()
                            .on_column(0)
                            .on_row(0)
                            .with_margin(Thickness::uniform(4.0)))
                            .with_text("New Game")
                            .with_font(font.clone())
                            .build(ui);
                        btn_new_game
                    })
                    .with_child({
                        btn_save_game = ButtonBuilder::new(WidgetBuilder::new()
                            .on_column(0)
                            .on_row(1)
                            .with_margin(Thickness::uniform(4.0)))
                            .with_text("Save Game")
                            .with_font(font.clone())
                            .build(ui);
                        btn_save_game
                    })
                    .with_child({
                        btn_load_game = ButtonBuilder::new(WidgetBuilder::new()
                            .on_column(0)
                            .on_row(2)
                            .with_margin(Thickness::uniform(4.0)))
                            .with_text("Load Game")
                            .with_font(font.clone())
                            .build(ui);
                        btn_load_game
                    })
                    .with_child({
                        btn_settings = ButtonBuilder::new(WidgetBuilder::new()
                            .on_column(0)
                            .on_row(3)
                            .with_margin(Thickness::uniform(4.0)))
                            .with_text("Settings")
                            .with_font(font.clone())
                            .build(ui);
                        btn_settings
                    })
                    .with_child({
                        btn_quit_game = ButtonBuilder::new(WidgetBuilder::new()
                            .on_column(0)
                            .on_row(4)
                            .with_margin(Thickness::uniform(4.0)))
                            .with_text("Quit")
                            .with_font(font.clone())
                            .build(ui);
                        btn_quit_game
                    }))
                    .add_column(Column::stretch())
                    .add_row(Row::strict(75.0))
                    .add_row(Row::strict(75.0))
                    .add_row(Row::strict(75.0))
                    .add_row(Row::strict(75.0))
                    .add_row(Row::strict(75.0))
                    .build(ui))
                .build(ui)))
            .add_row(Row::stretch())
            .add_row(Row::strict(500.0))
            .add_row(Row::stretch())
            .add_column(Column::stretch())
            .add_column(Column::strict(400.0))
            .add_column(Column::stretch())
            .build(ui);

        Self {
            sender: sender.clone(),
            root,
            btn_new_game,
            btn_settings,
            btn_save_game,
            btn_load_game,
            btn_quit_game,
            options_menu: OptionsMenu::new(engine, control_scheme.clone(), sender.clone()),
            match_menu: MatchMenu::new(&mut engine.user_interface, &mut engine.resource_manager, sender),
        }
    }

    pub fn set_visible(&mut self, ui: &mut Gui, visible: bool) {
        ui.node_mut(self.root)
            .widget_mut()
            .set_visibility(visible);

        if !visible {
            ui.post_message(UiMessage::targeted(self.options_menu.window, UiMessageData::Window(WindowMessage::Closed)));
            ui.post_message(UiMessage::targeted(self.match_menu.window, UiMessageData::Window(WindowMessage::Closed)));
        }
    }

    pub fn is_visible(&self, ui: &Gui) -> bool {
        ui.node(self.root)
            .widget()
            .visibility()
    }

    pub fn process_input_event(&mut self, engine: &mut GameEngine, event: &Event<()>) {
        if let Event::WindowEvent { event, .. } = event {
            if let WindowEvent::Resized(new_size) = event {
                engine.user_interface
                    .node_mut(self.root)
                    .widget_mut()
                    .set_width_mut(new_size.width as f32)
                    .set_height_mut(new_size.height as f32);
            }
        }

        self.options_menu.process_input_event(engine, event);
    }

    pub fn handle_ui_event(&mut self, engine: &mut GameEngine, event: &GuiMessage) {
        if let UiMessageData::Button(msg) = &event.data {
            if let ButtonMessage::Click = msg {
                if event.source() == self.btn_new_game {
                    engine.user_interface
                        .post_message(UiMessage::targeted(
                            self.match_menu.window, UiMessageData::Window(WindowMessage::Opened)));
                } else if event.source() == self.btn_save_game {
                    self.sender
                        .send(Message::SaveGame)
                        .unwrap();
                } else if event.source() == self.btn_load_game {
                    self.sender
                        .send(Message::LoadGame)
                        .unwrap();
                } else if event.source() == self.btn_quit_game {
                    self.sender
                        .send(Message::QuitGame)
                        .unwrap();
                } else if event.source() == self.btn_settings {
                    engine.user_interface
                        .post_message(UiMessage::targeted(
                            self.options_menu.window, UiMessageData::Window(WindowMessage::Opened)));
                }
            }
        }

        self.options_menu.handle_ui_event(engine, event);
        self.match_menu.handle_ui_event(engine, event);
    }
}