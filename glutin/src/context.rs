use gfx;
use glutin;
use gfx_device_gl;
use gfx_window_glutin;
use gfx_glyph;

use gfx::Device;
use glutin::{GlContext, Event, WindowEvent, VirtualKeyCode, ElementState};

use prototty_grid::*;
use prototty::*;
use cell::*;
use formats::*;
use background::*;

type Resources = gfx_device_gl::Resources;

const UNDERLINE_WIDTH_RATIO: u32 = 20;
const UNDERLINE_POSITION_RATIO: u32 = 20;
const FONT_SCALE: gfx_glyph::Scale = gfx_glyph::Scale { x: 16.0, y: 16.0 };

const FONT_ID: gfx_glyph::FontId = gfx_glyph::FontId(0);
const BOLD_FONT_ID: gfx_glyph::FontId = gfx_glyph::FontId(1);

pub const MAX_WIDTH_IN_CELLS: u32 = 256;
pub const MAX_HEIGHT_IN_CELLS: u32 = 256;

#[derive(Debug)]
pub enum Error {
    GfxGlyph(String),
    GlutinContextError(glutin::ContextError),
    WindowNoLongerExists,
    FailedToMeasureFont,
}
pub type Result<T> = ::std::result::Result<T, Error>;

pub struct ContextBuilder<'a> {
    font: &'a [u8],
    bold_font: Option<&'a [u8]>,
    font_scale: gfx_glyph::Scale,
    bold_font_scale: gfx_glyph::Scale,
    window_builder: glutin::WindowBuilder,
    context_builder: glutin::ContextBuilder<'a>,
    cell_dimensions: Option<Size>,
    locked_size: bool,
    underline_width: Option<u32>,
    underline_position: Option<u32>,
}

impl<'a> ContextBuilder<'a> {
    pub fn new_with_font(font: &'a [u8]) -> Self {
        Self {
            font,
            bold_font: None,
            font_scale: FONT_SCALE,
            bold_font_scale: FONT_SCALE,
            window_builder: glutin::WindowBuilder::new(),
            context_builder: glutin::ContextBuilder::new(),
            cell_dimensions: None,
            locked_size: false,
            underline_width: None,
            underline_position: None,
        }
    }

    pub fn with_cell_dimensions(self, width: u32, height: u32) -> Self {
        Self {
            cell_dimensions: Some(Size::new(width, height)),
            ..self
        }
    }

    pub fn with_window_dimensions(self, width: u32, height: u32) -> Self {
        Self {
            window_builder: self.window_builder.with_dimensions(width, height),
            ..self
        }
    }

    pub fn with_vsync(self, vsync: bool) -> Self {
        Self {
            context_builder: self.context_builder.with_vsync(vsync),
            ..self
        }
    }

    pub fn with_locked_size(self, locked_size: bool) -> Self {
        Self {
            locked_size,
            ..self
        }
    }

    pub fn with_underline_width(self, underline_width: u32) -> Self {
        Self {
            underline_width: Some(underline_width),
            ..self
        }
    }

    pub fn with_underline_position(self, underline_position: u32) -> Self {
        Self {
            underline_position: Some(underline_position),
            ..self
        }
    }

    pub fn with_bold_font(self, bold_font: &'a [u8]) -> Self {
        Self {
            bold_font: Some(bold_font),
            ..self
        }
    }

    pub fn with_font_scale(self, x: f32, y: f32) -> Self {
        Self {
            font_scale: gfx_glyph::Scale { x, y },
            ..self
        }
    }

    pub fn with_bold_font_scale(self, x: f32, y: f32) -> Self {
        Self {
            bold_font_scale: gfx_glyph::Scale { x, y },
            ..self
        }
    }

    pub fn build(self) -> Result<Context<'a>> {

        let events_loop = glutin::EventsLoop::new();

        let (window, device, mut factory, rtv, dsv) =
            gfx_window_glutin::init::<ColourFormat, DepthFormat>(self.window_builder, self.context_builder, &events_loop);

        let mut encoder: gfx::Encoder<Resources, gfx_device_gl::CommandBuffer> =
            factory.create_command_buffer().into();

        let mut glyph_brush_builder = gfx_glyph::GlyphBrushBuilder::using_font_bytes(self.font);
        let bold_font_id = glyph_brush_builder.add_font_bytes(self.bold_font.unwrap_or(self.font));
        assert_eq!(bold_font_id, BOLD_FONT_ID);

        let mut glyph_brush = glyph_brush_builder.build(factory.clone());

        let (window_width, window_height) = window.get_outer_size().ok_or(Error::WindowNoLongerExists)?;

        let (cell_width, cell_height) = if let Some(cell_dimensions) = self.cell_dimensions {
            (cell_dimensions.x(), cell_dimensions.y())
        } else {
            let section = gfx_glyph::Section {
                text: "@",
                scale: self.font_scale,
                font_id: FONT_ID,
                ..Default::default()
            };
            let rect = glyph_brush.pixel_bounds(section).ok_or(Error::FailedToMeasureFont)?;
            (rect.width() as u32, rect.height() as u32)
        };

        let width_in_cells = ::std::cmp::min(window_width / cell_width, MAX_WIDTH_IN_CELLS);
        let height_in_cells = ::std::cmp::min(window_height / cell_height, MAX_HEIGHT_IN_CELLS);

        let window_size_in_cells = Size::new(width_in_cells, height_in_cells);

        let grid = Grid::new(window_size_in_cells);
        let char_buf = String::with_capacity(1);

        let underline_width = self.underline_width.unwrap_or_else(|| {
            cell_height / UNDERLINE_WIDTH_RATIO
        });

        let underline_position = self.underline_position.unwrap_or_else(|| {
            cell_height - (cell_height / UNDERLINE_POSITION_RATIO)
        });

        let background_renderer = BackgroundRenderer::new(
            window_width,
            window_height,
            cell_width,
            cell_height,
            window_size_in_cells,
            rtv.clone(),
            &mut factory,
            &mut encoder);

        Ok(Context {
            window,
            device,
            encoder,
            factory,
            rtv,
            dsv,
            events_loop,
            glyph_brush,
            grid,
            char_buf,
            cell_width,
            cell_height,
            locked_size: self.locked_size,
            underline_width,
            underline_position,
            background_renderer,
            font_scale: self.font_scale,
            bold_font_scale: self.bold_font_scale,
        })
    }
}

pub struct Context<'a> {
    window: glutin::GlWindow,
    device: gfx_device_gl::Device,
    encoder: gfx::Encoder<Resources, gfx_device_gl::CommandBuffer>,
    factory: gfx_device_gl::Factory,
    rtv: gfx::handle::RenderTargetView<Resources, ColourFormat>,
    dsv: gfx::handle::DepthStencilView<Resources, DepthFormat>,
    events_loop: glutin::EventsLoop,
    glyph_brush: gfx_glyph::GlyphBrush<'a, Resources, gfx_device_gl::Factory>,
    grid: Grid<Cell>,
    char_buf: String,
    cell_width: u32,
    cell_height: u32,
    locked_size: bool,
    underline_width: u32,
    underline_position: u32,
    background_renderer: BackgroundRenderer<Resources>,
    font_scale: gfx_glyph::Scale,
    bold_font_scale: gfx_glyph::Scale,
}

impl<'a> Context<'a> {
    pub fn poll_input<F: FnMut(Input)>(&mut self, mut callback: F) {
        self.events_loop.poll_events(|event| {
            let input = match event {
                Event::WindowEvent { event, .. } => {
                    match event {
                        WindowEvent::Closed => inputs::ETX,
                        WindowEvent::KeyboardInput { input, .. } => {
                            if let ElementState::Pressed = input.state {
                                if let Some(virtual_keycode) = input.virtual_keycode {
                                    match virtual_keycode {
                                        VirtualKeyCode::Left => Input::Left,
                                        VirtualKeyCode::Right => Input::Right,
                                        VirtualKeyCode::Up => Input::Up,
                                        VirtualKeyCode::Down => Input::Down,
                                        VirtualKeyCode::Escape => inputs::ESCAPE,
                                        VirtualKeyCode::Return => inputs::RETURN,
                                        _ => return,
                                    }
                                } else {
                                    return;
                                }
                            } else {
                                return;
                            }
                        }
                        _ => return,
                    }
                }
                _ => return,
            };

            callback(input);
        });
    }
}

impl<'a> Renderer for Context<'a> {
    type Error = Error;
    fn render<V: View<T>, T>(&mut self, view: &V, data: &T) -> Result<()> {

        self.grid.clear();
        view.view(data, Coord::new(0, 0), 0, &mut self.grid);

        {
            let mut output_cells = self.background_renderer.map_cells(&mut self.factory);

            for ((coord, cell), output_cell) in izip!(self.grid.enumerate(), output_cells.iter_mut()) {
                self.char_buf.clear();
                self.char_buf.push(cell.character);

                let (font_id, scale) = if cell.bold {
                    (BOLD_FONT_ID, self.bold_font_scale)
                } else {
                    (FONT_ID, self.font_scale)
                };

                let section = gfx_glyph::Section {
                    text: self.char_buf.as_str(),
                    screen_position: ((coord.x * self.cell_width as i32) as f32, (coord.y * self.cell_height as i32) as f32),
                    scale,
                    font_id,
                    color: cell.foreground_colour.0,
                    ..Default::default()
                };

                self.glyph_brush.queue(section);

                output_cell.foreground_colour = cell.foreground_colour.0;
                output_cell.background_colour = cell.background_colour.0;
                output_cell.underline = cell.underline as u32;
            }
        }

        self.encoder.clear(&self.rtv, [0.0,0.0,0.0,1.0]);

        self.background_renderer.draw(&mut self.encoder);

        self.glyph_brush.draw_queued(&mut self.encoder, &self.rtv, &self.dsv).map_err(Error::GfxGlyph)?;

        self.encoder.flush(&mut self.device);
        self.window.swap_buffers().map_err(Error::GlutinContextError)?;
        self.device.cleanup();

        Ok(())
    }
}