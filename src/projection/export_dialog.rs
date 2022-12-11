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

use crate::gui;
use std::path::PathBuf;

pub struct ExportDialog {
    title: String,
    output_path: Option<PathBuf>,
    bounce_back: bool
}

impl ExportDialog {
    pub fn new(title: String, output_path: Option<PathBuf>) -> ExportDialog {
        ExportDialog{
            title,
            output_path,
            bounce_back: false
        }
    }

    pub fn title(&self) -> &str { &self.title }

    pub fn output_path(&self) -> PathBuf { self.output_path.as_ref().unwrap().clone() }

    pub fn bounce_back(&self) -> bool { self.bounce_back }
}

/// Returns `true` if dialog was accepted.
pub fn handle_export_dialog(
    ui: &imgui::Ui,
    gui_state: &mut gui::GuiState,
    dialog: &mut ExportDialog,
) -> bool {
    let mut result = false;

    ui.popup_modal(&dialog.title).build(ui, || {
        if ui.button("Output folder...") {
            let prev_path = match &dialog.output_path {
                Some(path) => path.clone(),
                None => PathBuf::from("")
            };
            let path = native_dialog::FileDialog::new()
                .set_location(&prev_path) // TODO: remember the MRU
                .show_open_single_dir()
                .unwrap();

            if let Some(path) = path {
                dialog.output_path = Some(path);
            }
        }
        ui.same_line();
        match &dialog.output_path {
            Some(path) => ui.text(path.as_os_str().to_string_lossy()),
            None => ui.text_disabled("(no folder selected)")
        }

        ui.checkbox("Back-and-forth sequence (1, 2, ... n-1, n, n-1, ... 2, 1)", &mut dialog.bounce_back);

        ui.separator();
        if ui.button("Export") {
            if dialog.output_path.is_none() {
                gui_state.message_box = Some(gui::MessageBox{
                    title: "Error".to_string(),
                    message: format!("Output folder not selected.")
                });
                ui.open_popup("Error");
            } else {
                result = true;
                ui.close_current_popup();
            }
        }
        ui.same_line();

        if ui.button("Cancel") {
            ui.close_current_popup();
        }

        gui::handle_message_box(ui, gui_state);
    });

    result
}
