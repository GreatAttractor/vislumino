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

use chrono::prelude::Utc;

fn main() {
    let output_dir = std::env::var("OUT_DIR").unwrap();
    let version_path = std::path::Path::new(&output_dir).join("version");

    let version_str = format!(
        "{} (commit {}, {} {}, built on {})",
        env!("CARGO_PKG_VERSION"),
        get_commit_hash(),
        std::env::consts::OS, std::env::consts::ARCH,
        Utc::now().format("%Y-%m-%d %H:%M UTC")
    );

    std::fs::write(version_path, version_str).unwrap();

    embed_resource::compile("app.rc");
}

fn get_commit_hash() -> String {
    let output = std::process::Command::new("git")
        .arg("log").arg("-1")
        .arg("--pretty=format:%h")
        .arg("--abbrev=8")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();

    if output.status.success() {
        String::from_utf8_lossy(&output.stdout).to_string()
    } else {
        "unspecified".to_string()
    }
}
