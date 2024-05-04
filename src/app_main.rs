// Copyright 2022 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use std::{
    num::NonZeroUsize,
    sync::{Arc, Mutex},
};

use accesskit::TreeUpdate;
use accesskit_winit::{Adapter, Event as AccessKitEvent, WindowEvent as AccessKitWindowEvent};
use vello::{
    kurbo::{Affine, Point, Size, Vec2},
    peniko::Color,
    util::{RenderContext, RenderSurface},
    AaSupport, RenderParams, Renderer, RendererOptions, Scene,
};
use wgpu::PresentMode;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, Modifiers, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

use crate::{
    app::App,
    view::View,
    widget::{Event, PointerCrusher, ScrollDelta},
};

// This is a bit of a hack just to get a window launched. The real version
// would deal with multiple windows and have other ways to configure things.
pub struct AppLauncher<T, V: View<T>> {
    title: String,
    app: App<T, V>,
}

// The logic of this struct is mostly parallel to DruidHandler in win_handler.rs.
struct MainState<'a, T, V: View<T>> {
    window: Arc<Window>,
    adapter: Arc<Mutex<Adapter>>,
    app: App<T, V>,
    render_cx: RenderContext,
    surface: RenderSurface<'a>,
    renderer: Option<Renderer>,
    scene: Scene,
    counter: u64,
    main_pointer: PointerCrusher,
}

impl<T: Send + 'static, V: View<T> + 'static> AppLauncher<T, V> {
    pub fn new(app: App<T, V>) -> Self {
        AppLauncher {
            title: "Xilem app".into(),
            app,
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn run(self) {
        let event_loop = EventLoop::with_user_event().build().unwrap();
        event_loop.set_control_flow(ControlFlow::Wait);
        let event_loop_proxy = event_loop.create_proxy();
        let _guard = self.app.rt.enter();
        #[allow(deprecated)]
        let window = event_loop
            .create_window(
                Window::default_attributes()
                    .with_inner_size(winit::dpi::LogicalSize {
                        width: 1024.,
                        height: 768.,
                    })
                    .with_title(self.title)
                    .with_visible(false),
            )
            .unwrap();

        let adapter = Arc::new(Mutex::new(Adapter::with_event_loop_proxy(
            &window,
            event_loop_proxy.clone(),
        )));
        window.set_visible(true);
        let mut main_state = MainState::new(self.app, Arc::new(window), adapter);
        let _ = event_loop.run_app(&mut main_state);
    }
}

impl<T: Send + 'static, V: View<T> + 'static> ApplicationHandler<AccessKitEvent>
    for MainState<'_, T, V>
{
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {}
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => self.paint(),
            WindowEvent::Resized(winit::dpi::PhysicalSize { width, height }) => {
                self.size(Size {
                    width: width.into(),
                    height: height.into(),
                });
            }
            WindowEvent::ModifiersChanged(modifiers) => self.mods(modifiers),
            WindowEvent::CursorMoved {
                position: winit::dpi::PhysicalPosition { x, y },
                ..
            } => self.pointer_move(Point { x, y }),
            WindowEvent::CursorLeft { .. } => self.pointer_leave(),
            WindowEvent::MouseInput { state, button, .. } => match state {
                ElementState::Pressed => self.pointer_down(button),
                ElementState::Released => self.pointer_up(button),
            },
            WindowEvent::MouseWheel { delta, .. } => self.pointer_wheel(delta),
            _ => (),
        }
    }
    fn user_event(&mut self, _: &ActiveEventLoop, user_event: AccessKitEvent) {
        match user_event.window_event {
            AccessKitWindowEvent::InitialTreeRequested => {
                let tu = self.accesskit_tree();
                self.adapter.lock().unwrap().update_if_active(|| tu);
            }
            AccessKitWindowEvent::ActionRequested(req) => self.accesskit_action(req),
            AccessKitWindowEvent::AccessibilityDeactivated => (),
        }
    }
}

impl<'a, T: Send + 'static, V: View<T> + 'static> MainState<'a, T, V> {
    fn new(app: App<T, V>, window: Arc<Window>, adapter: Arc<Mutex<Adapter>>) -> Self {
        let mut render_cx = RenderContext::new().unwrap();
        let size = window.inner_size();
        let surface = tokio::runtime::Handle::current()
            .block_on(render_cx.create_surface(
                window.clone(),
                size.width,
                size.height,
                PresentMode::AutoVsync,
            ))
            .unwrap();
        MainState {
            window: window.clone(),
            adapter: adapter.clone(),
            app,
            render_cx,
            surface,
            renderer: None,
            scene: Scene::default(),
            counter: 0,
            main_pointer: PointerCrusher::new(),
        }
    }

    fn accesskit_tree(&mut self) -> TreeUpdate {
        self.app.accesskit_connected = true;
        self.app.paint();
        self.app.accessibility(self.window.clone())
    }

    fn accesskit_action(&mut self, request: accesskit::ActionRequest) {
        self.app
            .window_event(Event::TargetedAccessibilityAction(request));
        self.app.accessibility(self.window.clone());
        self.window.request_redraw();
    }

    fn size(&mut self, size: Size) {
        self.app.size(size * 1.0 / self.window.scale_factor());
    }

    fn mods(&mut self, mods: Modifiers) {
        self.main_pointer.mods(mods);
        self.window.request_redraw();
    }

    fn pointer_move(&mut self, pos: Point) {
        let scale_coefficient = 1.0 / self.window.scale_factor();
        self.app
            .window_event(Event::MouseMove(self.main_pointer.moved(Point {
                x: pos.x * scale_coefficient,
                y: pos.y * scale_coefficient,
            })));
        self.window.request_redraw();
    }

    fn pointer_down(&mut self, button: MouseButton) {
        self.app
            .window_event(Event::MouseDown(self.main_pointer.pressed(button)));
        self.window.request_redraw();
    }

    fn pointer_up(&mut self, button: MouseButton) {
        self.app
            .window_event(Event::MouseUp(self.main_pointer.released(button)));
        self.window.request_redraw();
    }

    fn pointer_leave(&mut self) {
        self.app.window_event(Event::MouseLeft());
        self.window.request_redraw();
    }

    fn pointer_wheel(&mut self, delta: MouseScrollDelta) {
        self.app
            .window_event(Event::MouseWheel(self.main_pointer.wheel(match delta {
                MouseScrollDelta::LineDelta(x, y) => {
                    ScrollDelta::Lines(x.trunc() as isize, y.trunc() as isize)
                }
                MouseScrollDelta::PixelDelta(position) => {
                    let logical_pos = position.to_logical(self.window.scale_factor());
                    ScrollDelta::Precise(Vec2::new(logical_pos.x, logical_pos.y))
                }
            })));
        self.window.request_redraw();
    }

    fn paint(&mut self) {
        self.app.paint();
        self.render();
    }

    fn render(&mut self) {
        let fragment = self.app.fragment();
        let scale = self.window.scale_factor();
        let size = self.window.inner_size();
        let width = size.width;
        let height = size.height;

        if self.surface.config.width != width || self.surface.config.height != height {
            self.render_cx
                .resize_surface(&mut self.surface, width, height);
        }
        let transform = if scale != 1.0 {
            Some(Affine::scale(scale))
        } else {
            None
        };
        self.scene.reset();
        self.scene.append(fragment, transform);
        self.counter += 1;

        let surface_texture = self
            .surface
            .surface
            .get_current_texture()
            .expect("failed to acquire next swapchain texture");
        let dev_id = self.surface.dev_id;
        let device = &self.render_cx.devices[dev_id].device;
        let queue = &self.render_cx.devices[dev_id].queue;
        let renderer_options = RendererOptions {
            surface_format: Some(self.surface.format),
            use_cpu: false,
            antialiasing_support: AaSupport {
                area: true,
                msaa8: false,
                msaa16: false,
            },
            num_init_threads: NonZeroUsize::new(1),
        };
        let render_params = RenderParams {
            base_color: Color::BLACK,
            width,
            height,
            antialiasing_method: vello::AaConfig::Area,
        };
        self.renderer
            .get_or_insert_with(|| Renderer::new(device, renderer_options).unwrap())
            .render_to_surface(device, queue, &self.scene, &surface_texture, &render_params)
            .expect("failed to render to surface");
        surface_texture.present();
        device.poll(wgpu::Maintain::Wait);
    }
}
