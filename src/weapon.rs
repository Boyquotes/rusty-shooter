use std::path::Path;
use rg3d::{
    resource::model::Model,
    utils::{
        pool::Handle,
        visitor::{
            Visit,
            VisitResult,
            Visitor,
            VisitError,
        },
    },
    scene::{
        node::Node,
        Scene,
    },
    engine::state::State,
    math::vec3::Vec3,
};

use crate::GameTime;

pub enum WeaponKind {
    Unknown,
    M4,
    Ak47,
}

pub struct Weapon {
    kind: WeaponKind,
    model: Handle<Node>,
    offset: Vec3,
    dest_offset: Vec3,
    last_shot_time: f64,
}

impl Default for Weapon {
    fn default() -> Self {
        Self {
            kind: WeaponKind::Unknown,
            model: Handle::none(),
            offset: Vec3::new(),
            dest_offset: Vec3::new(),
            last_shot_time: 0.0,
        }
    }
}

impl Visit for Weapon {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        let mut kind_id: u8 = if visitor.is_reading() {
            0
        } else {
            match self.kind {
                WeaponKind::Unknown => return Err(VisitError::User(String::from("unknown weapon kind on save???"))),
                WeaponKind::M4 => 0,
                WeaponKind::Ak47 => 1,
            }
        };

        kind_id.visit("KindId", visitor)?;

        if visitor.is_reading() {
            self.kind = match kind_id {
                0 => WeaponKind::M4,
                1 => WeaponKind::Ak47,
                _ => return Err(VisitError::User(format!("unknown weapon kind {}", kind_id)))
            }
        }

        self.model.visit("Model", visitor)?;
        self.offset.visit("Offset", visitor)?;
        self.dest_offset.visit("DestOffset", visitor)?;
        self.last_shot_time.visit("LastShotTime", visitor)?;

        visitor.leave_region()
    }
}

impl Weapon {
    pub fn new(kind: WeaponKind, state: &mut State, scene: &mut Scene) -> Weapon {
        let model_path = match kind {
            WeaponKind::Unknown => panic!("must not be here"),
            WeaponKind::Ak47 => Path::new("data/models/ak47.fbx"),
            WeaponKind::M4 => Path::new("data/models/m4.fbx"),
        };

        let mut weapon_model = Handle::none();
        let model_resource_handle = state.request_resource(model_path);
        if model_resource_handle.is_some() {
            weapon_model = Model::instantiate(model_resource_handle.unwrap(), scene).unwrap_or(Handle::none());
        }

        Weapon {
            kind,
            model: weapon_model,
            offset: Vec3::new(),
            dest_offset: Vec3::new(),
            last_shot_time: 0.0,
        }
    }

    #[inline]
    pub fn get_model(&self) -> Handle<Node> {
        self.model
    }

    pub fn update(&mut self, scene: &mut Scene) {
        self.offset.x += (self.dest_offset.x - self.offset.x) * 0.2;
        self.offset.y += (self.dest_offset.y - self.offset.y) * 0.2;
        self.offset.z += (self.dest_offset.z - self.offset.z) * 0.2;

        if let Some(node) = scene.get_node_mut(self.model) {
            node.set_local_position(self.offset);
        }
    }

    pub fn shoot(&mut self, time: &GameTime) {
        if time.elapsed - self.last_shot_time >= 0.1 {
            self.offset = Vec3::make(0.0, 0.0, -0.05);
            self.last_shot_time = time.elapsed;
        }
    }
}