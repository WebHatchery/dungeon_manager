use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum OwnerId {
    Player,
    RivalKeeper(u8),
    Heroes,
    #[default]
    Neutral,
    Wild,
    AboveGround,
}

impl OwnerId {
    pub fn is_player_controlled(&self) -> bool {
        matches!(self, Self::Player)
    }

    pub fn is_dungeon_keeper(&self) -> bool {
        matches!(self, Self::Player | Self::RivalKeeper(_))
    }

    pub fn is_hostile_to(&self, other: &OwnerId) -> bool {
        use OwnerId::*;

        if self == other {
            return false;
        }

        match (self, other) {
            (Neutral, _) | (_, Neutral) => false,
            (Wild, Wild) => false,
            (Player, RivalKeeper(_)) | (RivalKeeper(_), Player) => true,
            (RivalKeeper(a), RivalKeeper(b)) => a != b,
            (Heroes, Player | RivalKeeper(_) | Wild) => true,
            (Player | RivalKeeper(_) | Wild, Heroes) => true,
            (AboveGround, Player | RivalKeeper(_) | Wild) => true,
            (Player | RivalKeeper(_) | Wild, AboveGround) => true,
            _ => false,
        }
    }

    pub fn label(&self) -> String {
        match self {
            OwnerId::Player => "player".to_string(),
            OwnerId::RivalKeeper(index) => format!("keeper{}", index),
            OwnerId::Heroes => "heroes".to_string(),
            OwnerId::Neutral => "neutral".to_string(),
            OwnerId::Wild => "wild".to_string(),
            OwnerId::AboveGround => "above_ground".to_string(),
        }
    }
}

impl fmt::Display for OwnerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.label())
    }
}

impl FromStr for OwnerId {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let normalized = value.trim().to_ascii_lowercase();

        match normalized.as_str() {
            "player" | "player0" | "keeper" | "keeper0" | "human" => Ok(Self::Player),
            "heroes" | "hero" | "hero_faction" => Ok(Self::Heroes),
            "neutral" | "none" | "unclaimed" => Ok(Self::Neutral),
            "wild" | "wildlife" => Ok(Self::Wild),
            "above_ground" | "aboveground" | "surface" | "town" => Ok(Self::AboveGround),
            _ => {
                if let Some(index) = normalized
                    .strip_prefix("keeper")
                    .or_else(|| normalized.strip_prefix("rival_keeper"))
                    .or_else(|| normalized.strip_prefix("rival"))
                {
                    let index = index
                        .parse::<u8>()
                        .map_err(|_| format!("Invalid rival keeper owner '{value}'"))?;
                    return Ok(Self::RivalKeeper(index));
                }

                Err(format!("Unknown owner '{value}'"))
            }
        }
    }
}

pub fn owner_from_label(value: &str) -> OwnerId {
    OwnerId::from_str(value).unwrap_or(OwnerId::Neutral)
}

#[cfg(test)]
mod tests;
