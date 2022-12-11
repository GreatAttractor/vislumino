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

use std::collections::HashMap;

pub mod cmdline {
    pub const MODE: &str = "mode";
    pub const PROJECTION: &str = "projection";
    pub const HELP: &str = "help";
}

#[derive(Debug)]
pub enum GUIMode {
    Selectable,
    Projection
}

#[derive(Debug)]
pub enum Mode {
    GUI(GUIMode),
    PrintHelp,
}

#[derive(Debug)]
pub struct Parameters {
    pub mode: Mode
}

impl Parameters {
    pub fn mode(&self) -> &Mode { &self.mode }
}

pub fn print_help() {
    println!(
r#"Command-line options:

  (TODO)

"#);
}

/// Returns the value of a single-valued option of type `T`.
fn get_option_value<T: std::str::FromStr>(
    option: &str,
    option_values: &HashMap::<String, Vec<String>>,
    num_values: usize,
    required: bool
) -> Result<Vec<T>, String> {
    match option_values.get(option) {
        None => if required { Err(format!("missing option {}", option)) } else { Ok(vec![]) },

        Some(values) => if values.is_empty() {
            Err(format!("value missing for option {}", option))
        } else if values.len() > num_values {
            Err(format!("too many values for option {}", option))
        } else if values.len() < num_values {
            Err(format!("too few values for option {}", option))
        } else {
            let mut parsed_vals = vec![];
            for value in values {
                match value.parse::<T>() {
                    Ok(value) => parsed_vals.push(value),
                    Err(_) => { return Err(format!("invalid value for option {}: {}", option, value)); }
                }
            }
            Ok(parsed_vals)
        }
    }
}

/// Returns map of (option_name: option_values).
fn collect_options<I: Iterator<Item=String>>(
    stream: I,
    allowed_options: &[&str]
) -> Result<HashMap<String, Vec<String>>, String> {
    let mut option_values = HashMap::<String, Vec<String>>::new();
    let mut current: Option<&mut Vec<String>> = None;

    for arg in stream {
        if arg.starts_with("--") {
            match &arg[2..] {
                x if !allowed_options.contains(&x) => {
                    return Err(format!("unknown option: {}", x));
                },

                opt => current = Some(option_values.entry(opt.to_string()).or_insert(vec![])),
            }
        } else {
            if current.is_none() {
                return Err(format!("unexpected value: {}", arg));
            } else {
                (*(*current.as_mut().unwrap())).push(arg);
            }
        }
    }

    Ok(option_values)
}

/// Returns Ok(None) if help was requested.
pub fn parse_command_line<I: Iterator<Item=String>>(stream: I) -> Result<Parameters, String> {
    let mut mode_found = false;

    let mut stream = stream.skip(1); // skip the binary name

    loop {
        match stream.next() {
            Some(arg) => {
                if arg.starts_with("--") {
                    if &arg[2..] == cmdline::MODE {
                        mode_found = true;
                    } else {
                        return Err(format!("invalid option: {}, expected: --{}", arg, cmdline::MODE));
                    }
                } else if mode_found {
                    match arg.as_str() {
                        cmdline::PROJECTION => {
                            return Ok(Parameters{ mode: Mode::GUI(GUIMode::Projection) });
                        },

                        _ => { return Err(format!("unrecognized value: {}", arg)); }
                    }
                } else {
                    return Err(format!("invalid option: {}, expected: --{}", arg, cmdline::MODE));
                }
            },
            None => break
        }
    }

    Ok(Parameters{ mode: Mode::GUI(GUIMode::Selectable) })
}
