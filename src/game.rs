/// some gameplay functions
use crate::*;
use thunderdome::*;

pub const ENEMY_TEAM: Team = Team::Red;
pub const PLAYER_TEAM: Team = Team::Blue;

#[derive(Debug)]
pub struct Actor {
    pub pos: IVec2,
    pub draw_pos: Vec2,
    pub sprite_coords: IVec2,
    pub team: Team,
    pub unit_type: UnitType,
    pub hp: i32,
    pub has_moved: bool,
}

pub const HP_MAX: i32 = 10;

#[derive(DeJson, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Team {
    Blue,
    Red,
}

#[derive(DeJson, Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitType {
    Infantry,
    Tank,
}

/// used for determining movement cost
#[derive(Default, DeJson, Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroundType {
    #[default]
    Ground,
    Water,
}

/// used for determining movement cost
#[derive(Default, DeJson, Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainType {
    #[default]
    None,
    Street,
    Forest,
}

/// returns units of other team which are in attack range
pub fn enemies_in_range(s: &GameState, me: Index) -> Vec<(Index, IVec2)> {
    let pos = s.entities[me].pos;
    let my_team = s.entities[me].team;
    let neighbors = get_neighbors(pos, &s.grids.ground);
    let in_range = neighbors
        .iter()
        .filter_map(|pos| actor_at_pos(s, *pos).map(|index| (index, *pos)));
    in_range
        .filter(|(index, _)| s.entities[*index].team != my_team)
        .collect_vec()
}

pub fn actor_at_pos(s: &GameState, pos: IVec2) -> Option<Index> {
    for (index, actor) in s.entities.iter() {
        if actor.pos == pos {
            return Some(index);
        }
    }
    None
}
