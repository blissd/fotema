// SPDX-FileCopyrightText: © 2022-2024 Sophie Herold
// SPDX-FileCopyrightText: © 2023 FineFindus
// SPDX-FileCopyrightText: © 2023 Lubosz Sarnecki
// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::*;
/// GPS code derived from Loupe.
/// See https://gitlab.gnome.org/GNOME/loupe/-/blob/main/src/metadata/gps.rs
use h3o::{CellIndex, LatLng, Resolution};

#[derive(Debug, Clone, Copy)]
pub struct GPSLocation {
    pub latitude: GPSCoord,
    pub longitude: GPSCoord,
}

#[derive(Debug, Clone, Copy)]
pub struct GPSCoord {
    sing: bool,
    deg: f64,
    min: Option<f64>,
    sec: Option<f64>,
}

impl GPSCoord {
    pub fn to_f64(&self) -> f64 {
        let sign = if self.sing { 1. } else { -1. };

        let min = self.min.unwrap_or_default();
        let sec = self.sec.unwrap_or_default();

        sign * (self.deg + min / 60. + sec / 60. / 60.)
    }

    fn latitude_sign(reference: &[Vec<u8>]) -> Option<bool> {
        let reference = reference.first().and_then(|x| x.first())?;
        match reference.to_ascii_uppercase() {
            b'N' => Some(true),
            b'S' => Some(false),
            _ => None,
        }
    }

    fn longitude_sign(reference: &[Vec<u8>]) -> Option<bool> {
        let reference = reference.first().and_then(|x| x.first())?;
        match reference.to_ascii_uppercase() {
            b'E' => Some(true),
            b'W' => Some(false),
            _ => None,
        }
    }

    fn position_exif(position: &[exif::Rational]) -> Option<(f64, Option<f64>, Option<f64>)> {
        let (deg, mut min, mut sec) = (position.first()?, position.get(1), position.get(2));

        if let (Some(min_), Some(sec_)) = (min, sec) {
            if min_.denom > 1 && sec_.num == 0 {
                sec = None;
            }
        }

        if min.is_some_and(|min_| deg.denom > 1 && min_.num == 0) {
            min = None;
        }

        Some((
            deg.to_f64(),
            min.map(exif::Rational::to_f64),
            sec.map(exif::Rational::to_f64),
        ))
    }
}

impl GPSLocation {
    pub fn for_exif(
        latitude: &[exif::Rational],
        latitude_ref: &[Vec<u8>],
        longitude: &[exif::Rational],
        longitude_ref: &[Vec<u8>],
    ) -> Option<Self> {
        let (lat_deg, lat_min, lat_sec) = GPSCoord::position_exif(latitude)?;
        let lat_sign = GPSCoord::latitude_sign(latitude_ref)?;

        let (lon_deg, lon_min, lon_sec) = GPSCoord::position_exif(longitude)?;
        let lon_sign = GPSCoord::longitude_sign(longitude_ref)?;

        Some(Self {
            latitude: GPSCoord {
                sing: lat_sign,
                deg: lat_deg,
                min: lat_min,
                sec: lat_sec,
            },
            longitude: GPSCoord {
                sing: lon_sign,
                deg: lon_deg,
                min: lon_min,
                sec: lon_sec,
            },
        })
    }

    pub fn to_cell_index(&self, resolution: Resolution) -> Result<CellIndex> {
        let ll = LatLng::new(self.latitude.to_f64(), self.longitude.to_f64())?;
        Ok(ll.to_cell(resolution))
    }
}
