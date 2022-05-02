// $ cargo rb ferris
#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use defmt::info;
use embassy::blocking_mutex::raw::NoopRawMutex;
use embassy::channel::channel::{Channel, Receiver, Sender};
use embassy::executor::Spawner;
use embassy::time::{Delay, Duration, Timer};
use embassy::util::Forever;
use embassy_nrf::gpio::{Input, Pin, Pull};
use embassy_nrf::gpio::{Level, Output, OutputDrive};
use embassy_nrf::gpiote::{InputChannel, InputChannelPolarity};
use embassy_nrf::{interrupt, spim, Peripherals};
use embedded_graphics::{image::Image, pixelcolor::Rgb565, prelude::*};
use embedded_hal_async::spi::ExclusiveDevice;
use nrf_embassy::{self as _, unwrap}; // global logger + panicking-behavior + memory layout
use st7735_embassy::{self, ST7735};
use tinybmp::Bmp;

enum Moves {
    Up,
    Down,
    Right,
    Left,
}

#[embassy::main]
async fn main(spawner: Spawner, p: Peripherals) {
    // Buttons configuration
    let btn1 = Input::new(p.P0_11.degrade(), Pull::Up);
    let btn2 = Input::new(p.P0_12.degrade(), Pull::Up);
    let btn3 = Input::new(p.P0_24.degrade(), Pull::Up);
    let btn4 = Input::new(p.P0_25.degrade(), Pull::Up);

    // SPI configuration
    let mut config = spim::Config::default();
    config.frequency = spim::Frequency::M32;
    let irq = interrupt::take!(SPIM3);
    // spim args: spi instance, irq, sck, mosi/SDA, config
    let spim = spim::Spim::new_txonly(p.SPI3, irq, p.P0_04, p.P0_28, config);
    // cs_pin: chip select pin
    let cs_pin = Output::new(p.P0_30, Level::Low, OutputDrive::Standard);
    let spi_dev = ExclusiveDevice::new(spim, cs_pin);

    // rst:  display reset pin, managed at driver level
    let rst = Output::new(p.P0_31, Level::High, OutputDrive::Standard);
    // dc: data/command selection pin, managed at driver level

    let dc = Output::new(p.P0_29, Level::High, OutputDrive::Standard);

    let mut display = ST7735::new(spi_dev, dc, rst, Default::default(), 160, 128);
    display.init(&mut Delay).await.unwrap();
    display.clear(Rgb565::BLACK).unwrap();

    let raw_image_front: Bmp<Rgb565> =
        Bmp::from_slice(include_bytes!("../../assets/ferris_fr.bmp")).unwrap();
    let mut start_point = Point { x: 32, y: 24 };
    let mut image_front = Image::new(&raw_image_front, start_point);

    let raw_image_back: Bmp<Rgb565> =
        Bmp::from_slice(include_bytes!("../../assets/ferris_bk.bmp")).unwrap();

    image_front.draw(&mut display).unwrap();
    display.flush().await.unwrap();

    // LED is set to max, but can be modulated with pwm to change backlight brightness
    let mut backlight = Output::new(p.P0_03, Level::High, OutputDrive::Standard);

    backlight.set_high();

    loop {
        if let Some(event) = recv.recv().await {
            match event {
                ButtonEvent::Pressed(id) => {
                    display.clear(Rgb565::BLACK).unwrap();
                    match id {
                        1 => {}
                        2 => {
                            info!("Button 2 pressed");
                            Timer::after(Duration::from_millis(100)).await;
                            start_point.x -= 10;
                            image_front = Image::new(&raw_image_front, start_point);
                            image_front.draw(&mut display).unwrap();
                        }

                        3 => {
                            info!("Button 3 pressed");
                            Timer::after(Duration::from_millis(100)).await;
                            start_point.y -= 10;
                            image_front = Image::new(&raw_image_back, start_point);
                            image_front.draw(&mut display).unwrap();
                        }

                        4 => {
                            info!("Button 4 pressed");
                            Timer::after(Duration::from_millis(100)).await;
                            start_point.y += 10;
                            image_front = Image::new(&raw_image_front, start_point);
                            image_front.draw(&mut display).unwrap();
                        }
                    }
                    display.flush().await.unwrap();
                }
                ButtonEvent::Released(id) => {
                    info!("Btn {} released", id);
                }
            }
        }
    }
}
