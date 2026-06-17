// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Optional VAAPI hardware-accelerated video decoding for thumbnail frame
//! extraction.
//!
//! All `unsafe` FFI lives here so the upstream thumbnailer only needs a tiny,
//! fallback-guarded hook. Every failure path returns to the caller's existing
//! software decode, so enabling this can never make thumbnailing fail — only
//! faster on machines with a working VAAPI driver (Intel/AMD, and NVIDIA via
//! the VA driver).
//!
//! Set the environment variable `FOTEMA_DISABLE_VAAPI` to force software
//! decoding (escape hatch for misbehaving drivers).

use ffmpeg_next as ffmpeg;
use ffmpeg::codec::context::Context;
use ffmpeg::ffi::*;
use ffmpeg::format::Pixel;
use ffmpeg::util::frame::video::Video;

use std::ptr;

use tracing::{debug, warn};

/// `get_format` callback: pick the VAAPI surface format when the decoder offers
/// it, otherwise fall back to the decoder's preferred (software) format.
unsafe extern "C" fn get_vaapi_format(
    _ctx: *mut AVCodecContext,
    formats: *const AVPixelFormat,
) -> AVPixelFormat {
    let vaapi: AVPixelFormat = Pixel::VAAPI.into();
    let none: AVPixelFormat = Pixel::None.into();

    unsafe {
        let mut p = formats;
        while *p != none {
            if *p == vaapi {
                return vaapi;
            }
            p = p.add(1);
        }
        // VAAPI not on offer: let the decoder use its first (software) choice.
        *formats
    }
}

/// Attach a VAAPI hardware device to `ctx` *before* it is opened. Returns true
/// when hardware decoding was set up; false means the caller should decode in
/// software. Must be called before [`Context::decoder`].
pub fn setup_vaapi(ctx: &mut Context) -> bool {
    if std::env::var_os("FOTEMA_DISABLE_VAAPI").is_some() {
        debug!("VAAPI disabled via FOTEMA_DISABLE_VAAPI; using software decode");
        return false;
    }

    unsafe {
        let mut device: *mut AVBufferRef = ptr::null_mut();
        let kind = av_hwdevice_find_type_by_name(c"vaapi".as_ptr());
        let ret = av_hwdevice_ctx_create(&mut device, kind, ptr::null(), ptr::null_mut(), 0);
        if ret < 0 || device.is_null() {
            debug!("VAAPI unavailable (av_hwdevice_ctx_create = {ret}); using software decode");
            return false;
        }

        let raw = ctx.as_mut_ptr();
        (*raw).hw_device_ctx = av_buffer_ref(device);
        (*raw).get_format = Some(get_vaapi_format);

        // The codec context now holds its own reference; drop ours.
        av_buffer_unref(&mut device);

        debug!("VAAPI hardware decode enabled");
        true
    }
}

/// True when `frame` lives in a VAAPI surface and must be copied to system
/// memory before software processing (scaling, PNG encode).
pub fn is_hw_frame(frame: &Video) -> bool {
    frame.format() == Pixel::VAAPI
}

/// Copy a hardware (VAAPI) frame into a freshly allocated software frame
/// (typically NV12 or P010). The returned frame can be fed to swscale.
pub fn transfer_to_software(hw: &Video) -> Result<Video, ffmpeg::Error> {
    let mut sw = Video::empty();
    unsafe {
        let ret = av_hwframe_transfer_data(sw.as_mut_ptr(), hw.as_ptr(), 0);
        if ret < 0 {
            warn!("av_hwframe_transfer_data failed: {ret}");
            return Err(ffmpeg::Error::from(ret));
        }
    }
    Ok(sw)
}
