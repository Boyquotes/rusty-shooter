use crate::{
    actor::{Actor, ActorContainer},
    bot::{Bot, BotKind},
    control_scheme::ControlScheme,
    effects,
    item::{Item, ItemContainer, ItemKind},
    jump_pad::{JumpPad, JumpPadContainer},
    leader_board::LeaderBoard,
    message::Message,
    player::Player,
    projectile::{Projectile, ProjectileContainer, ProjectileKind},
    weapon::{Weapon, WeaponContainer, WeaponKind},
    GameTime, MatchOptions,
};
use rg3d::core::algebra::Point3;
use rg3d::engine::Engine;
use rg3d::{
    core::{
        algebra::{Matrix3, Vector3},
        color::Color,
        math::{aabb::AxisAlignedBoundingBox, ray::Ray, PositionProvider, Vector3Ext},
        pool::Handle,
        rand::Rng,
        visitor::{Visit, VisitResult, Visitor},
    },
    engine::resource_manager::{MaterialSearchOptions, ResourceManager},
    event::Event,
    physics3d::{
        rapier::{
            geometry::{ContactEvent, InteractionGroups, IntersectionEvent},
            pipeline::ChannelEventCollector,
        },
        RayCastOptions,
    },
    rand,
    scene::{self, base::BaseBuilder, camera::CameraBuilder, node::Node, Scene},
    sound::{
        context,
        context::SoundContext,
        effects::{BaseEffect, Effect, EffectInput},
        source::{generic::GenericSourceBuilder, spatial::SpatialSourceBuilder, Status},
    },
    utils::{
        log::{Log, MessageKind},
        navmesh::Navmesh,
    },
};
use std::{
    path::{Path, PathBuf},
    sync::{mpsc::Sender, Arc, RwLock},
    time::Duration,
};

pub const RESPAWN_TIME: f32 = 4.0;

#[derive(Default)]
pub struct SoundManager {
    context: SoundContext,
    reverb: Handle<Effect>,
}

impl SoundManager {
    pub fn new(context: SoundContext) -> Self {
        let mut base_effect = BaseEffect::default();
        base_effect.set_gain(0.7);
        let mut reverb = rg3d::sound::effects::reverb::Reverb::new(base_effect);
        reverb.set_dry(0.5);
        reverb.set_wet(0.5);
        reverb.set_decay_time(Duration::from_secs_f32(3.0));
        let reverb = context
            .state()
            .add_effect(rg3d::sound::effects::Effect::Reverb(reverb));

        let hrtf_sphere = rg3d::sound::hrtf::HrirSphere::from_file(
            "data/sounds/IRC_1040_C.bin",
            context::SAMPLE_RATE,
        )
        .unwrap();
        context
            .state()
            .set_renderer(rg3d::sound::renderer::Renderer::HrtfRenderer(
                rg3d::sound::renderer::hrtf::HrtfRenderer::new(hrtf_sphere),
            ));

        Self { context, reverb }
    }

    pub async fn handle_message(&mut self, resource_manager: ResourceManager, message: &Message) {
        let mut state = self.context.state();

        match message {
            Message::PlaySound {
                path,
                position,
                gain,
                rolloff_factor,
                radius,
            } => {
                let shot_buffer = resource_manager
                    .request_sound_buffer(path, false)
                    .await
                    .unwrap();
                let shot_sound = SpatialSourceBuilder::new(
                    GenericSourceBuilder::new()
                        .with_buffer(shot_buffer.into())
                        .with_status(Status::Playing)
                        .with_play_once(true)
                        .with_gain(*gain)
                        .build()
                        .unwrap(),
                )
                .with_position(*position)
                .with_radius(*radius)
                .with_rolloff_factor(*rolloff_factor)
                .build_source();
                let source = state.add_source(shot_sound);
                state
                    .effect_mut(self.reverb)
                    .add_input(EffectInput::direct(source));
            }
            _ => {}
        }
    }
}

impl Visit for SoundManager {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.context.visit("Context", visitor)?;
        self.reverb.visit("Reverb", visitor)?;

        visitor.leave_region()
    }
}

pub struct Level {
    map_root: Handle<Node>,
    pub scene: Handle<Scene>,
    player: Handle<Actor>,
    projectiles: ProjectileContainer,
    pub actors: ActorContainer,
    weapons: WeaponContainer,
    jump_pads: JumpPadContainer,
    items: ItemContainer,
    spawn_points: Vec<SpawnPoint>,
    sender: Option<Sender<Message>>,
    pub navmesh: Handle<Navmesh>,
    pub control_scheme: Option<Arc<RwLock<ControlScheme>>>,
    death_zones: Vec<DeathZone>,
    pub options: MatchOptions,
    time: f32,
    pub leader_board: LeaderBoard,
    respawn_list: Vec<RespawnEntry>,
    spectator_camera: Handle<Node>,
    target_spectator_position: Vector3<f32>,
    sound_manager: SoundManager,
    proximity_events_receiver: Option<crossbeam::channel::Receiver<IntersectionEvent>>,
    contact_events_receiver: Option<crossbeam::channel::Receiver<ContactEvent>>,
}

impl Default for Level {
    fn default() -> Self {
        Self {
            map_root: Default::default(),
            projectiles: ProjectileContainer::new(),
            actors: ActorContainer::new(),
            scene: Default::default(),
            player: Handle::NONE,
            weapons: WeaponContainer::new(),
            jump_pads: JumpPadContainer::new(),
            items: ItemContainer::new(),
            spawn_points: Default::default(),
            sender: None,
            navmesh: Default::default(),
            control_scheme: None,
            death_zones: Default::default(),
            options: Default::default(),
            time: 0.0,
            leader_board: Default::default(),
            respawn_list: Default::default(),
            spectator_camera: Default::default(),
            target_spectator_position: Default::default(),
            sound_manager: Default::default(),
            proximity_events_receiver: None,
            contact_events_receiver: None,
        }
    }
}

impl Visit for Level {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.scene.visit("Scene", visitor)?;
        self.map_root.visit("MapRoot", visitor)?;
        self.player.visit("Player", visitor)?;
        self.actors.visit("Actors", visitor)?;
        self.projectiles.visit("Projectiles", visitor)?;
        self.weapons.visit("Weapons", visitor)?;
        self.jump_pads.visit("JumpPads", visitor)?;
        self.spawn_points.visit("SpawnPoints", visitor)?;
        self.death_zones.visit("DeathZones", visitor)?;
        self.options.visit("Options", visitor)?;
        self.time.visit("Time", visitor)?;
        self.leader_board.visit("LeaderBoard", visitor)?;
        self.respawn_list.visit("RespawnList", visitor)?;
        self.spectator_camera.visit("SpectatorCamera", visitor)?;
        self.target_spectator_position
            .visit("TargetSpectatorPosition", visitor)?;
        self.sound_manager.visit("SoundManager", visitor)?;
        self.items.visit("Items", visitor)?;
        self.navmesh.visit("Navmesh", visitor)?;

        visitor.leave_region()
    }
}

pub struct DeathZone {
    bounds: AxisAlignedBoundingBox,
}

impl Visit for DeathZone {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.bounds.visit("Bounds", visitor)?;

        visitor.leave_region()
    }
}

impl Default for DeathZone {
    fn default() -> Self {
        Self {
            bounds: Default::default(),
        }
    }
}

pub struct UpdateContext<'a> {
    pub time: GameTime,
    pub scene: &'a mut Scene,
    pub items: &'a ItemContainer,
    pub jump_pads: &'a JumpPadContainer,
    pub navmesh: Handle<Navmesh>,
    pub weapons: &'a WeaponContainer,
}

struct PlayerRespawnEntry {
    time_left: f32,
}

impl Default for PlayerRespawnEntry {
    fn default() -> Self {
        Self { time_left: 0.0 }
    }
}

impl Visit for PlayerRespawnEntry {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.time_left.visit("TimeLeft", visitor)?;

        visitor.leave_region()
    }
}

struct BotRespawnEntry {
    name: String,
    kind: BotKind,
    time_left: f32,
}

impl Default for BotRespawnEntry {
    fn default() -> Self {
        Self {
            name: "".to_string(),
            kind: BotKind::Mutant,
            time_left: 0.0,
        }
    }
}

impl Visit for BotRespawnEntry {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.name.visit("Name", visitor)?;
        self.time_left.visit("TimeLeft", visitor)?;

        let mut kind_id = self.kind.id();
        kind_id.visit("Kind", visitor)?;
        self.kind = BotKind::from_id(kind_id)?;

        visitor.leave_region()
    }
}

enum RespawnEntry {
    Bot(BotRespawnEntry),
    Player(PlayerRespawnEntry),
}

impl Default for RespawnEntry {
    fn default() -> Self {
        RespawnEntry::Player(PlayerRespawnEntry::default())
    }
}

impl RespawnEntry {
    fn id(&self) -> u32 {
        match self {
            RespawnEntry::Bot { .. } => 0,
            RespawnEntry::Player { .. } => 1,
        }
    }

    fn from_id(id: u32) -> Result<Self, String> {
        match id {
            0 => Ok(RespawnEntry::Bot(Default::default())),
            1 => Ok(RespawnEntry::Player(Default::default())),
            _ => Err(format!("Invalid RespawnEntry type {}", id)),
        }
    }
}

impl Visit for RespawnEntry {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        let mut id = self.id();
        id.visit("Id", visitor)?;
        if visitor.is_reading() {
            *self = Self::from_id(id)?;
        }

        match self {
            RespawnEntry::Bot(v) => v.visit("Data", visitor)?,
            RespawnEntry::Player(v) => v.visit("Data", visitor)?,
        }

        visitor.leave_region()
    }
}

fn build_navmesh(scene: &mut Scene) -> Handle<Navmesh> {
    let navmesh_handle = scene.graph.find_by_name(scene.graph.get_root(), "Navmesh");
    if navmesh_handle.is_some() {
        let navmesh_node = &mut scene.graph[navmesh_handle];
        navmesh_node.set_visibility(false);

        scene
            .navmeshes
            .add(Navmesh::from_mesh(navmesh_node.as_mesh()))
    } else {
        Log::writeln(
            MessageKind::Warning,
            "Unable to find Navmesh node to build navmesh!".to_owned(),
        );
        Handle::NONE
    }
}

#[derive(Default)]
pub struct AnalysisResult {
    jump_pads: JumpPadContainer,
    items: ItemContainer,
    death_zones: Vec<DeathZone>,
    spawn_points: Vec<SpawnPoint>,
}

pub async fn analyze(
    scene: &mut Scene,
    resource_manager: ResourceManager,
    sender: Sender<Message>,
) -> AnalysisResult {
    let mut result = AnalysisResult::default();

    let mut items = Vec::new();
    let mut spawn_points = Vec::new();
    let mut death_zones = Vec::new();
    for (handle, node) in scene.graph.pair_iter() {
        let position = node.global_position();
        let name = node.name();
        if name.starts_with("JumpPad") {
            let begin = scene
                .graph
                .find_by_name_from_root(format!("{}_Begin", name).as_str());
            let end = scene
                .graph
                .find_by_name_from_root(format!("{}_End", name).as_str());
            if begin.is_some() && end.is_some() {
                let begin = scene.graph[begin].global_position();
                let end = scene.graph[end].global_position();
                let d = end - begin;
                let len = d.norm();
                let force = d.try_normalize(std::f32::EPSILON);
                let force = force.unwrap_or(Vector3::y()).scale(len * 3.0);
                let shape = scene.physics.mesh_to_trimesh(handle, &scene.graph);
                scene.physics_binder.bind(handle, shape);
                result.jump_pads.add(JumpPad::new(shape, force));
            };
        } else if name.starts_with("Medkit") {
            items.push((ItemKind::Medkit, position));
        } else if name.starts_with("Ammo_Ak47") {
            items.push((ItemKind::Ak47Ammo, position));
        } else if name.starts_with("Ammo_M4") {
            items.push((ItemKind::M4Ammo, position));
        } else if name.starts_with("Ammo_Plasma") {
            items.push((ItemKind::Plasma, position));
        } else if name.starts_with("SpawnPoint") {
            spawn_points.push(node.global_position())
        } else if name.starts_with("DeathZone") {
            if let Node::Mesh(_) = node {
                death_zones.push(handle);
            }
        }
    }

    for (kind, position) in items {
        result.items.add(
            Item::new(
                kind,
                position,
                scene,
                resource_manager.clone(),
                sender.clone(),
            )
            .await,
        );
    }
    for handle in death_zones {
        let node = &mut scene.graph[handle];
        node.set_visibility(false);
        result.death_zones.push(DeathZone {
            bounds: node.as_mesh().world_bounding_box(),
        });
    }
    result.spawn_points = spawn_points
        .into_iter()
        .map(|p| SpawnPoint { position: p })
        .collect();

    result
}

async fn spawn_player(
    spawn_points: &[SpawnPoint],
    actors: &mut ActorContainer,
    weapons: &mut WeaponContainer,
    sender: Sender<Message>,
    resource_manager: ResourceManager,
    control_scheme: Arc<RwLock<ControlScheme>>,
    scene: &mut Scene,
) -> Handle<Actor> {
    let index = find_suitable_spawn_point(spawn_points, actors, scene);
    let spawn_position = spawn_points.get(index).map_or(Vector3::default(), |pt| {
        pt.position + Vector3::new(0.0, 1.5, 0.0)
    });
    let mut player = Player::new(scene, sender.clone());
    player.set_control_scheme(control_scheme);
    let player = actors.add(Actor::Player(player));
    actors
        .get_mut(player)
        .set_position(&mut scene.physics, spawn_position);

    let weapons_to_give = [
        WeaponKind::M4,
        WeaponKind::Ak47,
        WeaponKind::PlasmaRifle,
        WeaponKind::RocketLauncher,
    ];
    for (i, &weapon) in weapons_to_give.iter().enumerate() {
        give_new_weapon(
            weapon,
            player,
            sender.clone(),
            resource_manager.clone(),
            i == weapons_to_give.len() - 1,
            weapons,
            actors,
            scene,
        )
        .await;
    }

    player
}

async fn give_new_weapon(
    kind: WeaponKind,
    actor: Handle<Actor>,
    sender: Sender<Message>,
    resource_manager: ResourceManager,
    visible: bool,
    weapons: &mut WeaponContainer,
    actors: &mut ActorContainer,
    scene: &mut Scene,
) {
    if actors.contains(actor) {
        let mut weapon = Weapon::new(kind, resource_manager, scene, sender.clone()).await;
        weapon.set_owner(actor);
        let weapon_model = weapon.get_model();
        scene.graph[weapon_model].set_visibility(visible);
        let actor = actors.get_mut(actor);
        let weapon_handle = weapons.add(weapon);
        actor.add_weapon(weapon_handle);
        scene.graph.link_nodes(weapon_model, actor.weapon_pivot());

        sender
            .send(Message::AddNotification {
                text: format!("Actor picked up weapon {:?}", kind),
            })
            .unwrap();
    }
}

fn find_suitable_spawn_point(
    spawn_points: &[SpawnPoint],
    actors: &ActorContainer,
    scene: &Scene,
) -> usize {
    // Find spawn point with least amount of enemies nearby.
    let mut index = rand::thread_rng().gen_range(0..spawn_points.len());
    let mut max_distance = -std::f32::MAX;
    for (i, pt) in spawn_points.iter().enumerate() {
        let mut sum_distance = 0.0;
        for actor in actors.iter() {
            let position = actor.position(&scene.physics);
            sum_distance += pt.position.metric_distance(&position);
        }
        if sum_distance > max_distance {
            max_distance = sum_distance;
            index = i;
        }
    }
    index
}

async fn spawn_bot(
    kind: BotKind,
    name: Option<String>,
    spawn_points: &[SpawnPoint],
    actors: &mut ActorContainer,
    weapons: &mut WeaponContainer,
    resource_manager: ResourceManager,
    sender: Sender<Message>,
    leader_board: &mut LeaderBoard,
    scene: &mut Scene,
) -> Handle<Actor> {
    let index = find_suitable_spawn_point(spawn_points, actors, scene);
    let spawn_position = spawn_points
        .get(index)
        .map_or(Vector3::default(), |pt| pt.position);

    let bot = add_bot(
        kind,
        spawn_position,
        name,
        actors,
        weapons,
        resource_manager,
        sender,
        leader_board,
        scene,
    )
    .await;

    bot
}

async fn add_bot(
    kind: BotKind,
    position: Vector3<f32>,
    name: Option<String>,
    actors: &mut ActorContainer,
    weapons: &mut WeaponContainer,
    resource_manager: ResourceManager,
    sender: Sender<Message>,
    leader_board: &mut LeaderBoard,
    scene: &mut Scene,
) -> Handle<Actor> {
    let bot = Bot::new(
        kind,
        resource_manager.clone(),
        scene,
        position,
        sender.clone(),
    )
    .await;
    let name = name.unwrap_or_else(|| format!("Bot {:?} {}", kind, actors.count()));
    leader_board.get_or_add_actor(&name);
    let bot = actors.add(Actor::Bot(bot));
    give_new_weapon(
        WeaponKind::Ak47,
        bot,
        sender.clone(),
        resource_manager,
        true,
        weapons,
        actors,
        scene,
    )
    .await;
    bot
}

impl Level {
    pub async fn new(
        resource_manager: ResourceManager,
        control_scheme: Arc<RwLock<ControlScheme>>,
        sender: Sender<Message>,
        options: MatchOptions,
    ) -> (Level, Scene) {
        let mut scene = Scene::new();

        scene.ambient_lighting_color = Color::opaque(60, 60, 60);

        let sound_manager = SoundManager::new(scene.sound_context.clone());

        let (proximity_events_sender, proximity_events_receiver) = crossbeam::channel::unbounded();
        let (contact_events_sender, contact_events_receiver) = crossbeam::channel::unbounded();

        scene.physics.event_handler = Box::new(ChannelEventCollector::new(
            proximity_events_sender.clone(),
            contact_events_sender.clone(),
        ));

        // Spectator camera is used when there is no player on level.
        // This includes situation when player is dead - all dead actors are removed
        // from level.
        let spectator_camera = CameraBuilder::new(BaseBuilder::new())
            .enabled(false)
            .build(&mut scene.graph);

        let map_model = resource_manager
            .request_model(
                Path::new("data/models/dm6.fbx"),
                MaterialSearchOptions::MaterialsDirectory(PathBuf::from("data/textures")),
            )
            .await
            .unwrap();

        // Instantiate map
        let map_root = map_model.instantiate_geometry(&mut scene);

        // Create collision geometry
        let polygon_handle = scene.graph.find_by_name(map_root, "Polygon");
        if polygon_handle.is_some() {
            scene.physics.mesh_to_trimesh(polygon_handle, &scene.graph);
        } else {
            Log::writeln(
                MessageKind::Warning,
                "Unable to find Polygon node to build collision shape for level!".to_owned(),
            );
        }

        let AnalysisResult {
            jump_pads,
            items,
            death_zones,
            spawn_points,
        } = analyze(&mut scene, resource_manager.clone(), sender.clone()).await;
        let mut actors = ActorContainer::new();
        let mut weapons = WeaponContainer::new();
        let mut leader_board = LeaderBoard::default();

        for &kind in &[BotKind::Maw, BotKind::Mutant, BotKind::Parasite] {
            spawn_bot(
                kind,
                Some(kind.description().to_owned()),
                &spawn_points,
                &mut actors,
                &mut weapons,
                resource_manager.clone(),
                sender.clone(),
                &mut leader_board,
                &mut scene,
            )
            .await;
        }

        let level = Level {
            player: spawn_player(
                &spawn_points,
                &mut actors,
                &mut weapons,
                sender.clone(),
                resource_manager.clone(),
                control_scheme.clone(),
                &mut scene,
            )
            .await,
            map_root,
            options,
            spectator_camera,
            actors,
            weapons,
            jump_pads,
            items,
            death_zones,
            spawn_points,
            leader_board,
            navmesh: build_navmesh(&mut scene),
            scene: Handle::NONE, // Filled when scene will be moved to engine.
            sender: Some(sender),
            control_scheme: Some(control_scheme),
            time: 0.0,
            respawn_list: Default::default(),
            contact_events_receiver: Some(contact_events_receiver),
            proximity_events_receiver: Some(proximity_events_receiver),
            projectiles: ProjectileContainer::new(),
            target_spectator_position: Default::default(),
            sound_manager,
        };

        (level, scene)
    }

    pub fn destroy(&mut self, engine: &mut Engine) {
        engine.scenes.remove(self.scene);
    }

    async fn give_new_weapon(
        &mut self,
        engine: &mut Engine,
        actor: Handle<Actor>,
        kind: WeaponKind,
    ) {
        give_new_weapon(
            kind,
            actor,
            self.sender.clone().unwrap(),
            engine.resource_manager.clone(),
            true,
            &mut self.weapons,
            &mut self.actors,
            &mut engine.scenes[self.scene],
        )
        .await;
    }

    async fn spawn_player(&mut self, engine: &mut Engine) -> Handle<Actor> {
        let scene = &mut engine.scenes[self.scene];

        let player = spawn_player(
            &self.spawn_points,
            &mut self.actors,
            &mut self.weapons,
            self.sender.clone().unwrap(),
            engine.resource_manager.clone(),
            self.control_scheme.clone().unwrap(),
            scene,
        )
        .await;

        if let Node::Camera(spectator_camera) = &mut scene.graph[self.spectator_camera] {
            spectator_camera.set_enabled(false);
        }

        player
    }

    pub fn get_player(&self) -> Handle<Actor> {
        self.player
    }

    pub fn process_input_event(&mut self, event: &Event<()>) -> bool {
        if self.player.is_some() {
            if let Actor::Player(player) = self.actors.get_mut(self.player) {
                return player.process_input_event(event);
            }
        }
        false
    }

    pub fn actors(&self) -> &ActorContainer {
        &self.actors
    }

    pub fn actors_mut(&mut self) -> &mut ActorContainer {
        &mut self.actors
    }

    pub fn weapons(&self) -> &WeaponContainer {
        &self.weapons
    }

    fn pick(&self, engine: &mut Engine, from: Vector3<f32>, to: Vector3<f32>) -> Vector3<f32> {
        let scene = &mut engine.scenes[self.scene];
        let ray = Ray::from_two_points(from, to);
        let options = RayCastOptions {
            ray_origin: Point3::from(ray.origin),
            ray_direction: ray.dir,
            max_len: std::f32::MAX,
            groups: InteractionGroups::all(),
            sort_results: true,
        };
        let mut query_buffer = Vec::default();
        scene.physics.cast_ray(options, &mut query_buffer);
        if let Some(pt) = query_buffer.first() {
            pt.position.coords
        } else {
            from
        }
    }

    fn remove_weapon(&mut self, engine: &mut Engine, weapon: Handle<Weapon>) {
        for projectile in self.projectiles.iter_mut() {
            if projectile.owner == weapon {
                // Reset owner because handle to weapon will be invalid after weapon freed.
                projectile.owner = Handle::NONE;
            }
        }
        self.weapons[weapon].clean_up(&mut engine.scenes[self.scene]);
        self.weapons.free(weapon);
    }

    async fn add_bot(
        &mut self,
        engine: &mut Engine,
        kind: BotKind,
        position: Vector3<f32>,
        name: Option<String>,
    ) -> Handle<Actor> {
        add_bot(
            kind,
            position,
            name,
            &mut self.actors,
            &mut self.weapons,
            engine.resource_manager.clone(),
            self.sender.clone().unwrap(),
            &mut self.leader_board,
            &mut engine.scenes[self.scene],
        )
        .await
    }

    async fn remove_actor(&mut self, engine: &mut Engine, actor: Handle<Actor>) {
        if self.actors.contains(actor) {
            let scene = &mut engine.scenes[self.scene];
            let character = self.actors.get(actor);

            // Make sure to remove weapons and drop appropriate items (items will be temporary).
            let drop_position = character.position(&scene.physics);
            let weapons = character
                .weapons()
                .iter()
                .copied()
                .collect::<Vec<Handle<Weapon>>>();
            for weapon in weapons {
                let item_kind = match self.weapons[weapon].get_kind() {
                    WeaponKind::M4 => ItemKind::M4,
                    WeaponKind::Ak47 => ItemKind::Ak47,
                    WeaponKind::PlasmaRifle => ItemKind::PlasmaGun,
                    WeaponKind::RocketLauncher => ItemKind::RocketLauncher,
                };
                self.spawn_item(engine, item_kind, drop_position, true, Some(20.0))
                    .await;
                self.remove_weapon(engine, weapon);
            }

            let scene = &mut engine.scenes[self.scene];
            self.actors.get_mut(actor).clean_up(scene);
            self.actors.free(actor);

            if self.player == actor {
                self.player = Handle::NONE;
            }
        }
    }

    async fn give_item(&mut self, engine: &mut Engine, actor: Handle<Actor>, kind: ItemKind) {
        if self.actors.contains(actor) {
            let character = self.actors.get_mut(actor);
            match kind {
                ItemKind::Medkit => character.heal(20.0),
                ItemKind::Ak47 | ItemKind::PlasmaGun | ItemKind::M4 | ItemKind::RocketLauncher => {
                    let weapon_kind = match kind {
                        ItemKind::Ak47 => WeaponKind::Ak47,
                        ItemKind::PlasmaGun => WeaponKind::PlasmaRifle,
                        ItemKind::M4 => WeaponKind::M4,
                        ItemKind::RocketLauncher => WeaponKind::RocketLauncher,
                        _ => unreachable!(),
                    };

                    let mut found = false;
                    for weapon_handle in character.weapons() {
                        let weapon = &mut self.weapons[*weapon_handle];
                        // If actor already has weapon of given kind, then just add ammo to it.
                        if weapon.get_kind() == weapon_kind {
                            found = true;
                            weapon.add_ammo(200);
                            break;
                        }
                    }
                    // Finally if actor does not have such weapon, give new one to him.
                    if !found {
                        self.give_new_weapon(engine, actor, weapon_kind).await;
                    }
                }
                ItemKind::Plasma | ItemKind::Ak47Ammo | ItemKind::M4Ammo => {
                    for weapon in character.weapons() {
                        let weapon = &mut self.weapons[*weapon];
                        let (weapon_kind, ammo) = match kind {
                            ItemKind::Plasma => (WeaponKind::PlasmaRifle, 200),
                            ItemKind::Ak47Ammo => (WeaponKind::Ak47, 200),
                            ItemKind::M4Ammo => (WeaponKind::M4, 200),
                            _ => continue,
                        };
                        if weapon.get_kind() == weapon_kind {
                            weapon.add_ammo(ammo);
                            break;
                        }
                    }
                }
            }
        }
    }

    async fn pickup_item(&mut self, engine: &mut Engine, actor: Handle<Actor>, item: Handle<Item>) {
        if self.actors.contains(actor) && self.items.contains(item) {
            let item = self.items.get_mut(item);

            self.sender
                .as_ref()
                .unwrap()
                .send(Message::AddNotification {
                    text: format!("Actor picked up item {:?}", item.get_kind()),
                })
                .unwrap();

            let scene = &mut engine.scenes[self.scene];
            let position = item.position(&scene.graph);
            item.pick_up();
            let kind = item.get_kind();
            self.sender
                .as_ref()
                .unwrap()
                .send(Message::PlaySound {
                    path: PathBuf::from("data/sounds/item_pickup.ogg"),
                    position,
                    gain: 1.0,
                    rolloff_factor: 3.0,
                    radius: 2.0,
                })
                .unwrap();
            self.give_item(engine, actor, kind).await;
        }
    }

    async fn create_projectile(
        &mut self,
        engine: &mut Engine,
        kind: ProjectileKind,
        position: Vector3<f32>,
        direction: Vector3<f32>,
        initial_velocity: Vector3<f32>,
        owner: Handle<Weapon>,
        basis: Matrix3<f32>,
    ) {
        let scene = &mut engine.scenes[self.scene];
        let projectile = Projectile::new(
            kind,
            engine.resource_manager.clone(),
            scene,
            direction,
            position,
            owner,
            initial_velocity,
            self.sender.as_ref().unwrap().clone(),
            basis,
        )
        .await;
        self.projectiles.add(projectile);
    }

    async fn shoot_weapon(
        &mut self,
        engine: &mut Engine,
        weapon_handle: Handle<Weapon>,
        initial_velocity: Vector3<f32>,
        time: GameTime,
        direction: Option<Vector3<f32>>,
    ) {
        if self.weapons.contains(weapon_handle) {
            let scene = &mut engine.scenes[self.scene];
            let weapon = &mut self.weapons[weapon_handle];
            if weapon.try_shoot(scene, time) {
                let kind = weapon.definition.projectile;
                let position = weapon.get_shot_position(&scene.graph);
                let direction = direction
                    .unwrap_or_else(|| weapon.get_shot_direction(&scene.graph))
                    .try_normalize(std::f32::EPSILON)
                    .unwrap_or_else(|| Vector3::z());
                let basis = weapon.world_basis(&scene.graph);
                self.create_projectile(
                    engine,
                    kind,
                    position,
                    direction,
                    initial_velocity,
                    weapon_handle,
                    basis,
                )
                .await;
            }
        }
    }

    fn show_weapon(&mut self, engine: &mut Engine, weapon_handle: Handle<Weapon>, state: bool) {
        self.weapons[weapon_handle].set_visibility(state, &mut engine.scenes[self.scene].graph)
    }

    async fn spawn_bot(
        &mut self,
        engine: &mut Engine,
        kind: BotKind,
        name: Option<String>,
    ) -> Handle<Actor> {
        let bot = spawn_bot(
            kind,
            name,
            &self.spawn_points,
            &mut self.actors,
            &mut self.weapons,
            engine.resource_manager.clone(),
            self.sender.clone().unwrap(),
            &mut self.leader_board,
            &mut engine.scenes[self.scene],
        )
        .await;

        self.sender
            .as_ref()
            .unwrap()
            .send(Message::AddNotification {
                text: format!("Bot {} spawned!", self.actors.get(bot).name),
            })
            .unwrap();

        bot
    }

    fn damage_actor(
        &mut self,
        engine: &Engine,
        actor: Handle<Actor>,
        who: Handle<Actor>,
        amount: f32,
        time: GameTime,
    ) {
        if self.actors.contains(actor)
            && (who.is_none() || who.is_some() && self.actors.contains(who))
        {
            let mut who_name = Default::default();
            let message = if who.is_some() {
                who_name = self.actors.get(who).name.clone();
                format!(
                    "{} dealt {} damage to {}!",
                    who_name,
                    amount,
                    self.actors.get(actor).name
                )
            } else {
                format!("{} took {} damage!", self.actors.get(actor).name, amount)
            };

            self.sender
                .as_ref()
                .unwrap()
                .send(Message::AddNotification { text: message })
                .unwrap();

            let who_position = if who.is_some() {
                let scene = &engine.scenes[self.scene];
                Some(self.actors.get(who).position(&scene.physics))
            } else {
                None
            };
            let actor = self.actors.get_mut(actor);
            if let Actor::Bot(bot) = actor {
                if let Some(who_position) = who_position {
                    bot.set_point_of_interest(who_position, time);
                }
            }
            let was_dead = actor.is_dead();
            actor.damage(amount);
            if !was_dead && actor.is_dead() && who.is_some() {
                self.leader_board.add_frag(who_name)
            }
        }
    }

    async fn spawn_item(
        &mut self,
        engine: &mut Engine,
        kind: ItemKind,
        position: Vector3<f32>,
        adjust_height: bool,
        lifetime: Option<f32>,
    ) {
        let position = if adjust_height {
            self.pick(engine, position, position - Vector3::new(0.0, 1000.0, 0.0))
        } else {
            position
        };
        let scene = &mut engine.scenes[self.scene];
        let mut item = Item::new(
            kind,
            position,
            scene,
            engine.resource_manager.clone(),
            self.sender.as_ref().unwrap().clone(),
        )
        .await;
        item.set_lifetime(lifetime);
        self.items.add(item);
    }

    pub fn time(&self) -> f32 {
        self.time
    }

    fn update_respawn(&mut self, time: GameTime) {
        // Respawn is done in deferred manner: we just gather all info needed
        // for respawn, wait some time and then re-create actor. Actor is spawned
        // by sending a message: this is needed because there are some other
        // systems that catches such messages and updates their own state.
        for respawn_entry in self.respawn_list.iter_mut() {
            match respawn_entry {
                RespawnEntry::Bot(v) => {
                    v.time_left -= time.delta;
                    if v.time_left <= 0.0 {
                        self.sender
                            .as_mut()
                            .unwrap()
                            .send(Message::SpawnBot {
                                kind: v.kind,
                                name: v.name.clone(),
                            })
                            .unwrap();
                    }
                }
                RespawnEntry::Player(v) => {
                    v.time_left -= time.delta;
                    if v.time_left <= 0.0 {
                        self.sender
                            .as_mut()
                            .unwrap()
                            .send(Message::SpawnPlayer)
                            .unwrap();
                    }
                }
            }
        }

        self.respawn_list.retain(|entry| match entry {
            RespawnEntry::Bot(v) => v.time_left >= 0.0,
            RespawnEntry::Player(v) => v.time_left >= 0.0,
        });
    }

    fn update_spectator_camera(&mut self, scene: &mut Scene) {
        if let Node::Camera(spectator_camera) = &mut scene.graph[self.spectator_camera] {
            let mut position = spectator_camera.global_position();
            position.follow(&self.target_spectator_position, 0.1);
            spectator_camera
                .local_transform_mut()
                .set_position(position);
        }
    }

    fn update_death_zones(&mut self, scene: &Scene) {
        for (handle, actor) in self.actors.pair_iter_mut() {
            for death_zone in self.death_zones.iter() {
                if death_zone
                    .bounds
                    .is_contains_point(actor.position(&scene.physics))
                {
                    self.sender
                        .as_ref()
                        .unwrap()
                        .send(Message::RespawnActor { actor: handle })
                        .unwrap();
                }
            }
        }
    }

    fn update_game_ending(&self) {
        if self.leader_board.is_match_over(&self.options) {
            self.sender
                .as_ref()
                .unwrap()
                .send(Message::EndMatch)
                .unwrap();
        }
    }

    pub fn update(&mut self, engine: &mut Engine, time: GameTime) {
        self.time += time.delta;
        self.update_respawn(time);
        let scene = &mut engine.scenes[self.scene];
        while let Ok(proximity_event) = self.proximity_events_receiver.as_ref().unwrap().try_recv()
        {
            for proj in self.projectiles.iter_mut() {
                proj.handle_proximity(&proximity_event, scene, &self.actors, &self.weapons);
            }
        }
        self.update_spectator_camera(scene);
        self.update_death_zones(scene);
        self.weapons.update(scene, &self.actors);
        self.projectiles
            .update(scene, &self.actors, &self.weapons, time);
        self.items.update(scene, time);
        let mut ctx = UpdateContext {
            time,
            scene,
            items: &self.items,
            jump_pads: &self.jump_pads,
            navmesh: self.navmesh,
            weapons: &self.weapons,
        };
        self.actors.update(&mut ctx);
        while let Ok(contact_event) = self.contact_events_receiver.as_ref().unwrap().try_recv() {
            self.actors.handle_event(&contact_event, &mut ctx);
        }
        self.update_game_ending();
    }

    pub async fn respawn_actor(&mut self, engine: &mut Engine, actor: Handle<Actor>) {
        if self.actors.contains(actor) {
            let name = self.actors.get(actor).name.clone();

            self.leader_board.add_death(&name);

            let entry = match self.actors.get(actor) {
                Actor::Bot(bot) => RespawnEntry::Bot(BotRespawnEntry {
                    name,
                    kind: bot.definition.kind,
                    time_left: RESPAWN_TIME,
                }),
                Actor::Player(player) => {
                    // Turn on spectator camera and prepare its target position. Spectator
                    // camera will be used to render world until player is despawned.
                    let scene = &mut engine.scenes[self.scene];
                    let position = scene.graph[player.camera()].global_position();
                    if let Node::Camera(spectator_camera) = &mut scene.graph[self.spectator_camera]
                    {
                        spectator_camera
                            .set_enabled(true)
                            .local_transform_mut()
                            .set_position(position);
                    }
                    // Use ray casting to get target position for spectator camera, it is used to
                    // create "dropping head" effect.
                    let ray =
                        Ray::from_two_points(position, position - Vector3::new(0.0, 1000.0, 0.0));
                    let options = RayCastOptions {
                        ray_origin: Point3::from(ray.origin),
                        ray_direction: ray.dir,
                        max_len: std::f32::MAX,
                        groups: InteractionGroups::all(),
                        sort_results: true,
                    };

                    let mut query_buffer = Vec::default();
                    scene.physics.cast_ray(options, &mut query_buffer);
                    if let Some(hit) = query_buffer.first() {
                        self.target_spectator_position = hit.position.coords;
                        // Prevent see-thru-floor
                        self.target_spectator_position.y += 0.1;
                    } else {
                        self.target_spectator_position = position;
                    }

                    RespawnEntry::Player(PlayerRespawnEntry {
                        time_left: RESPAWN_TIME,
                    })
                }
            };

            self.remove_actor(engine, actor).await;

            self.respawn_list.push(entry);
        }
    }

    pub async fn handle_message(&mut self, engine: &mut Engine, message: &Message, time: GameTime) {
        self.sound_manager
            .handle_message(engine.resource_manager.clone(), &message)
            .await;

        match message {
            &Message::GiveNewWeapon { actor, kind } => {
                self.give_new_weapon(engine, actor, kind).await;
            }
            Message::AddBot {
                kind,
                position,
                name,
            } => {
                self.add_bot(engine, *kind, *position, name.clone()).await;
            }
            &Message::RemoveActor { actor } => self.remove_actor(engine, actor).await,
            &Message::GiveItem { actor, kind } => {
                self.give_item(engine, actor, kind).await;
            }
            &Message::PickUpItem { actor, item } => {
                self.pickup_item(engine, actor, item).await;
            }
            &Message::ShootWeapon {
                weapon,
                initial_velocity,
                direction,
            } => {
                self.shoot_weapon(engine, weapon, initial_velocity, time, direction)
                    .await
            }
            &Message::CreateProjectile {
                kind,
                position,
                direction,
                initial_velocity,
                owner,
                basis,
            } => {
                self.create_projectile(
                    engine,
                    kind,
                    position,
                    direction,
                    initial_velocity,
                    owner,
                    basis,
                )
                .await
            }
            &Message::ShowWeapon { weapon, state } => self.show_weapon(engine, weapon, state),
            Message::SpawnBot { kind, name } => {
                self.spawn_bot(engine, *kind, Some(name.clone())).await;
            }
            &Message::DamageActor { actor, who, amount } => {
                self.damage_actor(engine, actor, who, amount, time);
            }
            &Message::CreateEffect { kind, position } => {
                effects::create(
                    kind,
                    &mut engine.scenes[self.scene].graph,
                    engine.resource_manager.clone(),
                    position,
                );
            }
            Message::SpawnPlayer => {
                self.player = self.spawn_player(engine).await;
            }
            &Message::SpawnItem {
                kind,
                position,
                adjust_height,
                lifetime,
            } => {
                self.spawn_item(engine, kind, position, adjust_height, lifetime)
                    .await
            }
            &Message::RespawnActor { actor } => self.respawn_actor(engine, actor).await,
            _ => (),
        }
    }

    pub fn set_message_sender(&mut self, sender: Sender<Message>, engine: &mut Engine) {
        self.sender = Some(sender.clone());

        // Attach new sender to all event sources.
        for actor in self.actors.iter_mut() {
            actor.sender = Some(sender.clone());
        }
        for weapon in self.weapons.iter_mut() {
            weapon.sender = Some(sender.clone());
        }
        for projectile in self.projectiles.iter_mut() {
            projectile.sender = Some(sender.clone());
        }
        for item in self.items.iter_mut() {
            item.sender = Some(sender.clone());
        }

        let (proximity_events_sender, proximity_events_receiver) = crossbeam::channel::unbounded();
        let (contact_events_sender, contact_events_receiver) = crossbeam::channel::unbounded();

        self.proximity_events_receiver = Some(proximity_events_receiver);
        self.contact_events_receiver = Some(contact_events_receiver);

        engine.scenes[self.scene].physics.event_handler = Box::new(ChannelEventCollector::new(
            proximity_events_sender.clone(),
            contact_events_sender.clone(),
        ));
    }

    pub fn debug_draw(&self, engine: &mut Engine) {
        let scene = &mut engine.scenes[self.scene];

        let drawing_context = &mut scene.drawing_context;

        drawing_context.clear_lines();

        scene.physics.draw(drawing_context);

        if self.navmesh.is_some() {
            let navmesh = &scene.navmeshes[self.navmesh];

            for pt in navmesh.vertices() {
                for neighbour in pt.neighbours() {
                    drawing_context.add_line(scene::debug::Line {
                        begin: pt.position(),
                        end: navmesh.vertices()[*neighbour as usize].position(),
                        color: Default::default(),
                    });
                }
            }

            for actor in self.actors.iter() {
                if let Actor::Bot(bot) = actor {
                    bot.debug_draw(drawing_context);
                }
            }
        }

        for death_zone in self.death_zones.iter() {
            drawing_context.draw_aabb(&death_zone.bounds, Color::opaque(0, 0, 200));
        }
    }
}

pub struct SpawnPoint {
    position: Vector3<f32>,
}

impl Default for SpawnPoint {
    fn default() -> Self {
        Self {
            position: Default::default(),
        }
    }
}

impl Visit for SpawnPoint {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.position.visit("Position", visitor)?;

        visitor.leave_region()
    }
}
