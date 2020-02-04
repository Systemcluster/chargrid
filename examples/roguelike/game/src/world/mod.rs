use crate::{visibility::Light, ExternalEvent};
use ecs::{Ecs, Entity};
use grid_2d::{Coord, Size};
use rand::Rng;
use rgb24::Rgb24;
use serde::{Deserialize, Serialize};

mod spatial;
use spatial::Spatial;

mod data;
use data::{Components, Npc};
pub use data::{Disposition, HitPoints, Layer, Tile};

mod realtime_periodic;
pub use realtime_periodic::animation::Context as AnimationContext;
use realtime_periodic::data::RealtimeComponents;

mod query;
pub use query::WorldQuery;

mod explosion;
pub use explosion::spec as explosion_spec;

mod action;
pub use action::WorldAction;

mod spawn;
pub use spawn::WorldSpawn;

#[derive(Debug, Serialize, Deserialize)]
pub struct World {
    pub ecs: Ecs<Components>,
    pub realtime_components: RealtimeComponents,
    pub spatial: Spatial,
}

impl World {
    pub fn new(size: Size) -> Self {
        let ecs = Ecs::new();
        let realtime_components = RealtimeComponents::default();
        let spatial = Spatial::new(size);
        Self {
            ecs,
            realtime_components,
            spatial,
        }
    }
}

impl World {
    pub fn to_render_entities<'a>(&'a self) -> impl 'a + Iterator<Item = ToRenderEntity> {
        let tile_component = &self.ecs.components.tile;
        let spatial = &self.spatial;
        let realtime_fade_component = &self.realtime_components.fade;
        let colour_hint_component = &self.ecs.components.colour_hint;
        let blood_component = &self.ecs.components.blood;
        tile_component.iter().filter_map(move |(entity, &tile)| {
            if let Some(location) = spatial.location(entity) {
                let fade = realtime_fade_component.get(entity).and_then(|f| f.state.fading());
                let colour_hint = colour_hint_component.get(entity).cloned();
                let blood = blood_component.contains(entity);
                Some(ToRenderEntity {
                    coord: location.coord,
                    layer: location.layer,
                    tile,
                    fade,
                    colour_hint,
                    blood,
                })
            } else {
                None
            }
        })
    }

    pub fn all_lights_by_coord<'a>(&'a self) -> impl 'a + Iterator<Item = (Coord, &'a Light)> {
        self.ecs
            .components
            .light
            .iter()
            .filter_map(move |(entity, light)| self.spatial.coord(entity).map(|&coord| (coord, light)))
    }

    pub fn character_info(&self, entity: Entity) -> Option<CharacterInfo> {
        let &coord = self.spatial.coord(entity)?;
        let &hit_points = self.ecs.components.hit_points.get(entity)?;
        Some(CharacterInfo { coord, hit_points })
    }
}

impl World {
    pub fn entity_coord(&self, entity: Entity) -> Option<Coord> {
        self.spatial.coord(entity).cloned()
    }
    pub fn entity_npc(&self, entity: Entity) -> &Npc {
        self.ecs.components.npc.get(entity).unwrap()
    }
    pub fn entity_exists(&self, entity: Entity) -> bool {
        self.ecs.entity_allocator.exists(entity)
    }
    pub fn size(&self) -> Size {
        self.spatial.grid_size()
    }
    pub fn is_gameplay_blocked(&self) -> bool {
        !self.ecs.components.blocks_gameplay.is_empty()
    }
    pub fn animation_tick<R: Rng>(
        &mut self,
        animation_context: &mut AnimationContext,
        external_events: &mut Vec<ExternalEvent>,
        rng: &mut R,
    ) {
        animation_context.tick(self, external_events, rng)
    }
}

pub struct ToRenderEntity {
    pub coord: Coord,
    pub layer: Option<Layer>,
    pub tile: Tile,
    pub fade: Option<u8>,
    pub colour_hint: Option<Rgb24>,
    pub blood: bool,
}

#[derive(Serialize, Deserialize)]
pub struct CharacterInfo {
    pub coord: Coord,
    pub hit_points: HitPoints,
}
