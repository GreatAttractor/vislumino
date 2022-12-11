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

in vec2 position;
out vec2 tex_coord;

void main()
{
    // apply texture coords (0,1)-(1,0) to unit quad (-1,-1)-(1,1)
    tex_coord.xy = position.xy / 2 + vec2(0.5, 0.5);

    gl_Position.x = position.x;
    gl_Position.y = -position.y;
    gl_Position.z = 0.0;
    gl_Position.w = 1.0;
}
