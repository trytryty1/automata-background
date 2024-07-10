use std::iter;

use crate::game::world::*;
use crate::renderer::layeredwindow;
use trayicon::{Icon, MenuBuilder, MenuItem, TrayIcon, TrayIconBuilder};
use wgpu::{
    rwh::{HasWindowHandle, RawWindowHandle},
    util::DeviceExt,
};
use winapi::um::winuser::SetParent;
use winit::dpi::PhysicalSize;
use winit::{dpi::LogicalPosition, event_loop::EventLoopBuilder};
use winit::{
    event::*,
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowBuilder},
};

#[derive(Clone, Eq, PartialEq, Debug)]
enum UserEvents {
    RightClickTrayIcon,
    LeftClickTrayIcon,
    DoubleClickTrayIcon,
    Exit,
    Item1,
    Item2,
    Item3,
    Item4,
    DisabledItem1,
    CheckItem1,
    SubItem1,
    SubItem2,
    SubItem3,
}

use winapi::shared::windef::HWND;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Instance {
    position: [u32; 2],
    color: [f32; 3],
}
const PIXELS_PER_CELL: u32 = 6;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct SimulationParametersUniform {
    width: u32,
    height: u32,
}

impl Instance {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Instance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Uint32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[u32; 2]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

impl Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x3,
            }],
        }
    }
}

const VERTICES: &[Vertex] = &[
    // Just a quad with vertices ranging from 0 to 1
    Vertex {
        position: [0.0, 0.0, 0.0],
    },
    Vertex {
        position: [1.0, 0.0, 0.0],
    },
    Vertex {
        position: [1.0, 1.0, 0.0],
    },
    Vertex {
        position: [0.0, 1.0, 0.0],
    },
];

const INDICES: &[u16] = &[0, 1, 2, 2, 3, 0];
const PREY_COLOR: [f32; 3] = [0.0, 1.0, 0.0];
const PREDITOR_COLOR: [f32; 3] = [1.0, 0.0, 0.0];

struct State<'a> {
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    render_pipeline: wgpu::RenderPipeline,
    instances: Vec<Instance>,
    instance_buffer: wgpu::Buffer,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    window: &'a Window,

    trayicon: &'a TrayIcon<UserEvents>,

    simulation: Simulation,

    simulation_parameters_uniform: SimulationParametersUniform,
    simulation_parameters_buffer: wgpu::Buffer,
    simulation_parameters_uniform_bind_group: wgpu::BindGroup,
}

impl<'a> State<'a> {
    async fn new(window: &'a Window, trayicon: &'a TrayIcon<UserEvents>) -> State<'a> {
        let size = window.inner_size();

        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::GL,
            ..Default::default()
        });

        // # Safety
        //
        // The surface needs to live as long as the window that created it.
        // State owns the window so this should be safe.
        let surface = instance.create_surface(window).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    // WebGL doesn't support all of wgpu's features, so if
                    // we're building for the web we'll have to disable some.
                    required_limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    },
                },
                None, // Trace path
            )
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        // Shader code in this tutorial assumes an Srgb surface texture. Using a different
        // one will result all the colors comming out darker. If you want to support non
        // Srgb surfaces, you'll need to account for that when drawing to the frame.
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        let sim_scale = 1.0 / PIXELS_PER_CELL as f32;

        // Calculate aspect ratio
        println!("{}x{}", size.width, size.height);

        let simulation_parameters_uniform = SimulationParametersUniform {
            width: (size.width as f32 * sim_scale) as u32,
            height: (size.height as f32 * sim_scale) as u32,
        };

        let simulation_parameters_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Simulation Parameters"),
                contents: bytemuck::cast_slice(&[simulation_parameters_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let simulation_parameters_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Simulation Parameters Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let simulation_parameters_bind_group =
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Simulation Parameters Bind Group"),
                layout: &simulation_parameters_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: simulation_parameters_buffer.as_entire_binding(),
                }],
            });

        // create me a grid of instances
        let instances = (0..size.width * size.height)
            .map(|i| {
                let col = i % size.width;
                let row = i / size.width;

                // random color
                let r = rand::random::<f32>();
                let g = rand::random::<f32>();
                let b = rand::random::<f32>();
                Instance {
                    position: [col, row],
                    color: [r, g, b],
                }
            })
            .collect::<Vec<_>>();

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&simulation_parameters_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc(), Instance::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent::REPLACE,
                        alpha: wgpu::BlendComponent::OVER,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::POLYGON_MODE_LINE
                // or Features::POLYGON_MODE_POINT
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            // If the pipeline will be used with a multiview render pass, this
            // indicates how many array layers the attachments will have.
            multiview: None,
        });

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(&instances),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let num_indices = INDICES.len() as u32;

        let simulation = Simulation::new((
            simulation_parameters_uniform.width as usize,
            simulation_parameters_uniform.height as usize,
        ));

        Self {
            surface,
            device,
            instances,
            instance_buffer,
            queue,
            config,
            size,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            num_indices,
            window,

            trayicon,

            simulation_parameters_buffer,
            simulation_parameters_uniform_bind_group: simulation_parameters_bind_group,
            simulation_parameters_uniform,

            simulation,
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    #[allow(unused_variables)]
    fn input(&mut self, event: &WindowEvent) -> bool {
        false
    }

    fn update(&mut self) {
        self.simulation.update();

        // create simulation instances
        let mut instances = Vec::new();
        for (cell_idx, cell) in self.simulation.worlds[0].cells.iter().enumerate() {
            match cell.cell_type {
                CellType::Empty => {}
                CellType::Prey => {
                    let (x, y) = self.simulation.worlds[0].get_cell_x_y(cell_idx);
                    instances.push(Instance {
                        position: [x as u32, y as u32],
                        color: PREY_COLOR,
                    });
                }
                CellType::Preditor => {
                    let (x, y) = self.simulation.worlds[0].get_cell_x_y(cell_idx);
                    instances.push(Instance {
                        position: [x as u32, y as u32],
                        color: PREDITOR_COLOR,
                    });
                }
            }
        }
        self.instances = instances;

        // upload simulation instances
        self.queue.write_buffer(
            &self.instance_buffer,
            0,
            bytemuck::cast_slice(&self.instances),
        );
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 0.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.simulation_parameters_uniform_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.num_indices, 0, 0..self.instances.len() as _);
        }

        self.queue.submit(iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn run() {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Warn).expect("Could't initialize logger");
        } else {
            env_logger::init();
        }
    }

    let event_loop = EventLoopBuilder::<UserEvents>::with_user_event()
        .build()
        .unwrap();
    let proxy = event_loop.create_proxy();

    let icon = include_bytes!("../../desktop_automata_icon.ico");
    // let icon1 = Icon::from_buffer(icon, None, None).unwrap(); // (width, height)

    let trayicon = TrayIconBuilder::new()
        .sender(move |e: &UserEvents| {
            let _ = proxy.send_event(e.clone());
        })
        .icon_from_buffer(icon)
        .tooltip("Automata")
        .on_click(UserEvents::LeftClickTrayIcon)
        .on_right_click(UserEvents::RightClickTrayIcon)
        .on_double_click(UserEvents::DoubleClickTrayIcon)
        .build()
        .unwrap();

    let window = WindowBuilder::new()
        .with_title("Transparent Overlay Window")
        .with_decorations(false)
        .with_position(LogicalPosition::new(0.0, 0.0))
        .with_transparent(true)
        // Set window to be on second monitor
        .with_visible(false)
        .build(&event_loop)
        .unwrap();

    // get all available monitors
    let monitors = window.available_monitors();

    // get the width of all monitors
    let mut monitor_width = 0;

    // get the largest height of all monitors
    let mut monitor_height = 0;
    for monitor in monitors {
        monitor_width += monitor.size().width;
        if monitor.size().height > monitor_height {
            monitor_height = monitor.size().height;
        }
    }

    println!(
        "Monitor width: {}, height: {}",
        monitor_width, monitor_height
    );

    // set the size of the window
    let _ = window.request_inner_size(PhysicalSize::new(monitor_width, monitor_height));

    #[cfg(target_arch = "wasm32")]
    {
        // Winit prevents sizing with CSS, so we have to set
        // the size manually when on web.
        use winit::dpi::PhysicalSize;
        let _ = window.request_inner_size(PhysicalSize::new(450, 400));

        use winit::platform::web::WindowExtWebSys;
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| {
                let dst = doc.get_element_by_id("wasm-example")?;
                let canvas = web_sys::Element::from(window.canvas()?);
                dst.append_child(&canvas).ok()?;
                Some(())
            })
            .expect("Couldn't append canvas to document body.");
    }

    #[cfg(target_os = "windows")]
    {
        use winit::platform::windows::WindowExtWindows;
        let raw_window_handle = window.window_handle();
        match raw_window_handle {
            Ok(window_handle) => unsafe {
                match window_handle.as_raw() {
                    RawWindowHandle::Win32(handle) => {
                        let winit_hwnd = handle.hwnd.get() as HWND;

                        match layeredwindow::get_worker_window_handle() {
                            Ok(layered_window_handle) => {
                                let layered_hwnd = layered_window_handle as HWND;
                                println!("Layered window handle: {:?}", layered_window_handle);
                                // Set the winit window's parent to the layered window
                                SetParent(winit_hwnd, layered_hwnd);
                            }
                            Err(_) => {
                                println!("Failed to get worker window handle.");
                            }
                        }
                    }
                    _ => {}
                }
            },
            _ => {}
        }
        window.set_window_level(winit::window::WindowLevel::AlwaysOnBottom);
        window.set_ime_allowed(false);
        window.set_cursor_hittest(false).unwrap();

        window.set_enable(false);
        window.set_visible(true);
    }

    // State::new uses async code, so we're going to wait for it to finish
    let mut state = State::new(&window, &trayicon).await;
    let mut surface_configured = false;

    event_loop
        .run(move |event, control_flow| {
            match event {
                Event::UserEvent(event) => {
                    match event {
                        UserEvents::LeftClickTrayIcon => {
                            println!("Left click tray icon");
                        }
                        UserEvents::RightClickTrayIcon => {
                            // Exit the application
                            control_flow.exit();
                        }
                        UserEvents::DoubleClickTrayIcon => {
                            println!("Double click tray icon");
                        }
                        UserEvents::Exit => {
                            control_flow.exit();
                        }
                        _ => {}
                    }
                }
                Event::LoopExiting { .. } => {
                    layeredwindow::send_cleanup_message();
                }
                Event::WindowEvent {
                    ref event,
                    window_id,
                } if window_id == state.window().id() => {
                    if !state.input(event) {
                        match event {
                            WindowEvent::Occluded(_) => {
                                // unminimize the window
                                if state.window.is_minimized().unwrap_or(false) {
                                    state.window.set_minimized(false);
                                }
                            }
                            WindowEvent::CloseRequested
                            | WindowEvent::KeyboardInput {
                                event:
                                    KeyEvent {
                                        state: ElementState::Pressed,
                                        physical_key: PhysicalKey::Code(KeyCode::Escape),
                                        ..
                                    },
                                ..
                            } => control_flow.exit(),
                            WindowEvent::Resized(physical_size) => {
                                surface_configured = true;
                                state.resize(*physical_size);
                            }
                            WindowEvent::RedrawRequested => {
                                // This tells winit that we want another frame after this one
                                state.window().request_redraw();

                                if !surface_configured {
                                    return;
                                }

                                state.update();
                                match state.render() {
                                    Ok(_) => {}
                                    // Reconfigure the surface if it's lost or outdated
                                    Err(
                                        wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated,
                                    ) => state.resize(state.size),
                                    // The system is out of memory, we should probably quit
                                    Err(wgpu::SurfaceError::OutOfMemory) => {
                                        log::error!("OutOfMemory");
                                        control_flow.exit();
                                    }

                                    // This happens when the a frame takes too long to present
                                    Err(wgpu::SurfaceError::Timeout) => {
                                        log::warn!("Surface timeout")
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        })
        .unwrap();
}
