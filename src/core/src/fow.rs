// See LICENSE file for copyright and license details.

use common::types::{PlayerId, MapPos, Size2, ZInt};
use core::{CoreEvent};
use internal_state::{InternalState};
use map::{Map, Terrain, distance};
use fov::{fov};
use object::{ObjectTypes};
use unit::{Unit, UnitType, UnitClass};

#[derive(Clone, PartialEq, PartialOrd)]
pub enum TileVisibility {
    No,
    // Bad,
    Normal,
    Excellent,
}

pub fn fov_unit(
    object_types: &ObjectTypes,
    terrain: &Map<Terrain>,
    fow: &mut Map<TileVisibility>,
    unit: &Unit,
) {
    fov_unit_in_pos(object_types, terrain, fow, unit, &unit.pos);
}

pub fn fov_unit_in_pos(
    object_types: &ObjectTypes,
    terrain: &Map<Terrain>,
    fow: &mut Map<TileVisibility>,
    unit: &Unit,
    origin: &MapPos,
) {
    let unit_type = object_types.get_unit_type(&unit.type_id);
    let range = &unit_type.los_range;
    fov(
        terrain,
        origin,
        *range,
        &mut |pos| {
            let distance = distance(origin, pos);
            let vis = calc_visibility(terrain.tile(pos), unit_type, &distance);
            if vis > *fow.tile_mut(pos) {
                *fow.tile_mut(pos) = vis;
            }
        },
    );
}

fn calc_visibility(terrain: &Terrain, unit_type: &UnitType, distance: &ZInt)
    -> TileVisibility
{
    if *distance <= unit_type.cover_los_range {
        TileVisibility::Excellent
    } else if *distance <= unit_type.los_range {
        match terrain {
            &Terrain::Trees => TileVisibility::Normal,
            &Terrain::Plain => TileVisibility::Excellent,
        }
    } else {
        TileVisibility::No
    }
}

/// Fog of War
pub struct Fow {
    map: Map<TileVisibility>,
    player_id: PlayerId,
}

impl Fow {
    pub fn new(map_size: &Size2<ZInt>, player_id: &PlayerId) -> Fow {
        Fow {
            map: Map::new(map_size, TileVisibility::No),
            player_id: player_id.clone(),
        }
    }

    pub fn is_tile_visible(&self, pos: &MapPos) -> bool {
        match *self.map.tile(pos) {
            TileVisibility::Excellent => true,
            TileVisibility::Normal => true,
            TileVisibility::No => false,
        }
    }

    pub fn is_visible(&self, unit_type: &UnitType, pos: &MapPos) -> bool {
        match *self.map.tile(pos) {
            TileVisibility::Excellent => true,
            TileVisibility::Normal => match unit_type.class {
                UnitClass::Infantry => false,
                UnitClass::Vehicle => true,
            },
            TileVisibility::No => false,
        }
    }

    fn clear(&mut self) {
        for pos in self.map.get_iter() {
            *self.map.tile_mut(&pos) = TileVisibility::No;
        }
    }

    fn reset(&mut self, object_types: &ObjectTypes, state: &InternalState) {
        self.clear();
        for (_, unit) in state.units.iter() {
            if unit.player_id == self.player_id {
                fov_unit(object_types, &state.map, &mut self.map, &unit);
            }
        }
    }

    pub fn apply_event(
        &mut self,
        object_types: &ObjectTypes,
        state: &InternalState,
        event: &CoreEvent,
    ) {
        match event {
            &CoreEvent::Move{ref unit_id, ref path} => {
                let unit = state.units.get(unit_id)
                    .expect("BAD MOVE UNIT ID"); // TODO: fix errmsg
                if unit.player_id == self.player_id {
                    for path_node in path.nodes() {
                        let p = &path_node.pos;
                        fov_unit_in_pos(
                            object_types, &state.map, &mut self.map, unit, p);
                    }
                }
            },
            &CoreEvent::EndTurn{ref new_id, ..} => {
                if self.player_id == *new_id {
                    self.reset(object_types, state);
                }
            },
            &CoreEvent::CreateUnit{ref unit_id, ref player_id, ..} => {
                let unit = &state.units[unit_id];
                if self.player_id == *player_id {
                    fov_unit(object_types, &state.map, &mut self.map, unit);
                }
            },
            &CoreEvent::AttackUnit{..} => {},
            &CoreEvent::ShowUnit{..} => {},
            &CoreEvent::HideUnit{..} => {},
        }
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
