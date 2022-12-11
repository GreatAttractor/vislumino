//
// Vislumino - Astronomy Visualization Tools
// Copyright (c) 2022 Filip Szczerek <ga.software@yahoo.com>
//
// This file is part of Vislumino.
//
// Vislumino is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, version 3.
//
// Vislumino is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Vislumino.  If not, see <http://www.gnu.org/licenses/>.
//

use glium::{glutin, Surface};
use std::cell::RefCell;
use std::rc::Rc;

mod clipboard_support;

#[derive(Copy, Clone)]
pub struct FontSizeRequest(pub f32);

pub struct Runner {
    event_loop: glium::glutin::event_loop::EventLoop<()>,
    display: glium::Display,
    imgui: imgui::Context,
    platform: imgui_winit_support::WinitPlatform,
    renderer: Rc<RefCell<imgui_glium_renderer::Renderer>>
}

fn load_raw_gl_functions<F: Fn(&str) -> *const std::ffi::c_void>(loader: F) {
    gl::BindBuffer::load_with(&loader);
    gl::BindTexture::load_with(&loader);
    gl::GenTextures::load_with(&loader);
    gl::GetError::load_with(&loader);
    gl::GetIntegerv::load_with(&loader);
    gl::GetTexImage::load_with(&loader);
    gl::PixelStorei::load_with(&loader);
    gl::TexImage2D::load_with(&loader);
    gl::TexParameteri::load_with(&loader);
    gl::Finish::load_with(&loader);
}

fn create_font(physical_font_size: f32) -> imgui::FontSource<'static> {
    imgui::FontSource::TtfData{
        data: include_bytes!(
            "../resources/fonts/DejaVuSans.ttf"
        ),
        size_pixels: physical_font_size,
        config: Some(imgui::FontConfig {
            glyph_ranges: imgui::FontGlyphRanges::from_slice(&[
                0x0020, 0x00FF, // Basic Latin, Latin-1 Supplement
                '▶' as u32, '▶' as u32,
                '■' as u32, '■' as u32,
                '⟳' as u32, '⟳' as u32,
                '⇄' as u32, '⇄' as u32,
                '⚙' as u32, '⚙' as u32,
                0
            ]),
            ..imgui::FontConfig::default()
        }),
    }.into()
}

pub fn create_runner(logical_font_size: f32) -> (Runner, glium::glutin::Context<glium::glutin::NotCurrent>) {
    let event_loop = glium::glutin::event_loop::EventLoop::new();
    let context = glium::glutin::ContextBuilder::new().with_vsync(true);
    let builder = glium::glutin::window::WindowBuilder::new()
        .with_title("Vislumino".to_owned())
        .with_inner_size(glium::glutin::dpi::LogicalSize::new(1280f64, 768f64));
    let display =
        glium::Display::new(builder, context, &event_loop).expect("failed to initialize display");

    let mut imgui = imgui::Context::create();
    imgui.set_ini_filename(None);

    if let Some(backend) = clipboard_support::init() {
        imgui.set_clipboard_backend(backend);
    } else {
        eprintln!("Failed to initialize clipboard.");
    }

    let mut platform = imgui_winit_support::WinitPlatform::init(&mut imgui);
    {
        let gl_window = display.gl_window();
        let window = gl_window.window();
        platform.attach_window(imgui.io_mut(), &window, imgui_winit_support::HiDpiMode::Default);
    }

    let hidpi_factor = platform.hidpi_factor() as f32;
    let font_size = logical_font_size * hidpi_factor;

    imgui.fonts().add_font(&[create_font(font_size)]);

    imgui.io_mut().font_global_scale = 1.0 / hidpi_factor;
    imgui.io_mut().config_flags |= imgui::ConfigFlags::DOCKING_ENABLE;
    imgui.io_mut().config_windows_move_from_title_bar_only = true;

    let renderer = imgui_glium_renderer::Renderer::init(&mut imgui, &display).expect("failed to initialize renderer");

    let worker_context;

    {
        let window = display.gl_window();
        let context = window.context();
        let worker_context_builder = glium::glutin::ContextBuilder::new().with_shared_lists(context);
        let event_loop = glium::glutin::event_loop::EventLoop::new();

        load_raw_gl_functions(|symbol| window.context().get_proc_address(symbol) as _);

        worker_context = worker_context_builder.build_headless(&event_loop, glutin::dpi::PhysicalSize{ width: 128, height: 128 }).unwrap();
    }

    (Runner {
        event_loop,
        display,
        imgui,
        platform,
        renderer: Rc::new(RefCell::new(renderer))
    }, worker_context)
}

impl Runner {
    pub fn platform(&self) -> &imgui_winit_support::WinitPlatform {
        &self.platform
    }

    pub fn display(&self) -> &glium::Display {
        &self.display
    }

    pub fn main_loop<F>(self, mut run_ui: F)
        where F: FnMut(
            &mut bool,
            &mut imgui::Ui,
            &glium::Display,
            &Rc<RefCell<imgui_glium_renderer::Renderer>>
        ) -> Option<FontSizeRequest> + 'static
    {
        let Runner {
            event_loop,
            display,
            mut imgui,
            mut platform,
            renderer,
            ..
        } = self;

        let mut last_frame = std::time::Instant::now();

        event_loop.run(move |event, _, control_flow| match event {
            glium::glutin::event::Event::NewEvents(_) => {
                let now = std::time::Instant::now();
                imgui.io_mut().update_delta_time(now - last_frame);
                last_frame = now;
            },

            glium::glutin::event::Event::MainEventsCleared => {
                let gl_window = display.gl_window();
                platform
                    .prepare_frame(imgui.io_mut(), &gl_window.window())
                    .expect("failed to prepare frame");
                gl_window.window().request_redraw();
            },

            glium::glutin::event::Event::RedrawRequested(_) => {
                let font_size_request;
                {
                    let mut ui = imgui.frame();

                    let mut run = true;
                    font_size_request = run_ui(&mut run, &mut ui, &display, &renderer);
                    if !run {
                        *control_flow = glium::glutin::event_loop::ControlFlow::Exit;
                    }

                    let gl_window = display.gl_window();
                    let mut target = display.draw();
                    target.clear_color_srgb(0.5, 0.5, 0.5, 1.0);
                    platform.prepare_render(&ui, gl_window.window());
                    let draw_data = imgui.render();
                    renderer.borrow_mut()
                        .render(&mut target, draw_data)
                        .expect("rendering failed");
                    target.finish().expect("failed to swap buffers");
                }
                if let Some(fsr) = font_size_request {
                    imgui.fonts().clear();
                    imgui.fonts().add_font(&[create_font(platform.hidpi_factor() as f32 * fsr.0)]);
                    renderer.borrow_mut().reload_font_texture(&mut imgui).unwrap();
                }
            },

            glium::glutin::event::Event::WindowEvent {
                event: glium::glutin::event::WindowEvent::CloseRequested,
                ..
            } => *control_flow = glium::glutin::event_loop::ControlFlow::Exit,

            event => {
                let converted_event = convert_touch_to_mouse(event);

                let gl_window = display.gl_window();
                platform.handle_event(imgui.io_mut(), gl_window.window(), &converted_event);
            }
        })
    }
}

fn convert_touch_to_mouse<'a, T>(event: glium::glutin::event::Event<'a, T>) -> glium::glutin::event::Event<'a, T> {
    use glium::glutin::event;

    match event {
        event::Event::WindowEvent {
            window_id,
            event: event::WindowEvent::Touch(touch),
        } => {
            //TODO: do something better here, e.g. remember the last seen mouse device id
            let device_id = touch.device_id.clone();

            match touch.phase {
                event::TouchPhase::Started => event::Event::WindowEvent{
                    window_id: window_id.clone(),
                    event: event::WindowEvent::MouseInput{
                        device_id,
                        state: event::ElementState::Pressed,
                        button: event::MouseButton::Left,
                        modifiers: Default::default()
                    },
                },

                event::TouchPhase::Ended => event::Event::WindowEvent{
                    window_id: window_id.clone(),
                    event: event::WindowEvent::MouseInput{
                        device_id,
                        state: event::ElementState::Released,
                        button: event::MouseButton::Left,
                        modifiers: Default::default()
                    },
                },

                _ => event
            }
        },

        _ => event
    }
}