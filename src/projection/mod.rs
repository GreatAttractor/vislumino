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

use crate::config::ProjectionConfig;
use crate::gui;
use crate::gui::long_task_dialog::LongTaskDialog;
use crate::image_utils;
use crate::projection;
use crate::runner;
use crossbeam::channel::TryRecvError;
use ga_image::PixelFormat;
use glium::{CapabilitiesSource, GlObject};
use std::cell::RefCell;
use std::rc::Rc;
use strum::IntoEnumIterator;

mod data;
mod export_dialog;
mod globe_view;
mod projection_view;
mod source_view;
mod worker;

pub use data::ProgramData;
pub use export_dialog::{ExportDialog, handle_export_dialog};
pub use globe_view::GlobeView;
pub use projection_view::ProjectionView;
pub use source_view::SourceView;

use self::worker::MainToWorkerMsg;

#[derive(Copy, Clone, strum::EnumIter, PartialEq)]
pub enum Planet {
    Jupiter,
    Mars
}

impl Planet {
    pub fn name(&self) -> &str {
        match self {
            Planet::Jupiter => "Jupiter",
            Planet::Mars => "Mars",
        }
    }

    pub fn flattening(&self) -> f32 {
        match self {
            Planet::Jupiter => 0.06487,
            Planet::Mars => 0.00589,
        }
    }

    pub fn sidereal_rotation(&self) -> std::time::Duration {
        match self {
            Planet::Jupiter => std::time::Duration::from_secs(9 * 3600 + 55 * 60 + 30),
            Planet::Mars => std::time::Duration::from_secs(24 * 3600 + 37 * 60 + 23),
        }
    }

    pub fn as_index(&self) -> usize {
        for (idx, s) in Planet::iter().enumerate() {
            if s == *self { return idx; }
        }
        unreachable!()
    }
}

impl From<usize> for Planet {
    fn from(u: usize) -> Planet {
        for (idx, s) in Planet::iter().enumerate() {
            if idx == u { return s; }
        }
        panic!("cannot deduce Planet from index {}", u);
    }
}

fn handle_main_menu(
    ui: &imgui::Ui,
    gui_state: &mut gui::GuiState,
    program_data: &mut data::ProgramData,
    renderer: &Rc<RefCell<imgui_glium_renderer::Renderer>>,
    display: &glium::Display
) -> Option<runner::FontSizeRequest> {
    let mut about_clicked = false;
    let mut load_images_clicked = false;
    let mut new_projection_view_clicked = false;
    let mut new_globe_view_clicked = false;
    let mut font_size_clicked = false;

    match ui.begin_main_menu_bar() {
        None => (),

        Some(_) => {
            ui.menu("File", || { if ui.menu_item("Load images...") { load_images_clicked = true; }});

            ui.menu("View", || {
                let token = ui.begin_enabled(program_data.source_view().is_some());

                ui.menu("New", || {
                    if ui.menu_item("Projection") { new_projection_view_clicked = true; }
                    if ui.menu_item("Globe") { new_globe_view_clicked = true; }
                });

                token.end();
            });

            ui.menu("Settings", || { if ui.menu_item("Font size...") { font_size_clicked = true; }});

            ui.menu("Help", || { if ui.menu_item("About...") { about_clicked = true; }});
        }
    }

    gui::about_dialog::handle_about_dialog(ui, about_clicked);

    let font_size_request = gui::font_dialog::handle_font_dialog(ui, gui_state, font_size_clicked);

    if load_images_clicked { handle_load_images(ui, gui_state, display, program_data); }

    if new_projection_view_clicked { program_data.add_projection_view(display, renderer); }

    if new_globe_view_clicked { program_data.add_globe_view(display, renderer); }

    font_size_request
}

pub fn handle_gui(
    program_data: &mut ProgramData,
    ui: &imgui::Ui,
    gui_state: &mut crate::gui::GuiState,
    renderer: &Rc<RefCell<imgui_glium_renderer::Renderer>>,
    display: &glium::Display
) -> Option<runner::FontSizeRequest> {
    let result = handle_main_menu(ui, gui_state, program_data, renderer, display);

    let allow_playback = program_data.long_task_dialog().borrow().is_none();

    if let Some(source_view) = program_data.source_view_mut() {
        source_view::handle_source_view(ui, gui_state, source_view, allow_playback);
    }

    program_data.globe_views().borrow_mut().retain_mut(
        |view| globe_view::handle_globe_view(
            ui,
            gui_state,
            &mut view.borrow_mut(),
            program_data.long_task_dialog(),
            program_data.bg_task_sender()
        )
    );

    program_data.projection_views().borrow_mut().retain_mut(
        |view| projection_view::handle_projection_view(
            ui,
            gui_state,
            &mut program_data.base().borrow_mut().config,
            &mut view.borrow_mut(),
            program_data.source_view().as_ref().unwrap(),
            program_data.long_task_dialog(),
            program_data.bg_task_sender(),
            program_data.export_dialog()
        )
    );

    let mut in_progress = false;
    if let Some(long_task_dialog) = &mut *program_data.long_task_dialog().borrow_mut() {
        if let Some(long_fg_task) = &mut *program_data.long_fg_task().borrow_mut() {
            long_fg_task.step();
        }

        in_progress = gui::long_task_dialog::handle_long_task(
            ui,
            long_task_dialog,
            || {
                if let Some(long_fg_task) = &mut *program_data.long_fg_task().borrow_mut() {
                    long_fg_task.cancel();
                } else {
                    program_data.bg_task_sender().send(MainToWorkerMsg::Cancel).unwrap();
                }
            }
        );
    }
    if !in_progress {
        *program_data.long_fg_task().borrow_mut() = None;
        *program_data.long_task_dialog().borrow_mut() = None;
    }

    handle_image_loading(ui, gui_state, program_data, renderer, display);

    gui::handle_message_box(ui, gui_state);

    result
}

fn handle_image_loading(
    ui: &imgui::Ui,
    gui_state: &mut gui::GuiState,
    program_data: &mut ProgramData,
    renderer: &Rc<RefCell<imgui_glium_renderer::Renderer>>,
    display: &glium::Display
) {
    let mut finished = false;
    let mut loaded = false;
    let mut disk_info: Option<worker::DiskInfo> = None;

    match program_data.image_loading() {
        None => (),
        Some(imgl) => {
            match imgl.receiver.try_recv() {
                Ok(msg) => match msg {
                    worker::LoadImagesResultMsg::Success(dinfo) => {
                        loaded = true;
                        disk_info = Some(dinfo);
                        finished = true;
                    },

                    worker::LoadImagesResultMsg::Error(e) => {
                        finished = true;
                        gui_state.message_box = Some(gui::MessageBox{
                            title: "Error".to_string(),
                            message: format!("Failed to load images: {}.", e)
                        });
                        ui.open_popup("Error");
                    },

                    worker::LoadImagesResultMsg::Cancelled => finished = true,
                },

                Err(e) => match e {
                    TryRecvError::Empty => (),
                    _ => panic!("unexpected error {}", e)
                }
            }
        }
    }

    if loaded {
        let image_loading = program_data.image_loading_mut().take().unwrap();
        let disk_info = disk_info.unwrap();

        match program_data.source_view_mut() {
            None => *program_data.source_view_mut() = Some(source_view::SourceView::new(
                &program_data.gl_objects,
                display,
                renderer,
                image_loading.textures,
                disk_info.center,
                disk_info.diameter
            )),

            Some(source_view) =>
                source_view.set_images(image_loading.textures, disk_info.center, disk_info.diameter)
        }
    }

    if finished { *program_data.image_loading_mut() = None; }
}

fn handle_load_images(
    ui: &imgui::Ui,
    gui_state: &mut gui::GuiState,
    display: &glium::Display,
    program_data: &mut ProgramData
) {
    assert!(program_data.image_loading().is_none());

    let max_texture_size = display.get_capabilities().max_texture_size as u32;

    let mut paths = native_dialog::FileDialog::new()
        .set_location(&(match program_data.base().borrow().config.load_path() { None => "".into(), Some(p) => p }))
        .add_filter("image files (BMP, PNG, TIFF)", &["bmp", "png", "tif", "tiff"])
        .add_filter("BMP", &["bmp"])
        .add_filter("PNG", &["png"])
        .add_filter("TIFF", &["tif", "tiff"])
        .add_filter("all files", &["*"])
        .show_open_multiple_file()
        .unwrap();

    if !paths.is_empty() {
        paths.sort();

        // TODO: handle error gracefully
        // TODO: handle different pixel formats and bit depths
        let (width, height) = match image_utils::get_metadata(&paths[0]) {
            Ok((width, height, _)) => (width, height),

            Err(e) => {
                gui_state.message_box = Some(gui::MessageBox{
                    title: "Error".to_string(),
                    message: format!("{}", e.to_string())
                });
                ui.open_popup("Error");
                return;
            }
        };

        if width > max_texture_size || height > max_texture_size {
            panic!("image too big"); //TODO: handle gracefully
        }

        let textures: Vec<_> = (0..paths.len()).map(|_| Rc::new(glium::Texture2d::empty_with_format(
                display,
                glium::texture::UncompressedFloatFormat::U8U8U8,
                glium::texture::MipmapsOption::NoMipmap,
                width,
                height
            ).unwrap())
        ).collect();

        let (result_sender, result_receiver) = crossbeam::channel::unbounded();

        let (progress_sender, progress_receiver) = crossbeam::channel::bounded(1);

        program_data.bg_task_sender().send(worker::MainToWorkerMsg::LoadImages(worker::LoadImages{
            dimensions: [width, height],
            pixel_format: PixelFormat::RGB8,
            items: textures.iter().map(|t| t.get_id())
                .zip(paths.iter())
                .map(|(id, path)| (id, path.clone()))
                .collect(),
            progress_sender,
            result_sender
        })).unwrap();

        *program_data.image_loading_mut() = Some(projection::data::ImageLoading{ textures, receiver: result_receiver });

        *program_data.long_task_dialog().borrow_mut() =
            Some(LongTaskDialog::new("Image Loading".to_string(), "".to_string(), progress_receiver));

        program_data.base().borrow_mut().config.set_load_path(paths[0].parent().unwrap().to_str().unwrap()); //TODO: handle non-UTF-8 paths
    }
}
