use iced_winit::winit::dpi::PhysicalSize;
use iced_winit::winit::keyboard::KeyCode;
use log::info;
use render_playground::controls::Controls;

use iced_wgpu::graphics::Viewport;
use iced_wgpu::{wgpu, Engine, Renderer};
use iced_winit::conversion;
use iced_winit::core::mouse;
use iced_winit::core::renderer;
use iced_winit::core::{Color, Font, Pixels, Size, Theme};
use iced_winit::futures;
use iced_winit::runtime::program;
use iced_winit::runtime::Debug;
use iced_winit::winit;
use iced_winit::Clipboard;

use render_playground::scene::{Scene, UnitScene};
use winit::{
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoop},
    keyboard::ModifiersState,
};

use std::sync::Arc;
use std::time::Instant;

//TODO: toggle between polling and waiting
//const POLL_SLEEP_TIME: time::Duration = time::Duration::from_micros(1_000_000 / 60);

pub fn main() -> Result<(), winit::error::EventLoopError> {
    #[cfg(target_arch = "wasm32")]
    {
        console_log::init().expect("Initialize logger");
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    }

    #[cfg(not(target_arch = "wasm32"))]
    tracing_subscriber::fmt::init();

    // Initialize winit
    let event_loop = EventLoop::new()?;

    #[allow(clippy::large_enum_variant)]
    enum Runner {
        Loading,
        Ready {
            window: Arc<winit::window::Window>,
            device: wgpu::Device,
            queue: wgpu::Queue,
            surface: wgpu::Surface<'static>,
            format: wgpu::TextureFormat,
            engine: Engine,
            renderer: Renderer,
            scene: Scene,
            sample_count: u32,
            config: wgpu::SurfaceConfiguration,

            state: program::State<Controls>,
            cursor_position: Option<winit::dpi::PhysicalPosition<f64>>,
            clipboard: Clipboard,
            viewport: Viewport,
            modifiers: ModifiersState,
            resized: bool,
            debug: Debug,
            _begin: Instant,
        },
    }

    impl winit::application::ApplicationHandler for Runner {
        fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
            if let Self::Loading = self {
                let window = Arc::new(
                    event_loop
                        .create_window(
                            winit::window::WindowAttributes::default()
                                .with_inner_size(PhysicalSize::new(800, 1200)),
                        )
                        .expect("Create window"),
                );

                let physical_size = window.inner_size();
                let viewport = Viewport::with_physical_size(
                    Size::new(physical_size.width, physical_size.height),
                    window.scale_factor(),
                );
                let clipboard = Clipboard::connect(window.clone());

                let backend = wgpu::util::backend_bits_from_env().unwrap_or_default();

                let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
                    backends: backend,
                    ..Default::default()
                });
                let surface = instance
                    .create_surface(window.clone())
                    .expect("Create window surface");

                let (format, adapter, device, queue) =
                    futures::futures::executor::block_on(async {
                        let adapter = wgpu::util::initialize_adapter_from_env_or_default(
                            &instance,
                            Some(&surface),
                        )
                        .await
                        .expect("Create adapter");

                        let adapter_features = adapter.features();

                        let capabilities = surface.get_capabilities(&adapter);

                        let (device, queue) = adapter
                            .request_device(
                                &wgpu::DeviceDescriptor {
                                    label: None,
                                    required_features: adapter_features
                                        | wgpu::Features::default()
                                        | wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
                                        | wgpu::Features::POLYGON_MODE_LINE, // wireframe
                                    required_limits: wgpu::Limits::default(),
                                },
                                None,
                            )
                            .await
                            .expect("Request device");

                        (
                            capabilities
                                .formats
                                .iter()
                                .copied()
                                .find(wgpu::TextureFormat::is_srgb)
                                .or_else(|| capabilities.formats.first().copied())
                                .expect("Get preferred format"),
                            adapter,
                            device,
                            queue,
                        )
                    });

                let mut config = wgpu::SurfaceConfiguration {
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    format,
                    width: physical_size.width,
                    height: physical_size.height,
                    present_mode: wgpu::PresentMode::Mailbox,
                    alpha_mode: wgpu::CompositeAlphaMode::Auto,
                    view_formats: vec![format],
                    desired_maximum_frame_latency: 2,
                };

                let format = config.format.remove_srgb_suffix();
                config.format = format;
                config.view_formats.push(format);
                surface.configure(&device, &config);

                let sample_flags = adapter.get_texture_format_features(format).flags;

                let max_sample_count = {
                    //if sample_flags.contains(wgpu::TextureFormatFeatureFlags::MULTISAMPLE_X16) {
                    //    16
                    //} else if sample_flags.contains(wgpu::TextureFormatFeatureFlags::MULTISAMPLE_X8)
                    //{
                    //    8
                    //} else
                    if sample_flags.contains(wgpu::TextureFormatFeatureFlags::MULTISAMPLE_X4) {
                        4
                    } else if sample_flags.contains(wgpu::TextureFormatFeatureFlags::MULTISAMPLE_X2)
                    {
                        2
                    } else {
                        1
                    }
                };

                let sample_count = max_sample_count;
                info!("multisampling count: {}", sample_count);

                // Initialize scene and GUI controls
                //
                //let scene = futures::futures::executor::block_on(async {
                let scene = Scene::new(
                    UnitScene::TerrainScene,
                    &device,
                    &config,
                    &queue,
                    sample_count,
                );

                // ObjScene::init(&device, &config, &queue, sample_count));
                //});
                let controls = Controls::new();

                // Initialize iced
                let mut debug = Debug::new();
                let engine = Engine::new(&adapter, &device, &queue, format, None);
                let mut renderer =
                    Renderer::new(&device, &engine, Font::default(), Pixels::from(16));

                let state = program::State::new(
                    controls,
                    viewport.logical_size(),
                    &mut renderer,
                    &mut debug,
                );

                // You should change this if you want to render continuously
                //event_loop.set_control_flow(ControlFlow::Poll);
                event_loop.set_control_flow(ControlFlow::Wait);

                *self = Self::Ready {
                    window,
                    device,
                    queue,
                    surface,
                    format,
                    engine,
                    renderer,
                    scene,
                    config,
                    sample_count,
                    state,
                    cursor_position: None,
                    modifiers: ModifiersState::default(),
                    clipboard,
                    viewport,
                    resized: false,
                    debug,
                    _begin: Instant::now(),
                };
            }
        }

        fn window_event(
            &mut self,
            event_loop: &winit::event_loop::ActiveEventLoop,
            _window_id: winit::window::WindowId,
            event: WindowEvent,
        ) {
            let Self::Ready {
                window,
                device,
                queue,
                surface,
                format,
                engine,
                renderer,
                sample_count,
                config,
                scene,
                state,
                viewport,
                cursor_position,
                modifiers,
                clipboard,
                resized,
                debug,
                _begin,
            } = self
            else {
                return;
            };

            match event {
                WindowEvent::RedrawRequested => {
                    if *resized {
                        let size = window.inner_size();

                        *viewport = Viewport::with_physical_size(
                            Size::new(size.width, size.height),
                            window.scale_factor(),
                        );

                        let mut config = wgpu::SurfaceConfiguration {
                            format: *format,
                            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                            width: size.width,
                            height: size.height,
                            present_mode: wgpu::PresentMode::Mailbox,
                            alpha_mode: wgpu::CompositeAlphaMode::Auto,
                            view_formats: vec![*format],
                            desired_maximum_frame_latency: 2,
                        };
                        let format = config.format.remove_srgb_suffix();
                        config.format = format;
                        config.view_formats.push(format);

                        surface.configure(device, &config);

                        scene.resize(size, device, &config);

                        *resized = false;
                    }

                    match surface.get_current_texture() {
                        Ok(frame) => {
                            debug.render_started();
                            let mut encoder =
                                device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                                    label: None,
                                });

                            let program = state.program();

                            let view = frame
                                .texture
                                .create_view(&wgpu::TextureViewDescriptor::default());

                            {
                                let aspect = window.inner_size().width as f32
                                    / window.inner_size().height as f32;

                                scene.render(
                                    program,
                                    &view,
                                    program.camera,
                                    program.zoom,
                                    program.show_wireframe,
                                    aspect,
                                    device,
                                    queue,
                                );
                            }

                            debug.render_finished();
                            //debug.render_started();
                            // And then iced on top
                            debug.startup_started();
                            renderer.present(
                                engine,
                                device,
                                queue,
                                &mut encoder,
                                None,
                                frame.texture.format(),
                                &view,
                                viewport,
                                &debug.overlay(),
                            );

                            debug.startup_finished();
                            //debug.render_finished();
                            //debug.render_finished();

                            // Then we submit the work
                            engine.submit(queue, encoder);
                            frame.present();

                            // Update the mouse cursor
                            window.set_cursor(iced_winit::conversion::mouse_interaction(
                                state.mouse_interaction(),
                            ));
                        }
                        Err(error) => match error {
                            wgpu::SurfaceError::OutOfMemory => {
                                panic!(
                                    "Swapchain error: {error}. \
                                Rendering cannot continue."
                                )
                            }
                            _ => {
                                // Try rendering again next frame.
                                window.request_redraw();
                            }
                        },
                    }
                }
                WindowEvent::CursorMoved { position, .. } => {
                    *cursor_position = Some(position);
                }
                WindowEvent::ModifiersChanged(new_modifiers) => {
                    *modifiers = new_modifiers.state();
                }
                WindowEvent::Resized(_) => {
                    *resized = true;
                }
                WindowEvent::CloseRequested => {
                    event_loop.exit();
                }
                WindowEvent::KeyboardInput {
                    event:
                        winit::event::KeyEvent {
                            physical_key: p_key,

                            state: winit::event::ElementState::Pressed,
                            ..
                        },
                    ..
                } => match p_key {
                    winit::keyboard::PhysicalKey::Code(KeyCode::F12) => debug.toggle(),
                    winit::keyboard::PhysicalKey::Code(KeyCode::Digit1) => {
                        *scene =
                            Scene::new(UnitScene::ObjScene, device, config, queue, *sample_count)
                    }
                    winit::keyboard::PhysicalKey::Code(KeyCode::Digit2) => {
                        *scene = Scene::new(
                            UnitScene::TerrainScene,
                            device,
                            config,
                            queue,
                            *sample_count,
                        )
                    }
                    _ => {}
                },
                _ => {}
            }

            // Map window event to iced event
            if let Some(event) =
                iced_winit::conversion::window_event(event, window.scale_factor(), *modifiers)
            {
                state.queue_event(event);
            }

            // If there are events pending
            if !state.is_queue_empty() {
                // We update iced
                //debug.update_started();
                let _ = state.update(
                    viewport.logical_size(),
                    cursor_position
                        .map(|p| conversion::cursor_position(p, viewport.scale_factor()))
                        .map(mouse::Cursor::Available)
                        .unwrap_or(mouse::Cursor::Unavailable),
                    renderer,
                    &Theme::GruvboxLight,
                    &renderer::Style {
                        text_color: Color::BLACK,
                    },
                    clipboard,
                    debug,
                );
                //debug.update_finished();
                // and request a redraw
                //
                //window.request_redraw();
                window.request_redraw();
            }
        }
        //fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        //    //thread::sleep(POLL_SLEEP_TIME);
        //    trace!("render {:?}", Instant::now());
        //}
    }

    let mut runner = Runner::Loading;
    event_loop.run_app(&mut runner)
}
