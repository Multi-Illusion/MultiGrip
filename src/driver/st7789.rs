use embedded_hal::{
    digital::v2::OutputPin,
    blocking::delay::DelayUs
};
use embedded_graphics::primitives::Rectangle;
use embassy_stm32::{
    spi::Spi,
    peripherals::{
        SPI1, DMA2_CH3
    },
    dma::NoDma
};

pub struct ST7789<'a, DC, RST, BL>
where
    DC: OutputPin,
    RST: OutputPin,
    BL: OutputPin,
{
    spi: Spi<'a, SPI1, DMA2_CH3, NoDma>,
    dc: DC,
    // Reset pin.
    rst: RST,
    // Backlight pin,
    bl: BL,
}

impl<'a, DC, RST, BL> ST7789<'a, DC, RST, BL>
where
    DC: OutputPin,
    RST: OutputPin,
    BL: OutputPin,
{
    pub fn new(spi: Spi<'a, SPI1, DMA2_CH3, NoDma>, dc: DC, rst: RST, bl: BL) -> Self {
        Self {
            spi,
            dc,
            rst,
            bl,
        }
    }

    pub fn block_write_command(&mut self, command: Instruction) {
        self.dc.set_low().ok();
        self.spi.blocking_write(&[command as u8]).ok();
    }

    pub fn block_write_data(&mut self, data: &[u8]) {
        self.dc.set_high().ok();
        self.spi.blocking_write(data).ok();
    }

    pub fn init(&mut self, delay_source: &mut impl DelayUs<u32>) {
        self.hard_reset(delay_source);
        self.bl.set_low().ok();
        delay_source.delay_us(10_000);
        self.bl.set_high().ok();

        self.block_write_command(Instruction::SWRESET);
        delay_source.delay_us(150_000);
        self.block_write_command(Instruction::SLPOUT);
        delay_source.delay_us(10_000);
        self.block_write_command(Instruction::INVOFF);
        self.block_write_command(Instruction::VSCRDER);
        self.block_write_data(&[0,0,0x14,0,0,0]);
        self.block_write_command(Instruction::MADCTL);
        self.block_write_data(&[0b0000_0000]);
        self.block_write_command(Instruction::COLMOD);
        self.block_write_data(&[0b0101_0101]);
        self.block_write_command(Instruction::INVON);
        delay_source.delay_us(10_000);
        self.block_write_command(Instruction::NORON); // turn on display
        delay_source.delay_us(10_000);
        self.block_write_command(Instruction::DISPON); // turn on display
        delay_source.delay_us(10_000);

        self.block_write_command(Instruction::MADCTL);
        self.block_write_data(&[0b0000_0000]);

        self.set_tearing_effect();
    }

    fn hard_reset(&mut self, delay_source: &mut impl DelayUs<u32>) {
        
        self.rst.set_high().ok();
        delay_source.delay_us(10); // ensure the pin change will get registered
        self.rst.set_low().ok();
        delay_source.delay_us(10); // ensure the pin change will get registered
        self.rst.set_high().ok();
        delay_source.delay_us(10); // ensure the pin change will get registered
    }

    fn set_address_window(
        &mut self,
        sx: u16,
        sy: u16,
        ex: u16,
        ey: u16,
    ) {
        self.block_write_command(Instruction::CASET);
        self.block_write_data(&sx.to_be_bytes());
        self.block_write_data(&ex.to_be_bytes());
        self.block_write_command(Instruction::RASET);
        self.block_write_data(&sy.to_be_bytes());
        self.block_write_data(&ey.to_be_bytes());
    }

    pub async fn fill_continuous(&mut self, area: &Rectangle, datas: &[u8]) {
        if let Some(bottom_right) = area.bottom_right() {
            let sx = area.top_left.x as u16;
            let sy = area.top_left.y as u16;
            let ex = bottom_right.x as u16;
            let ey = bottom_right.y as u16;
            self.set_pixels(sx, sy, ex, ey, datas).await;
        }
    }

    async fn set_pixels(&mut self, sx: u16, sy: u16, ex: u16, ey: u16, datas: &[u8]) {
        self.set_address_window(sx, sy, ex, ey);
        self.block_write_command(Instruction::RAMWR);
        self.dc.set_high().ok();
        let a_datas = &datas[0..datas.len()/2];
        let b_datas = &datas[datas.len()/2..datas.len()];
        
        self.spi.write(a_datas).await.ok();
        self.spi.write(b_datas).await.ok();
        self.block_write_command(Instruction::NOP);

    }
    
    fn set_tearing_effect(&mut self) {
        self.block_write_command(Instruction::TEON);
        self.block_write_data(&[1])
    }
}

/// ST7789 instructions.
#[allow(dead_code)]
#[repr(u8)]
pub enum Instruction {
    NOP = 0x00,
    SWRESET = 0x01,
    RDDID = 0x04,
    RDDST = 0x09,
    SLPIN = 0x10,
    SLPOUT = 0x11,
    PTLON = 0x12,
    NORON = 0x13,
    INVOFF = 0x20,
    INVON = 0x21,
    DISPOFF = 0x28,
    DISPON = 0x29,
    CASET = 0x2A,
    RASET = 0x2B,
    RAMWR = 0x2C,
    RAMRD = 0x2E,
    PTLAR = 0x30,
    VSCRDER = 0x33,
    TEOFF = 0x34,
    TEON = 0x35,
    MADCTL = 0x36,
    VSCAD = 0x37,
    COLMOD = 0x3A,
    VCMOFSET = 0xC5,
}