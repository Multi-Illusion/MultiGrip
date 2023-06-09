#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

extern crate alloc;
use core::mem;
use alloc::{boxed::Box, vec};
use alloc_cortex_m::CortexMHeap;
use {defmt_rtt as _, panic_probe as _};
use cortex_m::peripheral::NVIC;

use embassy_executor::Spawner;
use embassy_stm32::{
    spi::Spi,
    dma::NoDma,
    time::Hertz,
    executor::InterruptExecutor,
    pac::Interrupt, 
    exti::ExtiInput,
    interrupt,
    gpio::{
        Output,
        Level,
        Speed,
        Input,
        Pull
    },
};
use embassy_sync::{signal::Signal, blocking_mutex::raw::CriticalSectionRawMutex};
use embassy_time::{
    Duration,
    Timer
};


// Module Tree
mod config;
mod slint_comp;
mod event;
mod algorithm;
mod driver {
    pub mod st7789;
}

use config::*;
use event::*;

// 临时


slint::include_modules!();

#[global_allocator]
static ALLOCATOR: CortexMHeap = CortexMHeap::empty();

static mut HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];
static EXECUTOR_KEY: InterruptExecutor = InterruptExecutor::new();
static EXECUTOR_DISPLAY: InterruptExecutor = InterruptExecutor::new();

static KEY_SIGAL: Signal<CriticalSectionRawMutex, KeyEvent> = Signal::new();
static HEART_RATE_SIGAL: Signal<CriticalSectionRawMutex, i32> = Signal::new();

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    unsafe { ALLOCATOR.init(&mut HEAP as *const u8 as usize, core::mem::size_of_val(&HEAP)); };
    
    let p = embassy_stm32::init(rcc_init());
    // unsafe { 
    //     pac::RCC.apb1enr().modify(|w| w.set_pwren(true));
    // }

    let mut nvic: NVIC = unsafe { mem::transmute(()) };
    unsafe { nvic.set_priority(Interrupt::UART4, 6 << 4); };
    let executor_spawner = EXECUTOR_KEY.start(Interrupt::UART4);
    executor_spawner.spawn(up_task(p.PF2, p.EXTI2)).ok();
    executor_spawner.spawn(press_task(p.PF1, p.EXTI1)).ok();
    executor_spawner.spawn(down_task(p.PF0, p.EXTI0)).ok();
    
    unsafe { nvic.set_priority(Interrupt::UART5, 7 << 4); };
    let executor_spawner = EXECUTOR_DISPLAY.start(Interrupt::UART5);
    executor_spawner.spawn(display_task(
        p.SPI1,
        p.PA5,
        p.PA7,
        p.DMA2_CH3,
        p.PD14,
        p.PD15,
        p.PF12,
        p.PF13
    )).ok();

    spawner.spawn(max30102_task(p.I2C1,p.PB8,p.PB9)).ok();
    spawner.spawn(mpu6050_task(p.I2C2,p.PB10,p.PB11)).ok();

    let mut led = Output::new(p.PB7, Level::Low, Speed::Low);
    loop {
        led.toggle();
        Timer::after(Duration::from_millis(200)).await;
    }

}


#[embassy_executor::task]
async fn display_task(
    spi_p: display::Spi,
    sck_pin: display::SckPin,
    mosi_pin: display::MosiPin,
    dma: display::DMA,
    rst_pin: display::RstPin,
    dc_pin: display::DcPin,
    cs_pin: display::CsPin,
    bl_pin: display::BlPin
) {
    use slint::platform::software_renderer::MinimalSoftwareWindow;
    use embedded_graphics::primitives::Rectangle;
    use embedded_graphics::prelude::Point;
    use embedded_graphics::geometry::Size;

    use driver::st7789::*;
    use slint_comp::*;

    let spi = Spi::new_txonly(
        spi_p,
        sck_pin,
        mosi_pin,
        dma,
        NoDma,
        Hertz(display::SPI_FREQUENCY),
        display::spi_config(),
    );
    let _cs = Output::new(cs_pin, Level::Low, Speed::Low);

    let mut display = ST7789::new(
        spi,
        Output::new(dc_pin, Level::Low, Speed::VeryHigh),
        Output::new(rst_pin, Level::Low, Speed::Low),
        Output::new(bl_pin, Level::Low, Speed::VeryHigh),
    );
    display.init(&mut embassy_time::Delay);

    let window = MinimalSoftwareWindow::<2>::new();
    slint::platform::set_platform(Box::new(
        WrapPlatform { window: window.clone() }
    )).unwrap();
    
    let ui = MultiGripWindow::new();

    let mut pixel_buffer = [DoubleU8Pixel(0,0); DISPLAY_WIDTH * DISPLAY_HEIGHT];

    let buffer_ptr = pixel_buffer.as_ptr() as *const u8;
    let bytes = DISPLAY_WIDTH * DISPLAY_HEIGHT * 2;
    let buffer_u8_slice = unsafe {core::slice::from_raw_parts(buffer_ptr, bytes)};

    window.set_size(slint::PhysicalSize::new(DISPLAY_WIDTH as _, DISPLAY_HEIGHT as _));
    let screen_area = Rectangle::new(
        Point::new(DISPLAY_WIDTH_OFFSET as _, DISPLAY_HEIGHT_OFFSET as _),
        Size::new(DISPLAY_WIDTH as _, DISPLAY_HEIGHT as _)
    );

    loop {
        if KEY_SIGAL.signaled() {
            match KEY_SIGAL.wait().await {
                KeyEvent::Up => {
                    ui.invoke_trigger_up_key();
                },
                KeyEvent::Down => {
                    ui.invoke_trigger_down_key();
                },
                KeyEvent::Press => {
                    ui.invoke_trigger_press_key();
                }
            }
        }
        if HEART_RATE_SIGAL.signaled() {
            ui.set_heart_rate(HEART_RATE_SIGAL.wait().await as _);
        }

        slint::platform::update_timers_and_animations();
        if window.draw_if_needed(|renderer| {
            renderer.render(&mut pixel_buffer, DISPLAY_WIDTH);
        }) {
            display.fill_continuous(&screen_area, buffer_u8_slice).await;
        } else {
            Timer::after(Duration::from_millis(10)).await;
        }
    }
}

#[embassy_executor::task]
async fn up_task(key_pin: key::UP_PIN, key_exti: key::UP_EXTI) {
    use event::KeyState;
    use config::key::*;

    let key = Input::new(key_pin, Pull::Up);
    let mut key = ExtiInput::new(key, key_exti);
    let mut key_state = KeyState::Waiting;
    let mut time_counter = 0;
    
    loop {
        match key_state {
            KeyState::Waiting => {
                key.wait_for_falling_edge().await;
                Timer::after(Duration::from_millis(SCAN_INTV)).await;
                if key.is_low() {
                    KEY_SIGAL.signal(KeyEvent::Up);
                }
                key_state = KeyState::Pressed;
            },
            KeyState::Pressed => {
                if key.is_low() {
                    time_counter += 1;
                    if time_counter * SCAN_INTV > 400 {
                        KEY_SIGAL.signal(KeyEvent::Up);
                        time_counter = 0;
                    }
                    Timer::after(Duration::from_millis(SCAN_INTV)).await;
                }
                else {
                    key_state = KeyState::Waiting;
                }
            }
        }
    }
}

#[embassy_executor::task]
async fn down_task(key_pin: key::DOWN_PIN, key_exti: key::DOWN_EXTI) {
    use event::KeyState;
    use config::key::*;

    let key = Input::new(key_pin, Pull::Up);
    let mut key = ExtiInput::new(key, key_exti);
    let mut key_state = KeyState::Waiting;
    let mut time_counter = 0;
    
    loop {
        match key_state {
            KeyState::Waiting => {
                key.wait_for_falling_edge().await;
                Timer::after(Duration::from_millis(SCAN_INTV)).await;
                if key.is_low() {
                    KEY_SIGAL.signal(KeyEvent::Down);
                }
                key_state = KeyState::Pressed;
            },
            KeyState::Pressed => {
                if key.is_low() {
                    time_counter += 1;
                    if time_counter * SCAN_INTV > 400 {
                        KEY_SIGAL.signal(KeyEvent::Down);
                        time_counter = 0;
                    }
                    Timer::after(Duration::from_millis(SCAN_INTV)).await;
                }
                else {
                    key_state = KeyState::Waiting;
                }
            }
        }
    }
}


#[embassy_executor::task]
async fn press_task(key_pin: key::PRESS_PIN, key_exti: key::PRESS_EXTI) {
    use config::key::*;

    let key = Input::new(key_pin, Pull::Up);
    let mut key = ExtiInput::new(key, key_exti);
    
    loop {
        key.wait_for_falling_edge().await;
        Timer::after(Duration::from_millis(SCAN_INTV)).await;
        if key.is_low() {
            KEY_SIGAL.signal(KeyEvent::Press);
        }
    }
}

#[embassy_executor::task]
async fn max30102_task(
    i2c_p: max30102_config::I2C,
    scl_pin: max30102_config::SCL_PIN,
    sda_pin: max30102_config::SDA_PIN,
) {
    use max3010x::Max3010x;

    let irq = interrupt::take!(I2C1_EV);
    let mut i2c = embassy_stm32::i2c::I2c::new(
        i2c_p,
        scl_pin,
        sda_pin,
        irq,
        NoDma,
        NoDma,
        Hertz(400_000),
        Default::default(),
    );
    let i2c = embassy_stm32::i2c::TimeoutI2c::new(&mut i2c, Duration::from_millis(1000));

    let mut sensor = Max3010x::new_max30102(i2c);
    
    sensor.reset().ok();
    Timer::after(Duration::from_millis(100)).await;

    let mut sensor = sensor.into_heart_rate().unwrap();
    
    sensor.enable_fifo_rollover().unwrap();
    sensor.set_pulse_amplitude(max3010x::Led::All, 15).unwrap();
    sensor.set_sample_averaging(max3010x::SampleAveraging::Sa4).unwrap();
    sensor.set_sampling_rate(max3010x::SamplingRate::Sps100).unwrap();
    sensor.set_pulse_width(max3010x::LedPulseWidth::Pw411).unwrap();

    sensor.clear_fifo().unwrap();
    let mut datas = [0; 100];

    loop {
        Timer::after(Duration::from_millis(1000)).await;
        let mut a = sensor.read_fifo(&mut datas[0..25]).unwrap();
        Timer::after(Duration::from_millis(1000)).await;
        a += sensor.read_fifo(&mut datas[25..50]).unwrap();
        Timer::after(Duration::from_millis(1000)).await;
        a += sensor.read_fifo(&mut datas[50..75]).unwrap();
        Timer::after(Duration::from_millis(1000)).await;
        a += sensor.read_fifo(&mut datas[75..100]).unwrap();
        HEART_RATE_SIGAL.signal(algorithm::cal_heart_rate(&datas, a as _));
        // HEART_RATE_SIGAL.signal(a as _);
    }
}

#[embassy_executor::task]
async fn mpu6050_task(
    i2c_p: mpu6050_config::I2C,
    scl_pin: mpu6050_config::SCL_PIN,
    sda_pin: mpu6050_config::SDA_PIN,
) {
    use mpu6050::Mpu6050;

    let irq = interrupt::take!(I2C2_EV);
    let mut i2c = embassy_stm32::i2c::I2c::new(
        i2c_p,
        scl_pin,
        sda_pin,
        irq,
        NoDma,
        NoDma,
        Hertz(100_000),
        Default::default(),
    );
    let i2c = embassy_stm32::i2c::TimeoutI2c::new(
        &mut i2c,
        Duration::from_millis(1000)
    );

    let mut sensor = Mpu6050::new(i2c);
    sensor.init(&mut embassy_time::Delay).ok();

    loop {
        Timer::after(Duration::from_millis(1000)).await;
        // let acc = sensor.get_acc().unwrap().data.0[0];
        // let gyro = sensor.get_gyro().unwrap().data.0[0];
        let temp = sensor.get_temp().unwrap();
        HEART_RATE_SIGAL.signal((temp * 1000.0) as _);
    }
}


#[interrupt]
unsafe fn UART4() {
    EXECUTOR_KEY.on_interrupt()
}

#[interrupt]
unsafe fn UART5() {
    EXECUTOR_DISPLAY.on_interrupt()
}