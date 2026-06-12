// core/src/frame.rs
// Frame type definitions for the Beam SDK.
// RawFrame is a non-owning C-compatible view over camera buffer memory.
// OwnedFrame is the heap-allocated copy used on WASM (mandatory copy boundary).
//
// Architecture constraint:
//   iOS:     CVPixelBuffer locked with .readOnly before RawFrame is created.
//            Unlock immediately after pipeline tick returns.
//   Android: AHardwareBuffer imported or locked — never held across frames.
//   WASM:    OwnedFrame mandatory copy from ImageData; documented as expected cost.

use std::vec::Vec;

/// Pixel format of the frame delivered by the camera HAL.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    /// YUV 4:2:0, semi-planar, UV interleaved. Native ISP output on most SoCs.
    /// Android: detected by planes[1].pixelStride == 2.
    /// iOS: kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange.
    Nv12 = 0,
    /// YUV 4:2:0, planar, UV separate. Older Android HALs.
    /// Android: detected by planes[1].pixelStride == 1.
    Yuv420P = 1,
    /// 8-bit RGBA interleaved. Used on WASM after ImageData conversion.
    Rgba8 = 2,
}

/// Non-owning view of a single camera frame's Y and UV planes.
///
/// # Safety
/// All pointer fields must remain valid for the lifetime of this struct.
/// On iOS: caller must hold CVPixelBufferLockBaseAddress(.readOnly).
/// On Android: caller must hold AHardwareBuffer_lock or GPU delegate import.
/// Callers must NEVER retain this struct across frame boundaries.
#[repr(C)]
pub struct RawFrame {
    /// Pointer to the Y (luma) plane.
    /// Valid for at least `height * y_stride` bytes.
    pub y_plane: *const u8,
    /// Pointer to the UV (chroma) plane.
    /// For NV12: interleaved CbCr, valid for at least `(height/2) * uv_stride` bytes.
    /// For YUV420P: U plane pointer; V plane follows at `(height/2) * uv_stride` offset.
    pub uv_plane: *const u8,
    /// Frame width in pixels.
    pub width: u32,
    /// Frame height in pixels.
    pub height: u32,
    /// Row stride of Y plane in bytes. May be larger than `width` due to hardware alignment.
    pub y_stride: u32,
    /// Row stride of UV plane in bytes.
    pub uv_stride: u32,
    /// Pixel format — determines UV plane layout.
    pub format: PixelFormat,
    /// Frame capture timestamp in microseconds (monotonic clock).
    pub timestamp_us: u64,
}

/// Heap-allocated copy of a camera frame. Used on WASM where zero-copy is not possible.
///
/// # WASM cost note
/// ImageData → OwnedFrame requires a full memcpy of the pixel data.
/// This is an EXPECTED and DOCUMENTED cost on WASM — it is not a bug.
/// The pipeline processes ≤25 frames/second, making this a ~5MB/s copy budget.
pub struct OwnedFrame {
    y_data: Vec<u8>,
    uv_data: Vec<u8>,
    width: u32,
    height: u32,
    y_stride: u32,
    uv_stride: u32,
    format: PixelFormat,
    timestamp_us: u64,
}

impl OwnedFrame {
    /// Create a new OwnedFrame by copying RGBA data (WASM path).
    /// The RGBA → NV12 conversion is performed here so downstream code
    /// always sees a consistent Y-plane regardless of input format.
    pub fn from_rgba(rgba: &[u8], width: u32, height: u32, timestamp_us: u64) -> Self {
        let w = width as usize;
        let h = height as usize;
        let mut y_data = Vec::with_capacity(w * h);
        let mut uv_data = Vec::with_capacity(w * h / 2);

        // BT.601 RGBA → Y conversion
        for row in 0..h {
            for col in 0..w {
                let base = (row * w + col) * 4;
                let r = rgba[base] as f32;
                let g = rgba[base + 1] as f32;
                let b = rgba[base + 2] as f32;
                let y = (0.257 * r + 0.504 * g + 0.098 * b + 16.0).clamp(0.0, 255.0) as u8;
                y_data.push(y);
            }
        }

        // NV12 UV plane (subsampled 2x2)
        for row in (0..h).step_by(2) {
            for col in (0..w).step_by(2) {
                let base = (row * w + col) * 4;
                let r = rgba[base] as f32;
                let g = rgba[base + 1] as f32;
                let b = rgba[base + 2] as f32;
                let cb = (-0.148 * r - 0.291 * g + 0.439 * b + 128.0).clamp(0.0, 255.0) as u8;
                let cr = (0.439 * r - 0.368 * g - 0.071 * b + 128.0).clamp(0.0, 255.0) as u8;
                uv_data.push(cb);
                uv_data.push(cr);
            }
        }

        Self {
            y_data,
            uv_data,
            width,
            height,
            y_stride: width,
            uv_stride: width,
            format: PixelFormat::Nv12,
            timestamp_us,
        }
    }

    /// Borrow as a non-owning RawFrame.
    ///
    /// # Safety
    /// The returned RawFrame borrows from self. Do not drop self while RawFrame is in use.
    pub fn as_raw(&self) -> RawFrame {
        RawFrame {
            y_plane: self.y_data.as_ptr(),
            uv_plane: self.uv_data.as_ptr(),
            width: self.width,
            height: self.height,
            y_stride: self.y_stride,
            uv_stride: self.uv_stride,
            format: self.format,
            timestamp_us: self.timestamp_us,
        }
    }
}
