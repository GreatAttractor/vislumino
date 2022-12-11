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

use crate::image_utils;
use crate::img_seq::ImageSequence;

pub fn create_image_list(file_paths: Vec<std::path::PathBuf>) -> Box<dyn ImageSequence> {
    Box::new(ImageList{ file_paths })
}

struct ImageList {
    file_paths: Vec<std::path::PathBuf>
}

impl ImageSequence for ImageList {
    fn get_image(&mut self, index: usize) -> Result<ga_image::Image, Box<dyn std::error::Error>> {
        image_utils::load_image(&self.file_paths[index])
    }

    fn num_images(&self) -> usize { self.file_paths.len() }
}
