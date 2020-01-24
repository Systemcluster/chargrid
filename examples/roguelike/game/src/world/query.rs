use crate::{
    visibility::Light,
    world::{
        data::{Components, Layer, Tile},
        realtime_periodic::data::RealtimeComponents,
        spatial_grid::SpatialGrid,
    },
};
use ecs::{Ecs, Entity};
use grid_2d::Coord;
use rgb24::Rgb24;

pub mod component {
    use super::*;
    use ecs::ComponentTable;

    pub fn is_solid_feature_at_coord(
        solid_component: &ComponentTable<()>,
        spatial_grid: &SpatialGrid,
        coord: Coord,
    ) -> bool {
        let cell = spatial_grid.get_checked(coord);
        if let Some(feature) = cell.feature {
            solid_component.contains(feature)
        } else {
            false
        }
    }
}

pub fn is_solid_feature_at_coord(ecs: &Ecs<Components>, spatial_grid: &SpatialGrid, coord: Coord) -> bool {
    component::is_solid_feature_at_coord(&ecs.components.solid, spatial_grid, coord)
}

pub fn is_wall_at_coord(ecs: &Ecs<Components>, spatial_grid: &SpatialGrid, coord: Coord) -> bool {
    if let Some(spatial_cell) = spatial_grid.get(coord) {
        if let Some(entity) = spatial_cell.feature {
            ecs.components.tile.get(entity) == Some(&Tile::Wall)
        } else {
            false
        }
    } else {
        false
    }
}

pub fn is_npc_at_coord(ecs: &Ecs<Components>, spatial_grid: &SpatialGrid, coord: Coord) -> bool {
    if let Some(spatial_cell) = spatial_grid.get(coord) {
        if let Some(entity) = spatial_cell.character {
            ecs.components.npc.contains(entity)
        } else {
            false
        }
    } else {
        false
    }
}

pub fn get_opacity_at_coord(ecs: &Ecs<Components>, spatial_grid: &SpatialGrid, coord: Coord) -> u8 {
    spatial_grid
        .get(coord)
        .and_then(|c| c.feature)
        .and_then(|e| ecs.components.opacity.get(e).cloned())
        .unwrap_or(0)
}

pub fn get_character_at_coord(spatial_grid: &SpatialGrid, coord: Coord) -> Option<Entity> {
    spatial_grid.get(coord).and_then(|cell| cell.character)
}

pub fn all_lights_by_coord<'a>(ecs: &'a Ecs<Components>) -> impl 'a + Iterator<Item = (Coord, &'a Light)> {
    ecs.components.light.iter().filter_map(move |(entity, light)| {
        ecs.components
            .location
            .get(entity)
            .map(|location| (location.coord, light))
    })
}

pub fn all_entites_to_render<'a>(
    ecs: &'a Ecs<Components>,
    realtime_components: &'a RealtimeComponents,
) -> impl 'a + Iterator<Item = ToRenderEntity> {
    let tile_component = &ecs.components.tile;
    let location_component = &ecs.components.location;
    let realtime_fade_component = &realtime_components.fade;
    let colour_hint_component = &ecs.components.colour_hint;
    tile_component.iter().filter_map(move |(entity, &tile)| {
        if let Some(location) = location_component.get(entity) {
            let fade = realtime_fade_component.get(entity).and_then(|f| f.state.fading());
            let colour_hint = colour_hint_component.get(entity).cloned();
            Some(ToRenderEntity {
                coord: location.coord,
                layer: location.layer,
                tile,
                fade,
                colour_hint,
            })
        } else {
            None
        }
    })
}

pub struct ToRenderEntity {
    pub coord: Coord,
    pub layer: Layer,
    pub tile: Tile,
    pub fade: Option<u8>,
    pub colour_hint: Option<Rgb24>,
}