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

use crossbeam::channel::TryRecvError;

pub struct ProgressMsg {
    info: String,
    progress: f32
}

impl ProgressMsg {
    pub fn new(info: String, progress: f32) -> ProgressMsg {
        assert!(progress >= 0.0 && progress <= 1.0);
        ProgressMsg { info, progress }
    }
}

/// Note: reports end of task only if `progress_receiver` becomes disconnected; owners of the receiver must remember to
/// disconnect one way or another (by getting dropped, or by dropping just the sender).
pub struct LongTaskDialog {
    title: String,
    info: String,
    progress: f32,
    progress_receiver: crossbeam::channel::Receiver<ProgressMsg>
}

impl LongTaskDialog {
    pub fn new(title: String, info: String, progress_receiver: crossbeam::channel::Receiver<ProgressMsg>) -> LongTaskDialog {
        LongTaskDialog{
            title,
            info,
            progress: 0.0,
            progress_receiver
        }
    }
}

/// Returns true if the task is still in progress.
pub fn handle_long_task<F: Fn()>(ui: &imgui::Ui, long_task: &mut LongTaskDialog, on_cancel: F) -> bool {
    let mut in_progress = true;

    ui.open_popup(&long_task.title);
    ui.popup_modal(&long_task.title).build(ui, || {
        match long_task.progress_receiver.try_recv() {
            Ok(msg) => {
                long_task.info = msg.info;
                long_task.progress = msg.progress;
            },

            Err(e) => match e {
                TryRecvError::Disconnected => in_progress = false,
                TryRecvError::Empty => ()
            }
        }

        ui.text(&long_task.info);

        imgui::ProgressBar::new(long_task.progress)
            .overlay_text(&format!("{:.1}%", 100.0 * long_task.progress))
            .build(ui);

        if ui.button("Cancel") { on_cancel(); }
    });

    in_progress
}
