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

mod image_list;
mod ser;

pub use image_list::create_image_list;
pub use ser::open_ser_video;

#[derive(Debug)]
pub struct ImgSeqError {
    description: String
}

impl ImgSeqError {
    fn new(description: String) -> Box<dyn std::error::Error> {
        Box::new(ImgSeqError{ description })
    }
}

impl std::fmt::Display for ImgSeqError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        let _ = write!(f, "{}", self.description);
        Ok(())
    }
}

impl std::error::Error for ImgSeqError {}

pub trait ImageSequence {
    fn get_image(&mut self, index: usize) -> Result<ga_image::Image, Box<dyn std::error::Error>>;

    fn num_images(&self) -> usize;
}
