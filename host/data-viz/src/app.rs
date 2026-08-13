mod camera;
mod event;
mod instance;
mod light;
mod model;
mod resources;
mod state;
mod texture;

use std::sync::Arc;

use cgmath::Quaternion;
use tracing::info;
use winit::{
    application::ApplicationHandler,
    event::{KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoopProxy},
    keyboard::PhysicalKey,
    window::Window,
};

use crate::app::{
    event::{AppEvent, EventSender},
    state::State,
};

pub struct App {
    state: Option<State>,
    rt: tokio::runtime::Handle,
    proxy: EventLoopProxy<AppEvent>,
    rx: Option<tokio::sync::watch::Receiver<Quaternion<f32>>>,
}

impl App {
    pub const fn new(
        rt: tokio::runtime::Handle,
        proxy: EventLoopProxy<AppEvent>,
        rx: tokio::sync::watch::Receiver<Quaternion<f32>>,
    ) -> Self {
        Self {
            state: None,
            rt,
            proxy,
            rx: Some(rx),
        }
    }
}

impl ApplicationHandler<AppEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        info!("Starting window");

        let window_attributes = Window::default_attributes().with_title("SkiBoot Viz");

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        let rt = self.rt.clone();
        let user_proxy = Arc::new(self.proxy.map(AppEvent::User));
        let rx = self.rx.take().unwrap();

        info!("Blocking for state");

        self.state = Some(self.rt
            .block_on(async move {
                State::new(window, rt, user_proxy, rx).await
            })
            .unwrap());
        info!("Blocked for state");

        // self.state = Some(
        //     tokio::runtime::Handle::current()
        //         .block_on(State::new(window))
        //         .unwrap(),
        // );
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: AppEvent) {
        info!("Event received: {event:?}");
        match event {
            // AppEvent::State(state) => self.state = Some(*state),
            AppEvent::User(user) => {
                let state = match &mut self.state {
                    Some(canvas) => canvas,
                    None => return,
                };

                state.handle_event(event_loop, user);
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        // info!("window event! {event:?}");
        let state = match &mut self.state {
            Some(canvas) => canvas,
            None => return,
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => state.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                state.update();
                match state.render() {
                    Result::Ok(_) => {}
                    Err(e) => {
                        // Log the error and exit gracefully
                        log::error!("{e}");
                        event_loop.exit();
                    }
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state: key_state,
                        ..
                    },
                ..
            } => state.handle_key(event_loop, code, key_state.is_pressed()),
            _ => {}
        }
    }
}
