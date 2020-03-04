use std::collections::HashMap;
use crate::{
    MatchOptions,
    UINodeHandle,
    GameEngine,
    Gui,
    character::Team,
    message::Message
};
use rg3d::{
    event::{WindowEvent, ElementState, VirtualKeyCode, Event},
    gui::{
        grid::{GridBuilder, Row, Column},
        widget::WidgetBuilder,
        text::TextBuilder,
        Thickness,
        HorizontalAlignment,
        VerticalAlignment,
        brush::Brush,
        Control
    },
    core::{
        visitor::{Visit, VisitResult, Visitor},
        color::Color
    },
};

#[derive(Copy, Clone)]
pub struct PersonalScore {
    pub kills: u32,
    pub deaths: u32,
}

impl Default for PersonalScore {
    fn default() -> Self {
        Self {
            kills: 0,
            deaths: 0,
        }
    }
}

impl Visit for PersonalScore {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.kills.visit("Kills", visitor)?;
        self.deaths.visit("Deaths", visitor)?;

        visitor.leave_region()
    }
}

pub struct LeaderBoard {
    personal_score: HashMap<String, PersonalScore>,
    team_score: HashMap<Team, u32>,
}

impl LeaderBoard {
    pub fn get_or_add_actor<P: AsRef<str>>(&mut self, actor_name: P) -> &mut PersonalScore {
        self.personal_score
            .entry(actor_name.as_ref().to_owned())
            .or_insert(Default::default())
    }

    pub fn remove_actor<P: AsRef<str>>(&mut self, actor_name: P) {
        self.personal_score.remove(actor_name.as_ref());
    }

    pub fn add_frag<P: AsRef<str>>(&mut self, actor_name: P) {
        self.get_or_add_actor(actor_name).kills += 1;
    }

    pub fn add_death<P: AsRef<str>>(&mut self, actor_name: P) {
        self.get_or_add_actor(actor_name).deaths += 1;
    }

    pub fn score_of<P: AsRef<str>>(&self, actor_name: P) -> u32 {
        match self.personal_score.get(actor_name.as_ref()) {
            None => 0,
            Some(value) => value.kills,
        }
    }

    pub fn add_team_frag(&mut self, team: Team) {
        *self.team_score.entry(team).or_insert(0) += 1;
    }

    pub fn team_score(&self, team: Team) -> u32 {
        match self.team_score.get(&team) {
            None => 0,
            Some(score) => *score,
        }
    }

    /// Returns record about leader as a pair of character name and its score.
    /// `except` parameter can be used to exclude already found leader and search
    /// for a character at second place.
    pub fn highest_personal_score(&self, except: Option<&str>) -> Option<(&str, u32)> {
        let mut pair = None;

        for (name, score) in self.personal_score.iter() {
            if let Some(except) = except {
                if name == except {
                    continue;
                }
            }
            match pair {
                None => pair = Some((name.as_str(), score.kills)),
                Some(ref mut pair) => {
                    if score.kills > pair.1 {
                        *pair = (name.as_str(), score.kills)
                    }
                }
            }
        }

        pair
    }

    pub fn values(&self) -> &HashMap<String, PersonalScore> {
        &self.personal_score
    }
}

impl Default for LeaderBoard {
    fn default() -> Self {
        Self {
            personal_score: Default::default(),
            team_score: Default::default(),
        }
    }
}

impl Visit for LeaderBoard {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.personal_score.visit("PersonalScore", visitor)?;
        self.team_score.visit("TeamScore", visitor)?;

        visitor.leave_region()
    }
}

pub struct LeaderBoardUI {
    root: UINodeHandle
}

impl LeaderBoardUI {
    pub fn new(engine: &mut GameEngine) -> Self {
        let frame_size = engine.renderer.get_frame_size();

        let ui = &mut engine.user_interface;

        let root: UINodeHandle = GridBuilder::new(WidgetBuilder::new()
            .with_visibility(false)
            .with_width(frame_size.0 as f32)
            .with_height(frame_size.1 as f32))
            .add_row(Row::stretch())
            .add_row(Row::strict(600.0))
            .add_row(Row::stretch())
            .add_column(Column::stretch())
            .add_column(Column::strict(500.0))
            .add_column(Column::stretch())
            .build(ui);
        Self {
            root
        }
    }

    pub fn sync_to_model(&mut self,
                         ui: &mut Gui,
                         leader_board: &LeaderBoard,
                         match_options: &MatchOptions,
    ) {
        // Rebuild entire table, this is far from ideal but it is simplest solution.
        // Shouldn't be a big problem because this method should be called once anything
        // changes in leader board.
        // TODO: Remove unnecessary rebuild of table.

        let row_template = Row::strict(30.0);

        let mut children = Vec::new();

        for (i, (name, score)) in leader_board.values().iter().enumerate() {
            let row = i + 1;

            children.push(TextBuilder::new(WidgetBuilder::new()
                .with_margin(Thickness::uniform(3.0))
                .on_row(row)
                .on_column(0))
                .with_text(name)
                .build(ui));

            children.push(TextBuilder::new(WidgetBuilder::new()
                .with_margin(Thickness::uniform(3.0))
                .on_row(row)
                .on_column(1))
                .with_text(format!("{}", score.kills))
                .build(ui));

            children.push(TextBuilder::new(WidgetBuilder::new()
                .with_margin(Thickness::uniform(3.0))
                .on_row(row)
                .on_column(2))
                .with_text(format!("{}", score.deaths))
                .build(ui));

            let kd = if score.deaths != 0 {
                format!("{}", score.kills as f32 / score.deaths as f32)
            } else {
                "N/A".to_owned()
            };

            children.push(TextBuilder::new(WidgetBuilder::new()
                .with_margin(Thickness::uniform(3.0))
                .on_row(row)
                .on_column(3))
                .with_text(kd)
                .build(ui));
        }

        let table = GridBuilder::new(WidgetBuilder::new()
            .on_row(1)
            .on_column(1)
            .with_background(Brush::Solid(Color::BLACK))
            .with_child(TextBuilder::new(WidgetBuilder::new()
                .on_column(0)
                .on_row(0)
                .with_horizontal_alignment(HorizontalAlignment::Center))
                .with_text({
                    let time_limit_secs = match match_options {
                        MatchOptions::DeathMatch(dm) => dm.time_limit_secs,
                        MatchOptions::TeamDeathMatch(tdm) => tdm.time_limit_secs,
                        MatchOptions::CaptureTheFlag(ctf) => ctf.time_limit_secs,
                    };

                    let seconds = (time_limit_secs % 60.0) as u32;
                    let minutes = (time_limit_secs / 60.0) as u32;
                    let hours = (time_limit_secs / 3600.0) as u32;

                    match match_options {
                        MatchOptions::DeathMatch(_) => format!("Death Match - Time Limit {:02}:{:02}:{:02}", hours, minutes, seconds),
                        MatchOptions::TeamDeathMatch(_) => format!("Team Death Match - Time Limit {:02}:{:02}:{:02}", hours, minutes, seconds),
                        MatchOptions::CaptureTheFlag(_) => format!("Capture The Flag - Time Limit {:02}:{:02}:{:02}", hours, minutes, seconds),
                    }
                })
                .build(ui))
            .with_child({
                match match_options {
                    MatchOptions::DeathMatch(dm) => {
                        let text = if let Some((name, kills)) = leader_board.highest_personal_score(None) {
                            format!("{} leads with {} frags\nPlaying until {} frags", name, kills, dm.frag_limit)
                        } else {
                            format!("Draw\nPlaying until {} frags", dm.frag_limit)
                        };
                        TextBuilder::new(WidgetBuilder::new()
                            .with_margin(Thickness::uniform(5.0))
                            .with_horizontal_alignment(HorizontalAlignment::Center)
                            .on_column(0)
                            .on_row(1))
                            .with_text(text)
                            .build(ui)
                    }
                    MatchOptions::TeamDeathMatch(tdm) => {
                        let red_score = leader_board.team_score(Team::Red);
                        let blue_score = leader_board.team_score(Team::Blue);

                        TextBuilder::new(WidgetBuilder::new()
                            .with_margin(Thickness::uniform(5.0))
                            .with_horizontal_alignment(HorizontalAlignment::Center)
                            .on_column(0)
                            .on_row(1))
                            .with_text(format!("{} team leads\nRed {} - {} Blue\nPlaying until {} frags",
                                               if red_score > blue_score { "Red" } else { "Blue" }, red_score, blue_score, tdm.team_frag_limit))
                            .build(ui)
                    }
                    MatchOptions::CaptureTheFlag(ctf) => {
                        // TODO - implement when CTF mode implemented
                        TextBuilder::new(WidgetBuilder::new()
                            .with_margin(Thickness::uniform(5.0))
                            .with_horizontal_alignment(HorizontalAlignment::Center)
                            .on_column(0)
                            .on_row(1))
                            .with_text(format!("Red team leads\nRed 0 - 0 Blue\nPlaying until {} flags", ctf.flag_limit))
                            .build(ui)
                    }
                }
            })
            .with_child(GridBuilder::new(WidgetBuilder::new()
                .on_column(0)
                .on_row(2)
                .with_foreground(Brush::Solid(Color::opaque(120, 120, 120)))
                .with_child(TextBuilder::new(WidgetBuilder::new()
                    .with_horizontal_alignment(HorizontalAlignment::Center)
                    .with_vertical_alignment(VerticalAlignment::Center)
                    .on_column(0)
                    .on_row(0))
                    .with_text("Name")
                    .build(ui))
                .with_child(TextBuilder::new(WidgetBuilder::new()
                    .with_horizontal_alignment(HorizontalAlignment::Center)
                    .with_vertical_alignment(VerticalAlignment::Center)
                    .on_column(1)
                    .on_row(0))
                    .with_text("Kills")
                    .build(ui))
                .with_child(TextBuilder::new(WidgetBuilder::new()
                    .with_horizontal_alignment(HorizontalAlignment::Center)
                    .with_vertical_alignment(VerticalAlignment::Center)
                    .on_column(2)
                    .on_row(0))
                    .with_text("Deaths")
                    .build(ui))
                .with_child(TextBuilder::new(WidgetBuilder::new()
                    .with_horizontal_alignment(HorizontalAlignment::Center)
                    .with_vertical_alignment(VerticalAlignment::Center)
                    .on_column(3)
                    .on_row(0))
                    .with_text("K/D")
                    .build(ui))
                .with_children(&children))
                .with_border_thickness(2.0)
                .add_row(Row::strict(30.0))
                .add_rows((0..leader_board.values().len()).map(|_| row_template).collect())
                .add_row(Row::stretch())
                .add_column(Column::stretch())
                .add_column(Column::stretch())
                .add_column(Column::stretch())
                .add_column(Column::stretch())
                .draw_border(true)
                .build(ui)))
            .add_column(Column::auto())
            .add_row(Row::auto())
            .add_row(Row::auto())
            .add_row(Row::stretch())
            .build(ui);

        if let Some(table) = ui.node(self.root).widget().children().first() {
            let table = *table;
            ui.remove_node(table);
        }
        ui.link_nodes(table, self.root);
    }

    pub fn process_input_event(&mut self, engine: &mut GameEngine, event: &Event<()>) {
        if let Event::WindowEvent { event, .. } = event {
            match event {
                WindowEvent::Resized(new_size) => {
                    engine.user_interface
                        .node_mut(self.root)
                        .widget_mut()
                        .set_width_mut(new_size.width as f32)
                        .set_height_mut(new_size.height as f32);
                }
                WindowEvent::KeyboardInput { input, .. } => {
                    if let Some(vk) = input.virtual_keycode {
                        if vk == VirtualKeyCode::Tab {
                            let visible = match input.state {
                                ElementState::Pressed => true,
                                ElementState::Released => false,
                            };

                            engine.user_interface
                                .node_mut(self.root)
                                .widget_mut()
                                .set_visibility(visible);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    pub fn handle_message(&mut self, message: &Message, ui: &mut Gui, leader_board: &LeaderBoard, match_options: &MatchOptions) {
        match message {
            Message::AddBot { .. } => self.sync_to_model(ui, leader_board, match_options),
            Message::RemoveActor { .. } => self.sync_to_model(ui, leader_board, match_options),
            Message::SpawnBot { .. } => self.sync_to_model(ui, leader_board, match_options),
            Message::SpawnPlayer => self.sync_to_model(ui, leader_board, match_options),
            Message::RespawnActor { .. } => self.sync_to_model(ui, leader_board, match_options),
            _ => ()
        }
    }
}