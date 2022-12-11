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

use crate::runner;
use crate::gui;

const TITLE: &str = "Font";

pub fn handle_font_dialog(
    ui: &imgui::Ui,
    gui_state: &mut gui::GuiState,
    show: bool
) -> Option<runner::FontSizeRequest> {
    if show { ui.open_popup(TITLE); }

    let mut result = None;

    ui.popup_modal(TITLE).build(ui, || {
        let mut value = if let Some(fs) = gui_state.provisional_font_size {
            fs
        } else {
            gui_state.font_size
        };

        gui::add_text_before(ui, "Font size:");
        if ui.input_float("##font-size", &mut value)
            .step(0.5)
            .display_format("%0.1f")
            .enter_returns_true(true)
            .build() {
            if value > 50.0 { value = 50.0 } else if value < 5.0 { value = 5.0 };
            gui_state.provisional_font_size = Some(value);
            result = Some(runner::FontSizeRequest(value));
        }

        ui.separator();

        if ui.button("OK") {
            ui.close_current_popup();
            result = Some(runner::FontSizeRequest(value));
            gui_state.provisional_font_size = None;
        }
        ui.same_line();

        if ui.button("Cancel") {
            ui.close_current_popup();
            if gui_state.provisional_font_size.is_some() {
                result = Some(runner::FontSizeRequest(gui_state.font_size));
            }
            gui_state.provisional_font_size = None;
        }
    });

    result
}
