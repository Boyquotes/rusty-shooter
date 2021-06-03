use crate::{message::Message, weapon::Weapon};
use rg3d::{
    core::{
        algebra::Vector3,
        pool::Handle,
        visitor::{Visit, VisitError, VisitResult, Visitor},
    },
    engine::RigidBodyHandle,
    scene::{node::Node, physics::Physics, Scene},
};
use std::sync::mpsc::Sender;

pub struct Character {
    pub name: String,
    pub pivot: Handle<Node>,
    pub body: RigidBodyHandle,
    pub health: f32,
    pub armor: f32,
    pub weapons: Vec<Handle<Weapon>>,
    pub current_weapon: u32,
    pub weapon_pivot: Handle<Node>,
    pub sender: Option<Sender<Message>>,
    pub team: Team,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum Team {
    None,
    Red,
    Blue,
}

impl Default for Team {
    fn default() -> Self {
        Team::None
    }
}

impl Visit for Team {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut id = match self {
            Team::None => 0,
            Team::Red => 1,
            Team::Blue => 2,
        };
        id.visit(name, visitor)?;
        if visitor.is_reading() {
            *self = match id {
                0 => Team::None,
                1 => Team::Red,
                2 => Team::Blue,
                _ => return Err(VisitError::User(format!("Invalid team id {}", id))),
            }
        }
        Ok(())
    }
}

impl Default for Character {
    fn default() -> Self {
        Self {
            name: Default::default(),
            pivot: Handle::NONE,
            body: Default::default(),
            health: 100.0,
            armor: 100.0,
            weapons: Vec::new(),
            current_weapon: 0,
            weapon_pivot: Handle::NONE,
            sender: None,
            team: Team::None,
        }
    }
}

impl Visit for Character {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.name.visit("Name", visitor)?;
        self.pivot.visit("Pivot", visitor)?;
        self.body.visit("Body", visitor)?;
        self.health.visit("Health", visitor)?;
        self.armor.visit("Armor", visitor)?;
        self.weapons.visit("Weapons", visitor)?;
        self.current_weapon.visit("CurrentWeapon", visitor)?;
        self.weapon_pivot.visit("WeaponPivot", visitor)?;
        self.team.visit("Team", visitor)?;

        visitor.leave_region()
    }
}

impl Character {
    pub fn get_body(&self) -> RigidBodyHandle {
        self.body
    }

    pub fn has_ground_contact(&self, physics: &Physics) -> bool {
        let body = physics.body(&self.body).unwrap();
        for contact in physics.narrow_phase.contacts_with(body.colliders()[0]) {
            for manifold in contact.manifolds.iter() {
                if manifold.local_n1.y > 0.7 {
                    return true;
                }
            }
        }
        false
    }

    pub fn set_team(&mut self, team: Team) {
        self.team = team;
    }

    pub fn team(&self) -> Team {
        self.team
    }

    pub fn get_health(&self) -> f32 {
        self.health
    }

    pub fn get_armor(&self) -> f32 {
        self.armor
    }

    pub fn set_position(&mut self, physics: &mut Physics, position: Vector3<f32>) {
        let body = physics.body_mut(&self.get_body()).unwrap();
        let mut body_position = *body.position();
        body_position.translation.vector = position;
        body.set_position(body_position, true);
    }

    pub fn position(&self, physics: &Physics) -> Vector3<f32> {
        physics
            .body(&self.get_body())
            .unwrap()
            .position()
            .translation
            .vector
    }

    pub fn damage(&mut self, amount: f32) {
        let amount = amount.abs();
        if self.armor > 0.0 {
            self.armor -= amount;
            if self.armor < 0.0 {
                self.health += self.armor;
            }
        } else {
            self.health -= amount;
        }
    }

    pub fn heal(&mut self, amount: f32) {
        self.health += amount.abs();

        if self.health > 150.0 {
            self.health = 150.0;
        }
    }

    pub fn is_dead(&self) -> bool {
        self.health <= 0.0
    }

    pub fn weapon_pivot(&self) -> Handle<Node> {
        self.weapon_pivot
    }

    pub fn weapons(&self) -> &[Handle<Weapon>] {
        &self.weapons
    }

    pub fn add_weapon(&mut self, weapon: Handle<Weapon>) {
        if let Some(sender) = self.sender.as_ref() {
            for other_weapon in self.weapons.iter() {
                sender
                    .send(Message::ShowWeapon {
                        weapon: *other_weapon,
                        state: false,
                    })
                    .unwrap();
            }
        }

        self.current_weapon = self.weapons.len() as u32;
        self.weapons.push(weapon);

        self.request_current_weapon_visible(true);
    }

    pub fn current_weapon(&self) -> Handle<Weapon> {
        if let Some(weapon) = self.weapons.get(self.current_weapon as usize) {
            *weapon
        } else {
            Handle::NONE
        }
    }

    fn request_current_weapon_visible(&self, state: bool) {
        if let Some(sender) = self.sender.as_ref() {
            if let Some(current_weapon) = self.weapons.get(self.current_weapon as usize) {
                sender
                    .send(Message::ShowWeapon {
                        weapon: *current_weapon,
                        state,
                    })
                    .unwrap()
            }
        }
    }

    pub fn next_weapon(&mut self) {
        if !self.weapons.is_empty() && (self.current_weapon as usize) < self.weapons.len() - 1 {
            self.request_current_weapon_visible(false);

            self.current_weapon += 1;

            self.request_current_weapon_visible(true);
        }
    }

    pub fn prev_weapon(&mut self) {
        if self.current_weapon > 0 {
            self.request_current_weapon_visible(false);

            self.current_weapon -= 1;

            self.request_current_weapon_visible(true);
        }
    }

    pub fn set_current_weapon(&mut self, i: usize) {
        if i < self.weapons.len() {
            self.request_current_weapon_visible(false);

            self.current_weapon = i as u32;

            self.request_current_weapon_visible(true);
        }
    }

    pub fn clean_up(&mut self, scene: &mut Scene) {
        scene.remove_node(self.pivot);
        scene.physics.remove_body(&self.body);
    }
}
