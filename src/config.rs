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

//TODO: add support for OsStr values (file system paths which may be not UTF-8)

use configparser::ini::Ini;
use std::path::{Path, PathBuf};

const CONFIG_FILE_NAME: &str = "vislumino.ini";

mod ids {
    pub mod pproj {
        pub const GROUP: &str = "PlanetaryProjection";

        pub const PROJECTION_EXPORT_PATH: &str = "ProjectionExportPath";
        pub const LOAD_PATH: &str = "LoadPath";
    }
}

pub trait ProjectionConfig {
    fn load_path(&self) -> Option<PathBuf>;
    fn set_load_path(&mut self, value: &str);

    fn projection_export_path(&self) -> Option<PathBuf>;
    fn set_projection_export_path(&mut self, value: &str);
}

pub struct Configuration {
    config_file: Ini
}

impl Configuration {
    pub fn store(&self) -> Result<(), std::io::Error> {
        self.config_file.write(config_file_path())
    }

    pub fn new() -> Configuration {
        let mut config_file = Ini::new_cs();
        let file_path = config_file_path();
        if config_file.load(file_path.clone()).is_err() {
            println!(
                "Could not load configuration from {}. A new configuration file will be created.",
                file_path.to_string_lossy()
            );
        }

        Configuration{ config_file }
    }
}

impl ProjectionConfig for Configuration {
    fn projection_export_path(&self) -> Option<PathBuf> {
        match self.config_file.get(ids::pproj::GROUP, ids::pproj::PROJECTION_EXPORT_PATH) {
            None => None,
            Some(s) => Some(s.into())
        }
    }

    fn set_projection_export_path(&mut self, value: &str) {
        self.config_file.set(ids::pproj::GROUP, ids::pproj::PROJECTION_EXPORT_PATH, Some(value.into()));
    }

    fn load_path(&self) -> Option<PathBuf> {
        match self.config_file.get(ids::pproj::GROUP, ids::pproj::LOAD_PATH) {
            None => None,
            Some(s) => Some(s.into())
        }
    }

    fn set_load_path(&mut self, value: &str) {
        self.config_file.set(ids::pproj::GROUP, ids::pproj::LOAD_PATH, Some(value.into()));
    }
}

impl Drop for Configuration {
    fn drop(&mut self) {
        if let Err(e) = self.store() {
            eprintln!("Error saving configuration: {}.", e.to_string());
        }
    }
}

fn config_file_path() -> PathBuf {
    Path::new(&dirs::config_dir().or(Some(Path::new("").to_path_buf())).unwrap()).join(CONFIG_FILE_NAME)
}
