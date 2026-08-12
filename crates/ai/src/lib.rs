//! Goal-Oriented Action Planning.
//!
//! The algorithm matches the public F.E.A.R. AI talks (Jeff Orkin, GDC 2006):
//! sensors write a world state, goals pick a desired state, A* plans a sequence
//! of actions whose effects satisfy that state.

mod planner;
mod state;

pub use planner::{Action, Goal, Plan, Planner};
pub use state::{Key, Value, WorldState};

/// Replica-style combat vocabulary used by the demo soldier.
pub mod replica {
    use super::{Action, Goal, Key, Planner, Value, WorldState};

    pub const TARGET_DEAD: Key = 1;
    pub const WEAPON_LOADED: Key = 2;
    pub const IN_COVER: Key = 3;
    pub const TARGET_VISIBLE: Key = 4;
    pub const HAS_WEAPON: Key = 5;
    pub const AT_LOCATION: Key = 6;
    pub const ALERT: Key = 7; // 0 patrol, 1 investigate, 2 combat

    pub const LOC_PATROL: Value = 1;
    pub const LOC_DISTURBANCE: Value = 2;
    pub const LOC_COVER: Value = 3;
    pub const LOC_ENGAGE: Value = 4;

    pub fn planner() -> Planner {
        let mut p = Planner::new();
        p.add_action(Action {
            name: "Patrol",
            cost: 5,
            pre: WorldState::from_pairs(&[(ALERT, 0)]),
            effects: WorldState::from_pairs(&[(AT_LOCATION, LOC_PATROL)]),
        });
        p.add_action(Action {
            name: "Investigate",
            cost: 3,
            pre: WorldState::from_pairs(&[(ALERT, 1)]),
            effects: WorldState::from_pairs(&[(AT_LOCATION, LOC_DISTURBANCE)]),
        });
        p.add_action(Action {
            name: "TakeCover",
            cost: 2,
            pre: WorldState::from_pairs(&[(ALERT, 2), (HAS_WEAPON, 1)]),
            effects: WorldState::from_pairs(&[(IN_COVER, 1), (AT_LOCATION, LOC_COVER)]),
        });
        p.add_action(Action {
            name: "Reload",
            cost: 2,
            pre: WorldState::from_pairs(&[(HAS_WEAPON, 1), (WEAPON_LOADED, 0)]),
            effects: WorldState::from_pairs(&[(WEAPON_LOADED, 1)]),
        });
        p.add_action(Action {
            name: "Advance",
            cost: 3,
            pre: WorldState::from_pairs(&[(ALERT, 2), (HAS_WEAPON, 1)]),
            effects: WorldState::from_pairs(&[(AT_LOCATION, LOC_ENGAGE), (TARGET_VISIBLE, 1)]),
        });
        p.add_action(Action {
            name: "Attack",
            cost: 1,
            pre: WorldState::from_pairs(&[
                (ALERT, 2),
                (HAS_WEAPON, 1),
                (WEAPON_LOADED, 1),
                (TARGET_VISIBLE, 1),
            ]),
            effects: WorldState::from_pairs(&[(TARGET_DEAD, 1)]),
        });
        p.add_action(Action {
            name: "Suppress",
            cost: 2,
            pre: WorldState::from_pairs(&[
                (ALERT, 2),
                (IN_COVER, 1),
                (WEAPON_LOADED, 1),
                (TARGET_VISIBLE, 1),
            ]),
            effects: WorldState::from_pairs(&[(TARGET_VISIBLE, 1)]),
        });
        p
    }

    pub fn kill_enemy() -> Goal {
        Goal {
            name: "KillEnemy",
            desired: WorldState::from_pairs(&[(TARGET_DEAD, 1)]),
            priority: 100,
        }
    }

    pub fn investigate() -> Goal {
        Goal {
            name: "InvestigateDisturbance",
            desired: WorldState::from_pairs(&[(AT_LOCATION, LOC_DISTURBANCE)]),
            priority: 40,
        }
    }

    pub fn patrol() -> Goal {
        Goal {
            name: "Patrol",
            desired: WorldState::from_pairs(&[(AT_LOCATION, LOC_PATROL)]),
            priority: 10,
        }
    }
}
