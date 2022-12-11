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

// looks from (1, 0, 0) at (0, 0, 0) with "up" being (0, 0, 1)
const mat4 VIEW = mat4(
    0, 0, 1, 0,
    1, 0, 0, 0,
    0, 1, 0, 0,
    0, 0, -1, 1
);

// corresponds to orthographic projection with x ∊ [-1, 1], y ∊ [-1, 1], z ∊ [0, 1]
const mat4 PROJECTION = mat4(
    1, 0, 0, 0,
    0, 1, 0, 0,
    0, 0, -2, 0,
    0, 0, -1, 1
);

uniform mat3 globe_orientation;
uniform float zoom;
uniform float wh_ratio;
uniform float flattening;

in vec2 lonlat_position;
out vec2 lonlat_out;

void main()
{
    float longitude = radians(lonlat_position.x);
    float latitude = radians(lonlat_position.y);

    vec3 position = vec3(
        cos(longitude) * cos(latitude),
        sin(longitude) * cos(latitude),
        sin(latitude)
    );

    mat4 view_model = VIEW * mat4(globe_orientation);
    vec4 view_model_position = view_model * vec4(position, 1.0);
    vec4 projected = PROJECTION * view_model_position;

    projected.y *= 1.0 - flattening;

    gl_Position.xy = vec2(projected.x * zoom / wh_ratio, projected.y * zoom);
    gl_Position.zw = projected.zw;
    lonlat_out = lonlat_position;
}
