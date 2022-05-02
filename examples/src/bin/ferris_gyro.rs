// $ cargo rb ferris
#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use nrf_embassy as _; // global logger + panicking-behavior + memory layout

use defmt::{info, Format};
use embassy::blocking_mutex::raw::ThreadModeRawMutex;
use embassy::channel::channel::{Channel, Receiver, Sender};
use embassy::executor::Spawner;
use embassy::time::{Delay, Duration, Timer};
use embassy::util::Forever;
use embassy_nrf::gpio::{AnyPin, Input, Level, Output, OutputDrive, Pin, Pull};
use embassy_nrf::gpiote::{InputChannel, InputChannelPolarity};
use embassy_nrf::twim::{self, Twim};
use embassy_nrf::{interrupt, peripherals::TWISPI0, spim, Peripherals};

use embedded_graphics::{image::Image, pixelcolor::Rgb565, prelude::*, primitives::{Circle,  PrimitiveStyle}};
use embedded_hal_async::spi::ExclusiveDevice;
use st7735_embassy::{self, ST7735};
use tinybmp::Bmp;

use mpu6050_async::*;

#[embassy::main]
async fn main(spawner: Spawner, p: Peripherals) {
    // SPI configuration
    let mut config_spi = spim::Config::default();
    config_spi.frequency = spim::Frequency::M32;
    let irq = interrupt::take!(SPIM3);
    // spim args: spi instance, irq, sck, mosi/SDA, config
    let spim = spim::Spim::new_txonly(p.SPI3, irq, p.P0_26, p.P0_27, config_spi);
    // cs_pin: chip select pin
    let cs_pin = Output::new(p.P0_30, Level::Low, OutputDrive::Standard);
    let spi_dev = ExclusiveDevice::new(spim, cs_pin);

    // rst:  display reset pin, managed at driver level
    let rst = Output::new(p.P0_31, Level::High, OutputDrive::Standard);
    // dc: data/command selection pin, managed at driver level

    let dc = Output::new(p.P0_29, Level::High, OutputDrive::Standard);

    // I2C config
    let config_i2c = twim::Config::default();
    let irq = interrupt::take!(SPIM0_SPIS0_TWIM0_TWIS0_SPI0_TWI0);
    let i2c = Twim::new(p.TWISPI0, irq, p.P0_04, p.P0_28, config_i2c);

    let mut mpu = Mpu6050::new(i2c);
    mpu.init(&mut Delay).await.unwrap();

    // Config display
    let mut display = ST7735::new(spi_dev, dc, rst, Default::default(), 160, 128);
    display.init(&mut Delay).await.unwrap();
    display.clear(Rgb565::BLACK).unwrap();

    let raw_image_front: Bmp<Rgb565> =
        Bmp::from_slice(include_bytes!("../../assets/ferris_fr.bmp")).unwrap();
    let mut start_point = Point { x: 32, y: 24 };
    let mut image = Image::new(&raw_image_front, start_point);

    let raw_image_back: Bmp<Rgb565> =
        Bmp::from_slice(include_bytes!("../../assets/ferris_back12.bmp")).unwrap();
    
    let raw_image_blink: Bmp<Rgb565> =
        Bmp::from_slice(include_bytes!("../../assets/ferris_blink.bmp")).unwrap();

    image.draw(&mut display).unwrap();
    display.flush().await.unwrap();

    // LED is set to max, but can be modulated with pwm to change backlight brightness
    let mut backlight = Output::new(p.P0_03, Level::High, OutputDrive::Standard);

    backlight.set_high();
    let mut old_roll = 0.0;
    let mut old_pitch = 0.0;
    let mut count_to_blink = 0;
    loop {
        count_to_blink += 1;
        // Get gyro data, scaled with sensitivity
        let gyro = mpu.get_gyro().await.unwrap();
        //info!("gyro: {:?}", gyro);
        let acc = mpu.get_acc_angles().await.unwrap();
        info!("r/p: {:?}", acc);
        let roll = (acc.0 * 2.0) as i32;
        let pitch = (acc.1 * 2.0) as i32;
        start_point.x = (start_point.x - roll) % 160;
        start_point.y = (start_point.y + pitch) % 128;

        if acc.1 >= 0.0 {
            image = Image::new(&raw_image_front, start_point);
            if count_to_blink >= 50 {
                Circle::new(Point::new(10, 20), 30)
                    .translate(Point::new(start_point.x, start_point.y))
                    .into_styled(PrimitiveStyle::with_fill(Rgb565::BLUE))
                    .draw(&mut display).unwrap();
                image.draw(&mut display).unwrap();
                display.flush().await.unwrap();
                
            }
            
        } else  {
            image = Image::new(&raw_image_back, start_point);
        }  
        display.clear(Rgb565::BLACK).unwrap();
        image.draw(&mut display).unwrap();
        display.flush().await.unwrap();
    }
}
