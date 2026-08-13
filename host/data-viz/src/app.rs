mod camera;
mod event;
mod instance;
mod light;
mod model;
mod resources;
mod state;
mod texture;

use std::sync::Arc;

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
}

impl App {
    pub const fn new(rt: tokio::runtime::Handle, proxy: EventLoopProxy<AppEvent>) -> Self {
        Self {
            state: None,
            rt,
            proxy,
        }
    }
}

impl ApplicationHandler<AppEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes().with_title("SkiBoot Viz");

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        // If we are not on web we can use pollster to
        // await the window creation

        let (rt, proxy) = (self.rt.clone(), self.proxy.clone());
        let user_proxy = Arc::new(self.proxy.map(AppEvent::User));

        self.rt.spawn(async move {
            proxy.send_event(AppEvent::State(Box::new(
                State::new(window, rt, user_proxy).await.unwrap(),
            )))
        });

        // self.state = Some(
        //     tokio::runtime::Handle::current()
        //         .block_on(State::new(window))
        //         .unwrap(),
        // );
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: AppEvent) {
        match event {
            AppEvent::State(state) => self.state = Some(*state),
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
