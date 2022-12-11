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
#version 330 core

#define PI 3.14159265358979323846

in vec2 tex_coord;

uniform sampler2D source_image;
uniform float disk_diameter; // value in pixels
uniform vec2 disk_center; // value in pixels
uniform bool equirectangular; // if false, equal-area projection is used
/// Transformation from normalized (within [-1; 1]) globe coordinates to normalized (within [-1; 1]) image coordinates
/// within the disk; compensates for planet flattening, planet inclination and image roll.
///
/// Globe coordinates: assuming spherical planet centered at (0, 0, 0), X points right, Y points up, Z points
/// to observer. Observer faces latitude 0°, longitude 0°.
///
/// Image disk coordinates: disk center is (0, 0), disk spans [-1; 1] in X, proportionally less in Y (depending
/// on flattening and inclination).
///
uniform mat3 globe_transform;

out vec4 output_color;

void main()
{
    vec2 source_size = vec2(textureSize(source_image, 0));

    float lon = -PI / 2 + tex_coord.x * PI;
    float sin_lat = 0.0;
    float cos_lat = 1.0;

    if (equirectangular)
    {
        float lat = -PI / 2 + tex_coord.y * PI;
        sin_lat = sin(lat);
        cos_lat = cos(lat);
    }
    else
    {
        sin_lat = -1.0 + tex_coord.y * 2.0;
        cos_lat = sqrt(1.0 - sin_lat * sin_lat);
    }

    vec3 globe_pos = vec3(
        cos_lat * sin(lon),
        sin_lat,
        cos_lat * cos(lon)
    );

    vec2 corrected_disk_pos = (globe_transform * globe_pos).xy;

    vec2 image_disk_pos = disk_center / source_size + (corrected_disk_pos * disk_diameter / 2) / source_size;

    output_color = vec4(texture(source_image, image_disk_pos).rgb, 1.0);
}
