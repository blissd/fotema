// SPDX-FileCopyrightText: © 2026 David Bliss
// SPDX-FileCopyrightText: © 2014 Vittorio Giovara <vittorio.giovara@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use byteorder::{BigEndian, ByteOrder};

/// fixed point to double
fn conv_fp(x: u32) -> f64 {
    x as f64 / (1 << 16) as f64
}

/// Based on https://ffmpeg.org/doxygen/3.4/display_8c_source.html
/// Extract rotation from the "display matrix" side data.
/// Display matrix is 36 bytes in side data.
pub fn av_display_rotation_get(matrix: &[u8]) -> f64 {
    if matrix.len() != 36 {
        return f64::NAN;
    }

    // Convert matrix of 36 bytes to matrix of 9 u32s.
    // u32 is 4 bytes so sizeof(u32) * 9 == 36.
    let matrix: &[u32; 9] = &[
        BigEndian::read_u32(&matrix[0..4]),
        BigEndian::read_u32(&matrix[4..8]),
        BigEndian::read_u32(&matrix[8..12]),
        BigEndian::read_u32(&matrix[12..16]),
        BigEndian::read_u32(&matrix[16..20]),
        BigEndian::read_u32(&matrix[20..24]),
        BigEndian::read_u32(&matrix[24..28]),
        BigEndian::read_u32(&matrix[28..32]),
        BigEndian::read_u32(&matrix[32..36]),
    ];

    let scale: &[f64; 2] = &[
        f64::hypot(conv_fp(matrix[0]), conv_fp(matrix[3])),
        f64::hypot(conv_fp(matrix[1]), conv_fp(matrix[4])),
    ];

    if scale[0] == 0.0 || scale[1] == 0.0 {
        return f64::NAN;
    }

    let rotation = f64::atan2(conv_fp(matrix[1]) / scale[1], conv_fp(matrix[0]) / scale[0]) * 180.
        / std::f64::consts::PI;

    return rotation;
}
