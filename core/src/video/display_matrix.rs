// SPDX-FileCopyrightText: © 2026 David Bliss
// SPDX-FileCopyrightText: © 2014 Vittorio Giovara <vittorio.giovara@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use byteorder::{ByteOrder, LittleEndian};

/// fixed point to double
fn conv_fp(x: i32) -> f64 {
    x as f64 / 65536.0
}

/// Based on https://ffmpeg.org/doxygen/3.4/display_8c_source.html
/// Extract rotation from the "display matrix" side data.
/// Display matrix is 36 bytes in side data.
pub fn av_display_rotation_get(matrix: &[u8]) -> f64 {
    if matrix.len() != 36 {
        return f64::NAN;
    }

    let matrix: &[i32; 9] = &[
        LittleEndian::read_i32(&matrix[0..4]),
        LittleEndian::read_i32(&matrix[4..8]),
        LittleEndian::read_i32(&matrix[8..12]),
        LittleEndian::read_i32(&matrix[12..16]),
        LittleEndian::read_i32(&matrix[16..20]),
        LittleEndian::read_i32(&matrix[20..24]),
        LittleEndian::read_i32(&matrix[24..28]),
        LittleEndian::read_i32(&matrix[28..32]),
        LittleEndian::read_i32(&matrix[32..36]),
    ];

    let a = conv_fp(matrix[0]);
    let c = conv_fp(matrix[3]);

    if a == 0.0 && c == 0.0 {
        return f64::NAN;
    }

    let mut rotation = f64::atan2(c, a) * 180.0 / std::f64::consts::PI;
    rotation = rotation.round() / 90.0 * 90.0; // Round to nearest 90°
    -rotation
}
