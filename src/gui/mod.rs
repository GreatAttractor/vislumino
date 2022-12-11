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

use crate::data;
use crate::projection;
use crate::runner;
use glium::glutin;
use std::cell::RefCell;
use std::rc::Rc;

pub mod about_dialog;
pub mod draw_buffer;
pub mod font_dialog;
pub mod long_task_dialog;

pub use draw_buffer::DrawBuffer;

const MODE_OF_OPERATION_POPUP_TITLE: &str = "Choose mode of operation";

pub struct MessageBox {
    pub title: String,
    pub message: String
}

#[derive(Default)]
pub struct GuiState {
    hidpi_factor: f64,
    mode_selection_activated: bool,
    pub mouse_drag_origin: [f32; 2],
    pub message_box: Option<MessageBox>,
    pub font_size: f32,
    pub provisional_font_size: Option<f32>
}

impl GuiState {
    pub fn new(hidpi_factor: f64, font_size: f32) -> GuiState {
        GuiState{
            hidpi_factor,
            font_size,
            mode_selection_activated: false,
            ..Default::default()
        }
    }

    pub fn hidpi_factor(&self) -> f64 { self.hidpi_factor }
}

pub fn handle_gui(
    base: &mut Option<data::BaseProgramData>,
    program_data: &mut Option<data::ProgramData>,
    ui: &imgui::Ui,
    gui_state: &mut GuiState,
    renderer: &Rc<RefCell<imgui_glium_renderer::Renderer>>,
    display: &glium::Display,
    worker_context: &mut Option<glutin::Context<glutin::NotCurrent>>
) -> Option<runner::FontSizeRequest> {
    unsafe { imgui::sys::igDockSpaceOverViewport(
        imgui::sys::igGetMainViewport(),
        imgui::sys::ImGuiDockNodeFlags_PassthruCentralNode as i32,
        std::ptr::null()
    ); }

    if program_data.is_none() && !gui_state.mode_selection_activated {
        ui.open_popup(MODE_OF_OPERATION_POPUP_TITLE);
        gui_state.mode_selection_activated = true;
    }

    if let Some(program_data) = program_data {
        match program_data {
            data::ProgramData::Projection(program_data) => projection::handle_gui(
                program_data,
                ui,
                gui_state,
                renderer,
                display
            )
        }
    } else {
        handle_mode_selection(base, program_data, ui, display, worker_context);
        None
    }
}

fn mult_size(size: [f32; 2], factor: f32) -> [f32; 2] {
    [size[0] * factor, size[1] * factor]
}

fn handle_mode_selection(
    base: &mut Option<data::BaseProgramData>,
    program_data: &mut Option<data::ProgramData>,
    ui: &imgui::Ui,
    display: &glium::Display,
    worker_context: &mut Option<glium::glutin::Context<glium::glutin::NotCurrent>>
) {
    unsafe { imgui::sys::igSetNextWindowSize(
        imgui::sys::ImVec2{ x: 600.0, y: 300.0 }, //TODO: use 1/2 of program's window size
        imgui::sys::ImGuiCond_FirstUseEver as i32
    ); }

    ui.popup_modal(MODE_OF_OPERATION_POPUP_TITLE).build(ui, || {
        let btn_label: &str = "Planetary projection";
        if ui.button_with_size(btn_label, mult_size(ui.calc_text_size(btn_label), 3.0)) {
            *program_data = Some(data::ProgramData::Projection(projection::ProgramData::new(
                base.take().unwrap(),
                display,
                worker_context.take().unwrap()
            )));

            ui.close_current_popup();
        }

        add_spacer(ui);
        ui.separator();

        let btn_label: &str = "About...";
        let mut about_clicked = false;
        if ui.button_with_size(btn_label, mult_size(ui.calc_text_size(btn_label), 2.0)) {
            about_clicked = true;
        }

        about_dialog::handle_about_dialog(ui, about_clicked);
    });
}

fn add_spacer(ui: &imgui::Ui) {
    ui.dummy(ui.calc_text_size("M"));
}

pub struct AdjustedImageSize {
    pub logical_size: [f32; 2],
    pub physical_size: [u32; 2]
}

/// Adjusts cursor screen position and returns size to be used for an `imgui::Image` (meant to fill the remaining window
/// space) to ensure exact 1:1 pixel rendering when high-DPI scaling is enabled.
pub fn adjust_pos_for_exact_hidpi_scaling(
    ui: &imgui::Ui,
    vertical_space_after: f32,
    hidpi_factor: f32
) -> AdjustedImageSize {
    let scr_pos = ui.cursor_screen_pos();

    let adjusted_pos_x = if (scr_pos[0] * hidpi_factor).fract() != 0.0 {
        (scr_pos[0] * hidpi_factor).trunc() / hidpi_factor
    } else {
        scr_pos[0]
    };

    let adjusted_pos_y = if (scr_pos[1] * hidpi_factor).fract() != 0.0 {
        (scr_pos[1] * hidpi_factor).trunc() / hidpi_factor
    } else {
        scr_pos[1]
    };

    ui.set_cursor_screen_pos([adjusted_pos_x, adjusted_pos_y]);

    let mut size = ui.content_region_avail();
    size[1] -= vertical_space_after;

    let mut adjusted_size_x = size[0].trunc();
    if (adjusted_size_x * hidpi_factor).fract() != 0.0 {
        adjusted_size_x = (adjusted_size_x * hidpi_factor).trunc() / hidpi_factor;
    }

    let mut adjusted_size_y = size[1].trunc();
    if (adjusted_size_y * hidpi_factor).fract() != 0.0 {
        adjusted_size_y = (adjusted_size_y * hidpi_factor).trunc() / hidpi_factor;
    }

    let physical_size = [
        (adjusted_size_x * hidpi_factor).trunc() as u32,
        (adjusted_size_y * hidpi_factor).trunc() as u32
    ];

    AdjustedImageSize{
        logical_size: [adjusted_size_x, adjusted_size_y],
        physical_size
    }
}

/// Adjusts cursor screen position and returns size to be used for an `imgui::Image` (meant to fill the remaining window
/// space) to ensure exact 1:1 pixel rendering when high-DPI scaling is enabled.
pub fn adjust_pos_size_for_exact_hidpi_scaling(
    ui: &imgui::Ui,
    hidpi_factor: f32,
    original_logical_size: [f32; 2]
) -> AdjustedImageSize {
    let scr_pos = ui.cursor_screen_pos();

    let adjusted_pos_x = if (scr_pos[0] * hidpi_factor).fract() != 0.0 {
        (scr_pos[0] * hidpi_factor).trunc() / hidpi_factor
    } else {
        scr_pos[0]
    };

    let adjusted_pos_y = if (scr_pos[1] * hidpi_factor).fract() != 0.0 {
        (scr_pos[1] * hidpi_factor).trunc() / hidpi_factor
    } else {
        scr_pos[1]
    };

    ui.set_cursor_screen_pos([adjusted_pos_x, adjusted_pos_y]);

    let mut adjusted_size_x = original_logical_size[0].trunc();
    if (adjusted_size_x * hidpi_factor).fract() != 0.0 {
        adjusted_size_x = (adjusted_size_x * hidpi_factor).trunc() / hidpi_factor;
    }

    let mut adjusted_size_y = original_logical_size[1].trunc();
    if (adjusted_size_y * hidpi_factor).fract() != 0.0 {
        adjusted_size_y = (adjusted_size_y * hidpi_factor).trunc() / hidpi_factor;
    }

    let physical_size = [
        (adjusted_size_x * hidpi_factor).trunc() as u32,
        (adjusted_size_y * hidpi_factor).trunc() as u32
    ];

    AdjustedImageSize{
        logical_size: [adjusted_size_x, adjusted_size_y],
        physical_size
    }
}

pub fn add_text_before(ui: &imgui::Ui, text: &str) {
    ui.align_text_to_frame_padding();
    ui.text(text);
    ui.same_line();
}

pub fn tooltip(ui: &imgui::Ui, text: &str) {
    if ui.is_item_hovered() {
        ui.tooltip_text(text);
    }
}

/// Returns adjusted `image_size` (preserving w/h ratio) so that image touches the container from inside.
pub fn touch_from_inside(image_size: [u32; 2], container_size: [f32; 2]) -> [f32; 2] {
    let container_wh_ratio = container_size[0] / container_size[1];
    let image_wh_ratio = image_size[0] as f32 / image_size[1] as f32;

    let mut new_width = container_size[0];
    let mut new_height = container_size[1];

    if container_wh_ratio >= image_wh_ratio {
        new_width = new_height * image_wh_ratio;
    } else {
        new_height = new_width / image_wh_ratio;
    }

    [new_width, new_height]
}

/// Returns adjusted `image_size` (preserving w/h ratio) so that image fills the container vertically.
pub fn fill_vertically(image_size: [u32; 2], container_size: [f32; 2]) -> [f32; 2] {
    let image_wh_ratio = image_size[0] as f32 / image_size[1] as f32;

    let new_height = container_size[1];
    let new_width = new_height * image_wh_ratio;

    [new_width, new_height]
}


pub fn handle_message_box(ui: &imgui::Ui, gui_state: &GuiState) {
    if let Some(message_box) = &gui_state.message_box {
        ui.popup_modal(&message_box.title).build(ui, || {
            ui.text(&message_box.message);
            ui.separator();
            if ui.button("Close") {
                ui.close_current_popup();
            }
        });
    }
}
