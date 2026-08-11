//! Projectile system for visual attack effects
//!
//! Manages short-lived visual projectiles that travel from attacker to defender.
//! Travel/lerp/lifetime mechanics come from `macroquad_toolkit::fx::ProjectileLayer`;
//! this module keeps the game-specific projectile types (textures, durations,
//! melee travel ratio) and the impact payload.

use crate::state::entities::EntityId;
use crate::state::tile_state::TilePos;
use macroquad::prelude::vec2;
use macroquad_toolkit::fx::{ProjectileLayer, TravelingProjectile};
use serde::{Deserialize, Serialize};

/// Type of projectile based on attack type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectileType {
    /// Melee slash - short range, stays near attacker
    Melee,
    /// Arrow - travels from attacker to defender
    Arrow,
    /// Magic orb - travels from attacker to defender with glow
    Magic,
}

impl ProjectileType {
    /// Get projectile type from attack_type string
    pub fn from_attack_type(attack_type: &str) -> Self {
        match attack_type {
            "ranged" => ProjectileType::Arrow,
            "magic" => ProjectileType::Magic,
            _ => ProjectileType::Melee,
        }
    }

    /// Get the texture key for this projectile type
    pub fn texture_key(&self) -> &'static str {
        match self {
            ProjectileType::Melee => "projectile_melee",
            ProjectileType::Arrow => "projectile_arrow",
            ProjectileType::Magic => "projectile_magic",
        }
    }

    /// Get the duration of this projectile in seconds
    pub fn duration(&self) -> f32 {
        match self {
            ProjectileType::Melee => 0.15, // Very quick slash
            ProjectileType::Arrow => 0.3,  // Fast arrow
            ProjectileType::Magic => 0.4,  // Slower magic orb
        }
    }

    /// Fraction of the attacker->defender distance the projectile travels
    /// (melee slashes stay near the attacker)
    fn travel_ratio(&self) -> f32 {
        match self {
            ProjectileType::Melee => 0.3,
            _ => 1.0,
        }
    }
}

/// Payload carried by each projectile: rendering type plus impact data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectilePayload {
    /// Type of projectile (texture/height/scale selection at render time)
    pub projectile_type: ProjectileType,
    /// Attacker entity ID (for reference)
    pub attacker_id: EntityId,
    /// Defender entity ID (for reference)
    pub defender_id: EntityId,
    /// Damage to deal on impact
    pub damage: f32,
}

/// A projectile in flight
pub type Projectile = TravelingProjectile<ProjectilePayload>;

/// Event generated when projectile hits target
pub struct Impact {
    pub attacker_id: EntityId,
    pub defender_id: EntityId,
    pub damage: f32,
}

/// Manager for all active projectiles
#[derive(Debug, Clone, Default, Serialize)]
#[serde(transparent)]
pub struct ProjectileManager {
    layer: ProjectileLayer<ProjectilePayload>,
}

impl<'de> Deserialize<'de> for ProjectileManager {
    /// Accepts the current `ProjectileLayer` shape, and falls back to an
    /// empty manager for older save formats. Projectiles are transient
    /// visuals lasting well under a second, so dropping any in flight when
    /// loading a legacy save is acceptable.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Compat {
            Current(ProjectileLayer<ProjectilePayload>),
            Legacy(serde::de::IgnoredAny),
        }

        Ok(match Compat::deserialize(deserializer)? {
            Compat::Current(layer) => Self { layer },
            Compat::Legacy(_) => Self::default(),
        })
    }
}

impl ProjectileManager {
    /// Create a new projectile manager
    pub fn new() -> Self {
        Self::default()
    }

    /// Spawn a new projectile targeting an entity
    pub fn spawn(
        &mut self,
        start_pos: (f32, f32),
        end_pos: (f32, f32),
        attack_type: &str,
        attacker_id: EntityId,
        defender_id: EntityId,
        damage: f32,
    ) {
        let projectile_type = ProjectileType::from_attack_type(attack_type);
        let duration = projectile_type.duration();
        let travel_ratio = projectile_type.travel_ratio();
        let payload = ProjectilePayload {
            projectile_type,
            attacker_id,
            defender_id,
            damage,
        };
        self.layer.push(
            TravelingProjectile::new(
                vec2(start_pos.0, start_pos.1),
                vec2(end_pos.0, end_pos.1),
                duration,
                payload,
            )
            .with_travel_ratio(travel_ratio),
        );
    }

    /// Spawn a projectile targeting a position (for structures like dungeon heart)
    pub fn spawn_at_position(
        &mut self,
        start_pos: (f32, f32),
        target_pos: TilePos,
        attack_type: &str,
        attacker_id: EntityId,
        damage: f32,
    ) {
        let end_pos = (target_pos.x as f32, target_pos.y as f32);
        // Use attacker_id as both since there's no defender entity
        self.spawn(
            start_pos,
            end_pos,
            attack_type,
            attacker_id,
            attacker_id,
            damage,
        );
    }

    /// Update all projectiles, removing completed ones and returning impacts
    pub fn update(&mut self, dt: f32) -> Vec<Impact> {
        self.layer
            .update(dt)
            .into_iter()
            .map(|payload| Impact {
                attacker_id: payload.attacker_id,
                defender_id: payload.defender_id,
                damage: payload.damage,
            })
            .collect()
    }

    /// Get all active projectiles for rendering
    pub fn active_projectiles(&self) -> impl Iterator<Item = &Projectile> {
        self.layer.iter()
    }
}

#[cfg(test)]
mod tests;
