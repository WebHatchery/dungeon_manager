//! Typed onboarding content and completion conditions.

use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TutorialData {
    #[serde(default)]
    pub steps: Vec<TutorialStepData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TutorialStepData {
    pub id: String,
    pub title: String,
    pub hint: String,
    pub completion: TutorialCompletion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TutorialCompletion {
    Dig { target: usize },
    Claim { target: usize },
    Room { room: String },
    Recruit,
    Combat,
    Trap,
    Payday,
    Research,
    Wave,
    Spell,
    Conversion,
    Temple,
}

pub fn load_tutorial() -> Result<TutorialData, Box<dyn Error>> {
    let data: TutorialData = macroquad_toolkit::include_json!("../../assets/data/tutorial.json")?;
    Ok(data)
}

impl TutorialData {
    pub fn validate(&self) -> Vec<String> {
        let mut problems = Vec::new();
        let mut ids = std::collections::HashSet::new();
        for step in &self.steps {
            if step.id.trim().is_empty() || !ids.insert(step.id.clone()) {
                problems.push(format!(
                    "tutorial step id is empty or duplicated: `{}`",
                    step.id
                ));
            }
            if step.title.trim().is_empty() {
                problems.push(format!("tutorial step `{}` has no title", step.id));
            }
            if step.hint.trim().is_empty() {
                problems.push(format!("tutorial step `{}` has no hint", step.id));
            }
            match &step.completion {
                TutorialCompletion::Dig { target } | TutorialCompletion::Claim { target }
                    if *target == 0 =>
                {
                    problems.push(format!("tutorial step `{}` has a zero target", step.id));
                }
                TutorialCompletion::Room { room } if room.trim().is_empty() => {
                    problems.push(format!("tutorial step `{}` has no room id", step.id));
                }
                _ => {}
            }
        }
        problems
    }
}
