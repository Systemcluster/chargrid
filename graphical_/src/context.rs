use prototty_app::{App, ControlFlow};
use prototty_render::{Coord, Size};
use std::time::Duration;

pub struct FontBytes {
    pub normal: Vec<u8>,
    pub bold: Vec<u8>,
}

#[derive(Clone, Copy)]
pub struct Dimensions<T> {
    pub width: T,
    pub height: T,
}

pub type NumPixels = f64;

impl Dimensions<NumPixels> {
    fn to_logical_size(self) -> winit::dpi::LogicalSize {
        winit::dpi::LogicalSize::new(self.width, self.height)
    }
    fn to_f32(self) -> Dimensions<f32> {
        Dimensions {
            width: self.width as f32,
            height: self.height as f32,
        }
    }
}

pub enum WindowDimensions {
    Windowed(Dimensions<NumPixels>),
    Fullscreen,
}

pub mod defaults {
    use super::{Dimensions, NumPixels, WindowDimensions};
    pub const CELL_DIMENSIONS: Dimensions<NumPixels> = Dimensions {
        width: 16.,
        height: 16.,
    };
    pub const FONT_DIMENSIONS: Dimensions<NumPixels> = Dimensions {
        width: 16.,
        height: 16.,
    };
    pub const WINDOW_DIMENSINOS: WindowDimensions = WindowDimensions::Windowed(Dimensions {
        width: 640.,
        height: 480.,
    });
    pub const UNDERLINE_WIDTH: NumPixels = 2.;
    pub const UNDERLINE_BOTTOM_OFFSET: NumPixels = 2.;
}

pub struct ContextBuilder {
    font_bytes: FontBytes,
    window_builder: winit::window::WindowBuilder,
    cell_dimensions: Dimensions<NumPixels>,
    font_dimensions: Dimensions<NumPixels>,
    underline_width: NumPixels,
    underline_bottom_offset: NumPixels,
}

#[derive(Debug)]
pub enum ContextBuildError {
    FailedToBuildWindow(winit::error::OsError),
    FailedToRequestGraphicsAdapter,
}

impl ContextBuilder {
    pub fn new_with_font_bytes(font_bytes: FontBytes) -> Self {
        Self {
            font_bytes,
            window_builder: winit::window::WindowBuilder::new(),
            cell_dimensions: defaults::CELL_DIMENSIONS,
            font_dimensions: defaults::FONT_DIMENSIONS,
            underline_width: defaults::UNDERLINE_WIDTH,
            underline_bottom_offset: defaults::UNDERLINE_BOTTOM_OFFSET,
        }
    }
    pub fn with_title(self, title: &str) -> Self {
        Self {
            window_builder: self.window_builder.with_title(title),
            ..self
        }
    }
    pub fn with_cell_dimensions(self, cell_dimensions: Dimensions<NumPixels>) -> Self {
        Self {
            cell_dimensions,
            ..self
        }
    }
    pub fn with_font_dimensions(self, font_dimensions: Dimensions<NumPixels>) -> Self {
        Self {
            font_dimensions,
            ..self
        }
    }
    pub fn with_window_dimensions(self, window_dimensions: WindowDimensions) -> Self {
        let window_builder = match window_dimensions {
            WindowDimensions::Fullscreen => self.window_builder.with_fullscreen(None),
            WindowDimensions::Windowed(dimensions) => {
                let logical_size = dimensions.to_logical_size();
                self.window_builder
                    .with_inner_size(logical_size)
                    .with_min_inner_size(logical_size)
                    .with_max_inner_size(logical_size)
            }
        };
        Self { window_builder, ..self }
    }
    pub fn with_underline_width(self, underline_width: NumPixels) -> Self {
        Self {
            underline_width,
            ..self
        }
    }
    pub fn with_underline_bottom_offset(self, underline_bottom_offset: NumPixels) -> Self {
        Self {
            underline_bottom_offset,
            ..self
        }
    }
    pub fn build(self) -> Result<Context, ContextBuildError> {
        let event_loop = winit::event_loop::EventLoop::new();
        let window = self
            .window_builder
            .build(&event_loop)
            .map_err(ContextBuildError::FailedToBuildWindow)?;
        let grid_size = Size::new(10, 10);
        let wgpu_context = WgpuContext::new(&window, grid_size)?;
        Ok(Context {
            event_loop,
            window,
            wgpu_context,
            size: SizeContext {
                cell_dimensions: self.cell_dimensions,
            },
            input: Default::default(),
        })
    }
}

struct WgpuContext {
    device: wgpu::Device,
    swap_chain: wgpu::SwapChain,
    render_pipeline: wgpu::RenderPipeline,
    instance_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    queue: wgpu::Queue,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct BackgroundCellInstance {
    _colour: [f32; 3],
}

impl BackgroundCellInstance {
    const fn black() -> Self {
        Self { _colour: [0.; 3] }
    }
}

impl WgpuContext {
    fn u8_slice_to_u32_vec(bytes: &[u8]) -> Vec<u32> {
        let mut buffer = Vec::with_capacity(bytes.len() / 4);
        let mut chunks = bytes.chunks_exact(4);
        for chunk in &mut chunks {
            let mut array: [u8; 4] = Default::default();
            array.copy_from_slice(chunk);
            buffer.push(u32::from_le_bytes(array));
        }
        assert!(chunks.remainder().is_empty());
        buffer
    }
    fn new(window: &winit::window::Window, grid_size: Size) -> Result<Self, ContextBuildError> {
        use std::iter;
        use std::mem;
        let physical_size = window.inner_size().to_physical(window.hidpi_factor());
        let surface = wgpu::Surface::create(window);
        let adapter = wgpu::Adapter::request(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::Default,
            backends: wgpu::BackendBit::PRIMARY,
        })
        .ok_or(ContextBuildError::FailedToRequestGraphicsAdapter)?;
        let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
            extensions: wgpu::Extensions {
                anisotropic_filtering: false,
            },
            limits: wgpu::Limits::default(),
        });
        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: physical_size.width.round() as u32,
            height: physical_size.height.round() as u32,
            present_mode: wgpu::PresentMode::Vsync,
        };
        let swap_chain = device.create_swap_chain(&surface, &sc_desc);
        let vs_module = device.create_shader_module(&Self::u8_slice_to_u32_vec(include_bytes!("./shader.vert.spv")));
        let fs_module = device.create_shader_module(&Self::u8_slice_to_u32_vec(include_bytes!("./shader.frag.spv")));
        let num_instances = grid_size.count() as usize;
        let instance_buffer = device
            .create_buffer_mapped(num_instances, wgpu::BufferUsage::VERTEX)
            .fill_from_slice(
                &iter::repeat_with(BackgroundCellInstance::black)
                    .take(num_instances)
                    .collect::<Vec<_>>(),
            );
        let grid_width_uniform_buffer = device
            .create_buffer_mapped(1, wgpu::BufferUsage::UNIFORM)
            .fill_from_slice(&[grid_size.width()]);
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            bindings: &[wgpu::BindGroupLayoutBinding {
                binding: 0,
                visibility: wgpu::ShaderStage::VERTEX,
                ty: wgpu::BindingType::UniformBuffer { dynamic: false },
            }],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            bindings: &[wgpu::Binding {
                binding: 0,
                resource: wgpu::BindingResource::Buffer {
                    buffer: &grid_width_uniform_buffer,
                    range: 0..1,
                },
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[&bind_group_layout],
        });
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            layout: &pipeline_layout,
            vertex_stage: wgpu::ProgrammableStageDescriptor {
                module: &vs_module,
                entry_point: "main",
            },
            fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                module: &fs_module,
                entry_point: "main",
            }),
            rasterization_state: Some(wgpu::RasterizationStateDescriptor {
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: wgpu::CullMode::None,
                depth_bias: 0,
                depth_bias_slope_scale: 0.,
                depth_bias_clamp: 0.,
            }),
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            color_states: &[wgpu::ColorStateDescriptor {
                format: wgpu::TextureFormat::Bgra8UnormSrgb,
                color_blend: wgpu::BlendDescriptor::REPLACE,
                alpha_blend: wgpu::BlendDescriptor::REPLACE,
                write_mask: wgpu::ColorWrite::ALL,
            }],
            depth_stencil_state: None,
            index_format: wgpu::IndexFormat::Uint16,
            vertex_buffers: &[wgpu::VertexBufferDescriptor {
                stride: mem::size_of::<BackgroundCellInstance>() as wgpu::BufferAddress,
                step_mode: wgpu::InputStepMode::Instance,
                attributes: &[wgpu::VertexAttributeDescriptor {
                    format: wgpu::VertexFormat::Float3,
                    offset: 0,
                    shader_location: 0,
                }],
            }],
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        });
        Ok(Self {
            device,
            swap_chain,
            render_pipeline,
            instance_buffer,
            bind_group,
            queue,
        })
    }
}

struct InputContext {
    closing: bool,
    last_mouse_coord: Coord,
    last_mouse_button: Option<prototty_input::MouseButton>,
}

impl Default for InputContext {
    fn default() -> Self {
        Self {
            closing: false,
            last_mouse_coord: Coord::new(0, 0),
            last_mouse_button: None,
        }
    }
}

struct SizeContext {
    cell_dimensions: Dimensions<NumPixels>,
}

pub struct Context {
    event_loop: winit::event_loop::EventLoop<()>,
    window: winit::window::Window,
    wgpu_context: WgpuContext,
    size: SizeContext,
    input: InputContext,
}

impl Context {
    pub fn run_app<A>(self, app: A) -> !
    where
        A: App,
    {
        self.event_loop.run(|event, _, control_flow| match event {
            winit::event::Event::WindowEvent { event, .. } => match event {
                winit::event::WindowEvent::CloseRequested => *control_flow = winit::event_loop::ControlFlow::Exit,
                _ => (),
            },
            _ => (),
        })
    }
}