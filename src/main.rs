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

mod args;
mod config;
mod data;
mod disk;
mod gui;
mod image_utils;
mod img_seq;
mod long_fg_task;
mod projection;
mod runner;
mod subscriber;

const VERSION_STRING: &'static str = include_str!(concat!(env!("OUT_DIR"), "/version"));

fn print_header() {
    println!(r#"
_________________

   Vislumino - Astronomy Visualization Tools
   Copyright © 2022 Filip Szczerek <ga.software@yahoo.com>

   version {}

   This program comes with ABSOLUTELY NO WARRANTY. This is free software, licensed under GNU General Public License v3 and you are welcome to redistribute it under certain conditions. See the LICENSE file for details.

_________________
"#,
        VERSION_STRING
    );
}

fn run_program() -> bool {
    print_header();
    println!();

    match args::parse_command_line(std::env::args()) {
        Ok(config) => match config.mode {
            args::Mode::GUI(mode) => run_gui(mode),

            args::Mode::PrintHelp => return true,
        },

        Err(msg) => {
            println!("Error parsing arguments: {}.\n\nUse --{} for more information.\n", msg, args::cmdline::HELP);
            return false;
        }
    }

    true
}

fn run_gui(mode: args::GUIMode) {
    const DEFAULT_FONT_SIZE: f32 = 15.0;
    let (runner, worker_context) = runner::create_runner(DEFAULT_FONT_SIZE);
    let mut worker_context_opt: Option<_> = Some(worker_context);

    let mut base = Some(data::BaseProgramData{ config: config::Configuration::new() });

    let mut data: Option<data::ProgramData> = match mode {
        args::GUIMode::Selectable => None,

        args::GUIMode::Projection => Some(data::ProgramData::Projection(projection::ProgramData::new(
            base.take().unwrap(),
            runner.display(),
            worker_context_opt.take().unwrap()
        )))
    };

    let mut gui_state = gui::GuiState::new(runner.platform().hidpi_factor(), DEFAULT_FONT_SIZE);

    runner.main_loop(move |_, ui, display, renderer| {
        gui::handle_gui(&mut base, &mut data, ui, &mut gui_state, renderer, display, &mut worker_context_opt)
    });
}

fn main() {
    std::process::exit(if run_program() { 0 } else { 1 });
}
