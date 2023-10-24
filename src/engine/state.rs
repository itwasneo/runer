use std::collections::HashMap;
use std::sync::Arc;

use smol::lock::Mutex;
use smol::process::Child;

use crate::model::runer::{Blueprint, Flow, Rune};
use serde::Serialize;

/// It represents the Application State throughout the Application
/// lifetime. It consists fields that should be available to Application
/// threads without compromsing thread safety.
///
/// Care that _blueprints_, _env_, and _flows_ fields are behind an Arc
/// pointer which makes them implicitly immutable.
///
/// _handles_ field on the other hand is behind a Mutex guard which allows
/// the Application to access the data safely.
pub struct State {
    pub blueprints: Option<Arc<HashMap<String, Blueprint>>>,
    pub env: Option<Arc<HashMap<String, Vec<(String, String)>>>>,
    pub flows: Option<Arc<Vec<Flow>>>,
    pub handles: Option<Arc<Mutex<HashMap<u32, Child>>>>,
}

/// By default the Application has no state.
impl Default for State {
    fn default() -> Self {
        Self {
            blueprints: None,
            env: None,
            flows: None,
            handles: None,
        }
    }
}

impl State {
    /// This function builds the Application State, according to the given
    /// Rune's Fragments.
    pub fn from_rune(mut self, rune: Rune) -> Self {
        if let Some(blueprints) = rune.blueprints {
            self.blueprints = Some(Arc::new(blueprints));
        }
        if let Some(env) = rune.env {
            self.env = Some(Arc::new(env));
        }
        if let Some(flows) = rune.flows {
            self.flows = Some(Arc::new(flows));
            self.handles = Some(Arc::new(Mutex::new(HashMap::<u32, Child>::new())));
        }
        self
    }

    pub fn serialize_blueprints(self) -> String {
        let mut result: Vec<String> = vec![];
        if let Some(blueprints) = self.blueprints {
            for p in blueprints.iter() {
                result.push(p.0.to_owned());
            }
        }
        result.join("\n")
    }
}
