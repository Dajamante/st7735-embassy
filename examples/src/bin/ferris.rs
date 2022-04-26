// $ cargo rb ferris
#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use embassy::executor::Spawner;
use embassy::time::{Delay, Duration, Timer};
use embassy_nrf::gpio::{Level, Output, OutputDrive};
use embassy_nrf::{interrupt, spim, Peripherals};
use embedded_graphics::{
    image::{Image, ImageRaw, ImageRawLE},
    pixelcolor::Rgb565,
    prelude::*,
};
use embedded_hal_async::spi::ExclusiveDevice;
use nrf_embassy as _; // global logger + panicking-behavior + memory layout
use st7735_embassy::{self, ST7735};
use tinybmp::Bmp;

#[embassy::main]
async fn main(_spawner: Spawner, p: Peripherals) {
    let mut config = spim::Config::default();
    config.frequency = spim::Frequency::M32;
    let irq = interrupt::take!(SPIM3);
    // spim, irq, sck, mosi or SDA, config
    let spim = spim::Spim::new_txonly(p.SPI3, irq, p.P0_04, p.P0_28, config);
    // sets which slave to use, command section
    let cs_pin = Output::new(p.P0_30, Level::Low, OutputDrive::Standard);
    let spi_dev = ExclusiveDevice::new(spim, cs_pin);

    // do not use the reset of the board!!
    let rst = Output::new(p.P0_31, Level::High, OutputDrive::Standard);
    // data/command selection
    let dc = Output::new(p.P0_29, Level::High, OutputDrive::Standard);

    let mut display = ST7735::new(spi_dev, dc, rst, Default::default(), 160, 128);
    display.init(&mut Delay).await.unwrap();
    display.clear(Rgb565::BLACK).unwrap();

    let raw_image: Bmp<Rgb565> =
        Bmp::from_slice(include_bytes!("../../assets/ferris3.bmp")).unwrap();
    let image = Image::new(&raw_image, Point::new(34, 24));

    //let image_raw: ImageRawLE<Rgb565> = ImageRaw::new(include_bytes!("../../assets/ado1.raw"), 86);
    //let image: Image<_> = Image::new(&image_raw, Point::new(34, 24));
    image.draw(&mut display).unwrap();
    display.flush().await.unwrap();

    // LED
    let mut backlight = Output::new(p.P0_03, Level::High, OutputDrive::Standard);
    loop {
        backlight.set_high();
        Timer::after(Duration::from_millis(1700)).await;
        backlight.set_low();
        Timer::after(Duration::from_millis(300)).await;
    }
}
