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

use cgmath::{EuclideanSpace, Point2};

/// Returns (center, diameter).
pub fn find_planetary_disk(image: &ga_image::Image) -> Result<(Point2<f32>, f32), ()> {
    let mut image8 = image.convert_pix_fmt(ga_image::PixelFormat::Mono8, None);

    let mut max_value = 0;
    for y in 0..image8.height() {
        let line = image8.line::<u8>(y);
        for value in line {
            max_value = max_value.max(*value);
        }
    }

    // cut the lower 2% of signal to prevent bright background's effect on centroid calculation
    for y in 0..image.height() {
        let line = image8.line_mut::<u8>(y);
        for value in line {
            if *value as i32 <= 2i32 * max_value as i32 / 100 {
                *value = 0;
            } else {
                *value = 0xFF;
            }
        }
    }

    let centroid = Point2::<f64>::from(image8.centroid(None)).cast::<f32>().unwrap();
    let c_int = centroid.cast::<i32>().unwrap();

    // TODO (?): determine the radius with subpixel precision

    let centroid_distances_to_img_boundaries = [
        centroid.x as u32,
        centroid.y as u32,
        image.width() - 1 - centroid.x as u32,
        image.height() - 1 - centroid.y as u32,
    ];

    let mut r_lower_bound = 2;
    let mut r_upper_bound = *centroid_distances_to_img_boundaries.iter().min().unwrap();

    let is_outside_disk = |circle: &[Point2<i32>]| {
        let pixels = image8.pixels::<u8>();
        let vals_per_line = image8.values_per_line::<u8>();
        for point in circle {
            if pixels[point.x as usize + point.y as usize * vals_per_line] != 0 { return false; }
        }
        true
    };

    let min_circle = rasterize_circle(c_int, r_lower_bound as u32);
    if is_outside_disk(&min_circle) { return Err(()); } // disk is less than 2 pixels in radius

    let max_circle = rasterize_circle(c_int, r_upper_bound as u32);
    if !is_outside_disk(&max_circle) { return Err(()); } // disk extends outside the image

    let radius;

    loop {
        let r_delta = (r_upper_bound - r_lower_bound) / 2;
        if r_delta == 0 {
            radius = r_lower_bound;
            break;
        }

        let r_mid = r_lower_bound + r_delta;
        let mid_circle = rasterize_circle(c_int, r_mid);
        if !is_outside_disk(&mid_circle) {
            r_lower_bound = r_mid;
        } else {
            r_upper_bound = r_mid;
        }
    }

    Ok((centroid, (radius * 2) as f32))
}

// Returns circle points clockwise (in a right-handed coordinate system), starting from the leftmost point.
fn rasterize_circle(center: Point2<i32>, radius: u32) -> Vec<Point2<i32>> {
    let mut octant = vec![];

    let mut point = Point2{ x: -(radius as i32), y: 0 };

    // is `Some` if the point having x=y belongs to the circle
    let mut diagonal_point: Option<Point2<i32>> = None;

    while -point.x > point.y {
        point.x += 1;
        point.y += 1;
        if point.x.pow(2) + point.y.pow(2) < radius.pow(2) as i32 {
            point.x -= 1;
        }
        if point.x.abs() == point.y.abs() {
            diagonal_point = Some(point);
        } else {
            octant.push(point);
        }
    }

    let mut points = vec![];

    // Order of filling octants:
    //
    //               y
    //               ^
    //               |
    //         oct2  |  oct3
    //        +      ^       +
    //     oct_1     |     oct4
    // ----+---------0------------+----->x
    //     oct8      |     oct5
    //        +      |       +
    //         oct7  |  oct6
    //               |


    points.push(Point2{ x: -(radius as i32), y: 0 });
    points.extend_from_slice(&octant);                                         // octant 1
    match diagonal_point { Some(ref p) => points.push(*p), _ => () }
    points.extend(octant.iter().rev().map(|p| Point2{ x: -p.y, y: -p.x }));    // octant 2
    points.push(Point2{ x: 0, y: radius as i32 });
    points.extend(octant.iter().map(|p| Point2{ x: p.y, y: -p.x })); // octant 3
    match diagonal_point { Some(ref p) => points.push(Point2{ x: -p.x, y: p.y }), _ => () }
    points.extend(octant.iter().rev().map(|p| Point2{ x: -p.x, y: p.y }));     // octant 4
    points.push(Point2{ x: radius as i32, y: 0 });
    points.extend(octant.iter().map(|p| Point2{ x: -p.x, y: -p.y }));          // octant 5
    match diagonal_point { Some(ref p) => points.push(Point2{ x: -p.x, y: -p.y }), _ => () }
    points.extend(octant.iter().rev().map(|p| Point2{ x: p.y, y: p.x }));      // octant 6
    points.push(Point2{ x: 0, y: -(radius as i32) });
    points.extend(octant.iter().map(|p| Point2{ x: -p.y, y: p.x }));           // octant 7
    match diagonal_point { Some(ref p) => points.push(Point2{ x: p.x, y: -p.y }), _ => () }
    points.extend(octant.iter().rev().map(|p| Point2{ x: p.x, y: -p.y }));     // octant 8

    for p in &mut points { *p += center.to_vec(); }

    points
}
