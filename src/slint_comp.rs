use alloc::rc::Rc;
use slint::platform::{
    Platform,
    software_renderer::{
        MinimalSoftwareWindow,
        TargetPixel,
        PremultipliedRgbaColor
    }
};

// DoubleU8Pixel RGB color mask.
const R_MASK: u8 = 0b11111000;
const GH_MASK: u8 = 0b00000111; //High-order byte mask
const GL_MASK: u8 = 0b11100000; //Low-order byte mask
const B_MASK: u8 = 0b00011111;

/// Splitting a 16-bit Rgb565 pixel into two 8-bit combined pixel
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct DoubleU8Pixel(pub u8, pub u8);

impl TargetPixel for DoubleU8Pixel {
    fn blend(&mut self, color: PremultipliedRgbaColor) {
        let a = (u8::MAX - color.alpha) as u32;
        // convert to 5 bits
        let a = (a + 4) >> 3;

        // 00000ggg_ggg00000_rrrrr000_000bbbbb
        let expanded = (((self.0 & R_MASK) as u32) << 8) | ((self.1 & B_MASK) as u32)
            | (((self.0 & GH_MASK) as u32) << 24) | (((self.1 & GL_MASK) as u32) << 16);

        // gggggggg_000rrrrr_rrr000bb_bbbbbb00
        let c =
            ((color.red as u32) << 13) | ((color.green as u32) << 24) | ((color.blue as u32) << 2);
        // gggggg00_000rrrrr_000000bb_bbb00000
        let c = c & 0b11111100_00011111_00000011_11100000;

        let res = expanded * a + c;

        self.0 = (((res >> 13) as u8) & R_MASK) | (((res >> 29) as u8) & GH_MASK);
        self.1 = (((res >> 5) as u8) & B_MASK) | (((res >> 21) as u8) & GL_MASK);
    }

    fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self((r & R_MASK) | (g >> 5), ((g << 3) & GL_MASK) | (b >> 3))
    }
}


pub struct WrapPlatform {
    pub window: Rc<MinimalSoftwareWindow<2>>,
}

impl Platform for WrapPlatform {
    fn create_window_adapter(&self) -> Rc<dyn slint::platform::WindowAdapter> {
        self.window.clone()
    }

    fn duration_since_start(&self) -> core::time::Duration {
        let now = embassy_time::Instant::now();
        core::time::Duration::from_micros(now.as_micros())
    }
}