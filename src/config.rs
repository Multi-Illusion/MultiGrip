use embassy_stm32::time::Hertz;

pub const DISPLAY_WIDTH: usize = 172;
pub const DISPLAY_HEIGHT: usize = 320;
pub const DISPLAY_WIDTH_OFFSET: usize = 34;
pub const DISPLAY_HEIGHT_OFFSET: usize = 0;

pub const HEAP_SIZE: usize = 30 * 1024;

pub fn rcc_init() -> embassy_stm32::Config {
    let mut config = embassy_stm32::Config::default();
    config.rcc.hse = Some(Hertz(8_000_000));
    config.rcc.sys_ck = Some(Hertz(180_000_000));
    config.rcc.hclk = Some(Hertz(180_000_000));
    config.rcc.pclk1 = Some(Hertz(45_000_000));
    config.rcc.pclk2 = Some(Hertz(90_000_000));

    config
}

pub mod display {
    use embassy_stm32::peripherals;
    use embassy_stm32::spi;

    pub use peripherals::SPI1 as Spi;
    pub use peripherals::PA5 as SckPin;
    pub use peripherals::PA7 as MosiPin;
    pub use peripherals::DMA2_CH3 as DMA;
    pub use peripherals::PD14 as RstPin;
    pub use peripherals::PD15 as DcPin;
    pub use peripherals::PF12 as CsPin;
    pub use peripherals::PF13 as BlPin;

    pub const SPI_FREQUENCY: u32 = 90_000_000;

    pub fn spi_config() -> spi::Config {
        let mut config = spi::Config::default();
        config.mode.polarity = spi::Polarity::IdleHigh;
        config.mode.phase = spi::Phase::CaptureOnSecondTransition;
        config.bit_order = spi::BitOrder::MsbFirst;

        config
    }
}

pub mod key {
    use embassy_stm32::peripherals;
    pub const SCAN_INTV: u64 = 100;

    pub use peripherals::PF2 as UP_PIN;
    pub use peripherals::PF1 as PRESS_PIN;
    pub use peripherals::PF0 as DOWN_PIN;

    pub use peripherals::EXTI2 as UP_EXTI;
    pub use peripherals::EXTI1 as PRESS_EXTI;
    pub use peripherals::EXTI0 as DOWN_EXTI;
}

pub mod max30102_config {
    use embassy_stm32::peripherals;

    pub use peripherals::I2C1 as I2C;
    pub use peripherals::PB9 as SDA_PIN;
    pub use peripherals::PB8 as SCL_PIN;
}

pub mod mpu6050_config {
    use embassy_stm32::peripherals;

    pub use peripherals::I2C2 as I2C;
    pub use peripherals::PB11 as SDA_PIN;
    pub use peripherals::PB10 as SCL_PIN;
}