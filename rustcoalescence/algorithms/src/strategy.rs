use serde::{Deserialize, Serialize};

#[expect(clippy::module_name_repetitions)]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct RestartFixUpStrategy {
    #[serde(alias = "deme", alias = "ood")]
    pub out_of_deme: OutOfDemeStrategy,
    #[serde(alias = "habitat", alias = "ooh")]
    pub out_of_habitat: OutOfHabitatStrategy,
    #[serde(alias = "dup", alias = "coa")]
    pub coalescence: CoalescenceStrategy,
}

impl Default for RestartFixUpStrategy {
    fn default() -> Self {
        Self {
            out_of_deme: OutOfDemeStrategy::Abort,
            out_of_habitat: OutOfHabitatStrategy::Abort,
            coalescence: CoalescenceStrategy::Abort,
        }
    }
}

#[expect(clippy::module_name_repetitions)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum OutOfDemeStrategy {
    Abort,
    Dispersal,
}

#[expect(clippy::module_name_repetitions)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum OutOfHabitatStrategy {
    Abort,
    #[serde(alias = "Uniform")]
    UniformDispersal,
}

#[expect(clippy::module_name_repetitions)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CoalescenceStrategy {
    Abort,
    Coalescence,
}
