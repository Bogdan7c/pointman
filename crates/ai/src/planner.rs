use crate::state::WorldState;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

#[derive(Clone, Debug)]
pub struct Action {
    pub name: &'static str,
    pub cost: u32,
    pub pre: WorldState,
    pub effects: WorldState,
}

impl Action {
    pub fn available(&self, world: &WorldState) -> bool {
        world.satisfies(&self.pre)
    }
}

#[derive(Clone, Debug)]
pub struct Goal {
    pub name: &'static str,
    pub desired: WorldState,
    pub priority: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    pub goal: &'static str,
    pub steps: Vec<&'static str>,
    pub cost: u32,
}

#[derive(Default)]
pub struct Planner {
    actions: Vec<Action>,
}

#[derive(Clone, Eq, PartialEq)]
struct Node {
    cost: u32,
    est: u32,
    state: WorldState,
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        (other.cost + other.est).cmp(&(self.cost + self.est))
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Planner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_action(&mut self, action: Action) {
        self.actions.push(action);
    }

    pub fn plan(&self, world: &WorldState, goal: &Goal) -> Option<Plan> {
        if world.satisfies(&goal.desired) {
            return Some(Plan {
                goal: goal.name,
                steps: Vec::new(),
                cost: 0,
            });
        }

        let mut open = BinaryHeap::new();
        open.push(Node {
            cost: 0,
            est: world.heuristic(&goal.desired),
            state: world.clone(),
        });
        let mut came: HashMap<WorldState, (WorldState, &'static str, u32)> = HashMap::new();
        let mut best: HashMap<WorldState, u32> = HashMap::new();
        best.insert(world.clone(), 0);

        let mut iterations = 0usize;
        while let Some(node) = open.pop() {
            iterations += 1;
            if iterations > 4096 {
                break;
            }
            if node.state.satisfies(&goal.desired) {
                return Some(reconstruct(goal.name, &came, &node.state));
            }
            for action in &self.actions {
                if !action.available(&node.state) {
                    continue;
                }
                let next = node.state.apply(&action.effects);
                if next == node.state {
                    continue;
                }
                let g = node.cost + action.cost;
                if best.get(&next).is_some_and(|b| g >= *b) {
                    continue;
                }
                best.insert(next.clone(), g);
                came.insert(next.clone(), (node.state.clone(), action.name, g));
                open.push(Node {
                    cost: g,
                    est: next.heuristic(&goal.desired),
                    state: next,
                });
            }
        }
        None
    }

    pub fn best_goal<'a>(&self, world: &WorldState, goals: &'a [Goal]) -> Option<(&'a Goal, Plan)> {
        let mut ranked: Vec<_> = goals.iter().collect();
        ranked.sort_by_key(|g| std::cmp::Reverse(g.priority));
        for goal in ranked {
            if let Some(plan) = self.plan(world, goal) {
                return Some((goal, plan));
            }
        }
        None
    }
}

fn reconstruct(
    goal: &'static str,
    came: &HashMap<WorldState, (WorldState, &'static str, u32)>,
    end: &WorldState,
) -> Plan {
    let mut steps = Vec::new();
    let mut cur = end.clone();
    let mut cost = 0;
    while let Some((prev, name, c)) = came.get(&cur) {
        steps.push(*name);
        cost = *c;
        cur = prev.clone();
    }
    steps.reverse();
    Plan { goal, steps, cost }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::replica;

    #[test]
    fn reloads_then_attacks() {
        let planner = replica::planner();
        let world = WorldState::from_pairs(&[
            (replica::ALERT, 2),
            (replica::HAS_WEAPON, 1),
            (replica::WEAPON_LOADED, 0),
            (replica::TARGET_VISIBLE, 1),
        ]);
        let plan = planner.plan(&world, &replica::kill_enemy()).unwrap();
        assert_eq!(plan.steps, ["Reload", "Attack"]);
    }

    #[test]
    fn patrol_when_idle() {
        let planner = replica::planner();
        let world = WorldState::from_pairs(&[(replica::ALERT, 0)]);
        let goals = [replica::kill_enemy(), replica::investigate(), replica::patrol()];
        let (goal, plan) = planner.best_goal(&world, &goals).unwrap();
        assert_eq!(goal.name, "Patrol");
        assert_eq!(plan.steps, ["Patrol"]);
    }

    #[test]
    fn combat_beats_patrol() {
        let planner = replica::planner();
        let world = WorldState::from_pairs(&[
            (replica::ALERT, 2),
            (replica::HAS_WEAPON, 1),
            (replica::WEAPON_LOADED, 1),
            (replica::TARGET_VISIBLE, 1),
        ]);
        let goals = [replica::kill_enemy(), replica::patrol()];
        let (goal, plan) = planner.best_goal(&world, &goals).unwrap();
        assert_eq!(goal.name, "KillEnemy");
        assert_eq!(plan.steps, ["Attack"]);
    }
}
