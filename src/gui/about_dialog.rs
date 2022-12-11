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

const TITLE: &str = "About";

pub fn handle_about_dialog(ui: &imgui::Ui, show: bool) {
    if show { ui.open_popup(TITLE); }

    ui.popup_modal(TITLE).build(ui, || {
        ui.text(format!(r#"Vislumino - Astronomy Visualization Tools
Copyright © 2022 Filip Szczerek <ga.software@yahoo.com>

version {}

This program comes with ABSOLUTELY NO WARRANTY. This is free software,
licensed under GNU General Public License v3 and you are welcome
to redistribute it under certain conditions. See the LICENSE file for details.
"#, crate::VERSION_STRING));
        ui.separator();
        if ui.button("Close") {
            ui.close_current_popup();
        }
    });
}
