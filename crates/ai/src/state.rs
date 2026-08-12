use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

pub type Key = u16;
pub type Value = i32;

/// Sparse world state. Keys not present are treated as 0 by [`WorldState::get`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WorldState {
    values: BTreeMap<Key, Value>,
}

impl WorldState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_pairs(pairs: &[(Key, Value)]) -> Self {
        let mut s = Self::new();
        for &(k, v) in pairs {
            s.set(k, v);
        }
        s
    }

    pub fn get(&self, key: Key) -> Value {
        self.values.get(&key).copied().unwrap_or(0)
    }

    pub fn set(&mut self, key: Key, value: Value) {
        if value == 0 {
            self.values.remove(&key);
        } else {
            self.values.insert(key, value);
        }
    }

    pub fn satisfies(&self, desired: &WorldState) -> bool {
        desired.values.iter().all(|(k, v)| self.get(*k) == *v)
    }

    pub fn apply(&self, effects: &WorldState) -> WorldState {
        let mut next = self.clone();
        for (&k, &v) in &effects.values {
            next.set(k, v);
        }
        next
    }

    pub fn heuristic(&self, goal: &WorldState) -> u32 {
        goal.values
            .iter()
            .filter(|(k, v)| self.get(**k) != **v)
            .count() as u32
    }
}

impl Hash for WorldState {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for (k, v) in &self.values {
            k.hash(state);
            v.hash(state);
        }
    }
}
