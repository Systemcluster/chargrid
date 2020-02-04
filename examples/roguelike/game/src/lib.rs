pub use direction::CardinalDirection;
pub use grid_2d::{Coord, Grid, Size};
use rand::{Rng, SeedableRng};
use rand_isaac::Isaac64Rng;
use rgb24::Rgb24;
use serde::{Deserialize, Serialize};
use shadowcast::Context as ShadowcastContext;
use std::time::Duration;

mod behaviour;
mod visibility;
mod world;

use behaviour::{Agent, BehaviourContext};
use ecs::ComponentTable;
pub use ecs::Entity;
pub use visibility::{CellVisibility, Omniscient, VisibilityGrid};
use world::{AnimationContext, World, WorldAction, WorldQuery, WorldSpawn};
pub use world::{CharacterInfo, HitPoints, Layer, Tile, ToRenderEntity};

pub struct Config {
    pub omniscient: Option<Omniscient>,
}

/// Events which the game can report back to the io layer so it can
/// respond with a sound/visual effect.
#[derive(Serialize, Deserialize, Clone, Copy)]
pub enum ExternalEvent {
    Explosion(Coord),
}

pub enum GameControlFlow {
    GameOver,
}

#[derive(Clone, Copy, Debug)]
pub enum Input {
    Walk(CardinalDirection),
    Fire(Coord),
    Wait,
}

#[derive(Serialize, Deserialize)]
pub struct Game {
    world: World,
    visibility_grid: VisibilityGrid,
    player: Entity,
    last_player_info: CharacterInfo,
    rng: Isaac64Rng,
    frame_count: u64,
    events: Vec<ExternalEvent>,
    shadowcast_context: ShadowcastContext<u8>,
    behaviour_context: BehaviourContext,
    animation_context: AnimationContext,
    agents: ComponentTable<Agent>,
    agents_to_remove: Vec<Entity>,
}

impl Game {
    pub fn new<R: Rng>(config: &Config, rng: &mut R) -> Self {
        let s = include_str!("terrain.txt");
        let rows = s.split('\n').filter(|s| !s.is_empty()).collect::<Vec<_>>();
        let size = Size::new_u16(rows[0].len() as u16, rows.len() as u16);
        let mut world = World::new(size);
        let mut agents = ComponentTable::default();
        let mut player = None;
        for (y, row) in rows.iter().enumerate() {
            for (x, ch) in row.chars().enumerate() {
                if ch.is_control() {
                    continue;
                }
                let coord = Coord::new(x as i32, y as i32);
                match ch {
                    '.' => {
                        world.spawn_floor(coord);
                    }
                    '*' => {
                        world.spawn_floor(coord);
                        world.spawn_light(coord, Rgb24::new(187, 187, 187));
                    }
                    ',' => {
                        world.spawn_carpet(coord);
                    }
                    '#' => {
                        world.spawn_floor(coord);
                        world.spawn_wall(coord);
                    }
                    '@' => {
                        world.spawn_floor(coord);
                        player = Some(world.spawn_player(coord));
                    }
                    'f' => {
                        world.spawn_floor(coord);
                        let entity = world.spawn_former_human(coord);
                        agents.insert(entity, Agent::new(size));
                    }
                    'h' => {
                        world.spawn_floor(coord);
                        let entity = world.spawn_human(coord);
                        agents.insert(entity, Agent::new(size));
                    }
                    _ => log::warn!("unexpected char in terrain: {} ({})", ch.escape_unicode(), ch),
                }
            }
        }
        let player = player.expect("didn't create player");
        let last_player_info = world.character_info(player).expect("couldn't get info for player");
        let mut game = Self {
            world,
            visibility_grid: VisibilityGrid::new(size),
            player,
            last_player_info,
            rng: Isaac64Rng::seed_from_u64(rng.gen()),
            frame_count: 0,
            events: Vec::new(),
            shadowcast_context: ShadowcastContext::default(),
            behaviour_context: BehaviourContext::new(size),
            animation_context: AnimationContext::default(),
            agents,
            agents_to_remove: Vec::new(),
        };
        game.update_behaviour();
        game.update_visibility(config);
        game
    }
    pub fn is_gameplay_blocked(&self) -> bool {
        self.world.is_gameplay_blocked()
    }
    pub fn update_visibility(&mut self, config: &Config) {
        if let Some(player_coord) = self.world.entity_coord(self.player) {
            self.visibility_grid.update(
                player_coord,
                &self.world,
                &mut self.shadowcast_context,
                config.omniscient,
            );
        }
    }
    fn update_behaviour(&mut self) {
        self.behaviour_context.update(self.player, &self.world);
    }
    #[must_use]
    pub fn handle_input(&mut self, input: Input, config: &Config) -> Option<GameControlFlow> {
        if !self.is_gameplay_blocked() {
            match input {
                Input::Walk(direction) => self.world.character_walk_in_direction(self.player, direction),
                Input::Fire(coord) => self.world.character_fire_rocket(self.player, coord),
                Input::Wait => (),
            }
        }
        self.update_visibility(config);
        self.update_behaviour();
        self.npc_turn();
        self.update_last_player_info();
        if self.is_game_over() {
            Some(GameControlFlow::GameOver)
        } else {
            None
        }
    }
    pub fn handle_npc_turn(&mut self) {
        if !self.is_gameplay_blocked() {
            self.update_behaviour();
            self.npc_turn();
        }
    }
    fn npc_turn(&mut self) {
        for (entity, agent) in self.agents.iter_mut() {
            if !self.world.entity_exists(entity) {
                self.agents_to_remove.push(entity);
                continue;
            }
            if let Some(input) = agent.act(
                entity,
                &self.world,
                self.player,
                &mut self.behaviour_context,
                &mut self.shadowcast_context,
            ) {
                match input {
                    Input::Walk(direction) => self.world.character_walk_in_direction(entity, direction),
                    Input::Fire(coord) => self.world.character_fire_rocket(entity, coord),
                    Input::Wait => (),
                }
            }
        }
        for entity in self.agents_to_remove.drain(..) {
            self.agents.remove(entity);
        }
    }
    #[must_use]
    pub fn handle_tick(&mut self, _since_last_tick: Duration, config: &Config) -> Option<GameControlFlow> {
        self.events.clear();
        self.update_visibility(config);
        self.world
            .animation_tick(&mut self.animation_context, &mut self.events, &mut self.rng);
        self.frame_count += 1;
        self.update_last_player_info();
        if self.is_game_over() {
            Some(GameControlFlow::GameOver)
        } else {
            None
        }
    }
    pub fn events(&self) -> impl '_ + Iterator<Item = ExternalEvent> {
        self.events.iter().cloned()
    }
    pub fn player_info(&self) -> &CharacterInfo {
        &self.last_player_info
    }
    pub fn world_size(&self) -> Size {
        self.world.size()
    }
    pub fn to_render_entities<'a>(&'a self) -> impl 'a + Iterator<Item = ToRenderEntity> {
        self.world.to_render_entities()
    }
    pub fn visibility_grid(&self) -> &VisibilityGrid {
        &self.visibility_grid
    }
    pub fn contains_wall(&self, coord: Coord) -> bool {
        self.world.is_wall_at_coord(coord)
    }
    fn update_last_player_info(&mut self) {
        if let Some(character_info) = self.world.character_info(self.player) {
            self.last_player_info = character_info;
        } else {
            self.last_player_info.hit_points.current = 0;
        }
    }
    fn is_game_over(&self) -> bool {
        self.last_player_info.hit_points.current == 0
    }
}
