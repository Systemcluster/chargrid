use crate::controls::{AppInput, Controls};
pub use game::Input as GameInput;
use game::{Direction, Game};
use line_2d::{Config as LineConfig, LineSegment};
use prototty::event_routine::common_event::*;
use prototty::event_routine::*;
use prototty::input::*;
use prototty::render::*;
use prototty_storage::{format, Storage};
use rand::{Rng, SeedableRng};
use rand_isaac::Isaac64Rng;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use std::time::Duration;

const AUTO_SAVE_PERIOD: Duration = Duration::from_secs(2);

pub struct GameView {
    last_offset: Coord,
}

impl GameView {
    pub fn new() -> Self {
        Self {
            last_offset: Coord::new(0, 0),
        }
    }
    pub fn absolute_coord_to_game_coord(&self, coord: Coord) -> Coord {
        coord - self.last_offset
    }
}

impl<'a> View<&'a Game> for GameView {
    fn view<F: Frame, C: ColModify>(&mut self, game: &'a Game, context: ViewContext<C>, frame: &mut F) {
        let grid = game.grid();
        for (coord, cell) in grid.enumerate() {
            let view_cell = ViewCell::new().with_character('.');
            let view_cell = if let Some(_wall) = cell.wall() {
                view_cell.with_character('#')
            } else {
                view_cell
            };
            let view_cell = if let Some(_character) = cell.character() {
                view_cell.with_character('@')
            } else {
                view_cell
            };
            frame.set_cell_relative(coord, 0, view_cell, context);
        }
        for particle in game.particles() {
            let view_cell = ViewCell::new()
                .with_character('*')
                .with_foreground(Rgb24::new(0, 255, 255));
            frame.set_cell_relative(particle.coord(), 0, view_cell, context);
        }
        for (coord, cell) in game.trails_grid().enumerate() {
            if let Some(col) = cell.col() {
                let view_cell = ViewCell::new().with_background(col);
                frame.set_cell_relative(coord, 0, view_cell, context);
            }
        }
        self.last_offset = context.offset;
    }
}

#[derive(Serialize, Deserialize)]
struct GameInstance {
    rng: Isaac64Rng,
    game: Game,
}

#[derive(Clone)]
pub enum RngSeed {
    Entropy,
    U64(u64),
}

impl GameInstance {
    fn new(mut rng: Isaac64Rng) -> Self {
        Self {
            game: Game::new(&mut rng),
            rng,
        }
    }
}

pub struct GameData<S: Storage> {
    instance: Option<GameInstance>,
    controls: Controls,
    rng_source: Isaac64Rng,
    storage_wrapper: StorageWrapper<S>,
    last_aim_with_mouse: bool,
}

struct StorageWrapper<S: Storage> {
    storage: S,
    save_key: String,
    until_auto_save: Duration,
}

impl<S: Storage> StorageWrapper<S> {
    pub fn save_instance(&mut self, instance: &GameInstance) {
        self.storage
            .store(&self.save_key, instance, format::Bincode)
            .expect("failed to save instance");
    }
    pub fn clear_instance(&mut self) {
        let _ = self.storage.remove(&self.save_key);
    }
    pub fn autosave_tick(&mut self, instance: &GameInstance, since_previous: Duration) {
        if let Some(remaining) = self.until_auto_save.checked_sub(since_previous) {
            self.until_auto_save = remaining;
        } else {
            self.save_instance(instance);
            self.until_auto_save = AUTO_SAVE_PERIOD;
        }
    }
}

impl<S: Storage> GameData<S> {
    pub fn new(controls: Controls, storage: S, save_key: String, rng_seed: RngSeed) -> Self {
        let instance = storage.load(&save_key, format::Bincode).ok();
        let rng_source = match rng_seed {
            RngSeed::Entropy => Isaac64Rng::from_entropy(),
            RngSeed::U64(u64) => Isaac64Rng::seed_from_u64(u64),
        };
        let storage_wrapper = StorageWrapper {
            storage,
            save_key,
            until_auto_save: AUTO_SAVE_PERIOD,
        };
        Self {
            instance,
            controls,
            rng_source,
            storage_wrapper,
            last_aim_with_mouse: false,
        }
    }
    pub fn has_instance(&self) -> bool {
        self.instance.is_some()
    }
    pub fn instantiate(&mut self) {
        let rng = Isaac64Rng::from_seed(self.rng_source.gen());
        self.instance = Some(GameInstance::new(rng));
    }
    pub fn save_instance(&mut self) {
        if let Some(instance) = self.instance.as_ref() {
            self.storage_wrapper.save_instance(instance);
        } else {
            self.storage_wrapper.clear_instance();
        }
    }
    pub fn clear_instance(&mut self) {
        self.instance = None;
        self.storage_wrapper.clear_instance();
    }
    pub fn game(&self) -> Result<&Game, NoGameInstnace> {
        self.instance.as_ref().map(|i| &i.game).ok_or(NoGameInstnace)
    }
    pub fn initial_aim_coord(&self, mouse_coord: Coord) -> Result<Coord, NoGameInstnace> {
        if let Some(instance) = self.instance.as_ref() {
            if self.last_aim_with_mouse {
                Ok(mouse_coord)
            } else {
                Ok(instance.game.player_coord())
            }
        } else {
            Err(NoGameInstnace)
        }
    }
}

pub struct NoGameInstnace;

pub struct AimEventRoutine<S: Storage> {
    s: PhantomData<S>,
    coord: Coord,
    duration: Duration,
    blink: Blink,
}

struct Blink {
    min: Rgb24,
    max: Rgb24,
    cycle_length: Duration,
}

impl Blink {
    fn intensity(&self, duration: Duration) -> u8 {
        let cycle_length_micros = self.cycle_length.as_micros();
        let duration_micros = duration.as_micros();
        let progress_through_cycle_micros = duration_micros % cycle_length_micros;
        let scaled_progress = (progress_through_cycle_micros * 512) / cycle_length_micros;
        if scaled_progress < 256 {
            scaled_progress as u8
        } else {
            (511 - scaled_progress) as u8
        }
    }
    fn col(&self, duration: Duration) -> Rgb24 {
        self.min.linear_interpolate(self.max, self.intensity(duration))
    }
}

impl<S: Storage> AimEventRoutine<S> {
    pub fn new(coord: Coord) -> Self {
        Self {
            s: PhantomData,
            coord,
            duration: Duration::from_millis(0),
            blink: Blink {
                min: Rgb24::new(63, 0, 0),
                max: Rgb24::new(187, 0, 0),
                cycle_length: Duration::from_millis(500),
            },
        }
    }
}

impl<S: Storage> EventRoutine for AimEventRoutine<S> {
    type Return = Option<Coord>;
    type Data = GameData<S>;
    type View = GameView;
    type Event = CommonEvent;

    fn handle<EP>(self, data: &mut Self::Data, view: &Self::View, event_or_peek: EP) -> Handled<Self::Return, Self>
    where
        EP: EventOrPeek<Event = Self::Event>,
    {
        enum Aim {
            Mouse { coord: Coord, press: bool },
            KeyboardDirection(Direction),
            KeyboardFinalise,
            Cancel,
            Ignore,
            Frame(Duration),
        }
        event_or_peek_with_handled(event_or_peek, self, |mut s, event| {
            data.last_aim_with_mouse = false;
            let aim = match event {
                CommonEvent::Input(input) => match input {
                    Input::Keyboard(keyboard_input) => {
                        if let Some(app_input) = data.controls.get(keyboard_input) {
                            match app_input {
                                AppInput::Aim => Aim::KeyboardFinalise,
                                AppInput::Move(direction) => Aim::KeyboardDirection(direction),
                            }
                        } else {
                            match keyboard_input {
                                keys::RETURN => Aim::KeyboardFinalise,
                                keys::ESCAPE => Aim::Cancel,
                                _ => Aim::Ignore,
                            }
                        }
                    }
                    Input::Mouse(mouse_input) => match mouse_input {
                        MouseInput::MouseMove { coord, .. } => Aim::Mouse { coord, press: false },
                        MouseInput::MousePress { coord, .. } => Aim::Mouse { coord, press: true },
                        _ => Aim::Ignore,
                    },
                },
                CommonEvent::Frame(since_last) => Aim::Frame(since_last),
            };
            match aim {
                Aim::KeyboardFinalise => Handled::Return(Some(s.coord)),
                Aim::KeyboardDirection(direction) => {
                    s.coord += direction.coord();
                    Handled::Continue(s)
                }
                Aim::Mouse { coord, press } => {
                    s.coord = view.absolute_coord_to_game_coord(coord);
                    if press {
                        data.last_aim_with_mouse = true;
                        Handled::Return(Some(s.coord))
                    } else {
                        Handled::Continue(s)
                    }
                }
                Aim::Cancel => Handled::Return(None),
                Aim::Ignore => Handled::Continue(s),
                Aim::Frame(since_last) => {
                    s.duration += since_last;
                    Handled::Continue(s)
                }
            }
        })
    }

    fn view<F, C>(&self, data: &Self::Data, view: &mut Self::View, context: ViewContext<C>, frame: &mut F)
    where
        F: Frame,
        C: ColModify,
    {
        if let Some(instance) = data.instance.as_ref() {
            view.view(&instance.game, context, frame);
            let player_coord = instance.game.player_coord();
            if self.coord != player_coord {
                for node in LineSegment::new(player_coord, self.coord).config_node_iter(LineConfig {
                    exclude_start: true,
                    exclude_end: true,
                }) {
                    if !node.coord.is_valid(instance.game.grid().size()) {
                        break;
                    }
                    frame.set_cell_relative(
                        node.coord,
                        1,
                        ViewCell::new().with_background(Rgb24::new(127, 0, 0)),
                        context,
                    );
                }
            }
            if self.coord.is_valid(instance.game.grid().size()) {
                let col = self.blink.col(self.duration);
                frame.set_cell_relative(self.coord, 1, ViewCell::new().with_background(col), context);
            }
        }
    }
}

pub struct GameEventRoutine<S: Storage> {
    s: PhantomData<S>,
    injected_inputs: Vec<GameInput>,
}

impl<S: Storage> GameEventRoutine<S> {
    pub fn new() -> Self {
        Self::new_injecting_inputs(Vec::new())
    }
    pub fn new_injecting_inputs(injected_inputs: Vec<GameInput>) -> Self {
        Self {
            s: PhantomData,
            injected_inputs,
        }
    }
}

pub enum GameReturn {
    Pause,
    Aim,
}

impl<S: Storage> EventRoutine for GameEventRoutine<S> {
    type Return = GameReturn;
    type Data = GameData<S>;
    type View = GameView;
    type Event = CommonEvent;

    fn handle<EP>(mut self, data: &mut Self::Data, _view: &Self::View, event_or_peek: EP) -> Handled<Self::Return, Self>
    where
        EP: EventOrPeek<Event = Self::Event>,
    {
        let storage_wrapper = &mut data.storage_wrapper;
        if let Some(instance) = data.instance.as_mut() {
            for game_input in self.injected_inputs.drain(..) {
                instance.game.handle_input(game_input);
            }
            let controls = &data.controls;
            event_or_peek_with_handled(event_or_peek, self, |s, event| match event {
                CommonEvent::Input(input) => {
                    match input {
                        Input::Keyboard(keyboard_input) => {
                            if keyboard_input == keys::ESCAPE {
                                return Handled::Return(GameReturn::Pause);
                            }
                            if !instance.game.has_animations() {
                                if let Some(app_input) = controls.get(keyboard_input) {
                                    match app_input {
                                        AppInput::Move(direction) => {
                                            instance.game.handle_input(GameInput::Move(direction))
                                        }
                                        AppInput::Aim => return Handled::Return(GameReturn::Aim),
                                    }
                                }
                            }
                        }
                        Input::Mouse(_) => (),
                    }
                    Handled::Continue(s)
                }
                CommonEvent::Frame(period) => {
                    instance.game.handle_tick(period);
                    storage_wrapper.autosave_tick(instance, period);
                    Handled::Continue(s)
                }
            })
        } else {
            storage_wrapper.clear_instance();
            Handled::Continue(self)
        }
    }

    fn view<F, C>(&self, data: &Self::Data, view: &mut Self::View, context: ViewContext<C>, frame: &mut F)
    where
        F: Frame,
        C: ColModify,
    {
        if let Some(instance) = data.instance.as_ref() {
            view.view(&instance.game, context, frame);
        }
    }
}
