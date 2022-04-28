// $ cargo rb ferris
#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use defmt::{info, Format};
use embassy::blocking_mutex::raw::{ThreadModeRawMutex};
use embassy::channel::channel::{Channel, Receiver, Sender};
use embassy::executor::Spawner;
use embassy::time::{Delay, Duration, Timer};
use embassy::util::Forever;
use embassy_nrf::gpio::{AnyPin, Input, Pin, Pull};
use embassy_nrf::gpio::{Level, Output, OutputDrive};
use embassy_nrf::gpiote::{InputChannel, InputChannelPolarity};
use embassy_nrf::{interrupt, spim, Peripherals};
use embedded_graphics::{image::Image, pixelcolor::Rgb565, prelude::*};
use embedded_hal_async::spi::ExclusiveDevice;
use nrf_embassy::{self as _}; // global logger + panicking-behavior + memory layout
use st7735_embassy::{self, ST7735};
use tinybmp::Bmp;

static CHANNEL: Channel<ThreadModeRawMutex, ButtonEvent, 1> = Channel::new();

#[embassy::task(pool_size = 4)]
async fn button_task(
    sender: Sender<'static, ThreadModeRawMutex, ButtonEvent, 1>,
    id: Button,
    mut pin: Input<'static, AnyPin>,
) {
    loop {
        pin.wait_for_low().await;
        Timer::after(Duration::from_millis(25)).await; // Debounce
        if pin.is_low() {
            let _ = sender.send(ButtonEvent::Pressed(id)).await;
            pin.wait_for_high().await;
            let _ = sender.send(ButtonEvent::Released(id)).await;
        }
    }
}

#[embassy::main]
async fn main(spawner: Spawner, p: Peripherals) {
    // Channel
    let sender = CHANNEL.sender();
    let mut receiver = CHANNEL.receiver();
    // Buttons configuration
    let btn1 = Input::new(p.P0_11.degrade(), Pull::Up);
    let btn2 = Input::new(p.P0_12.degrade(), Pull::Up);
    let btn3 = Input::new(p.P0_24.degrade(), Pull::Up);
    let btn4 = Input::new(p.P0_25.degrade(), Pull::Up);

    spawner
        .spawn(button_task(sender.clone(), Button::Left, btn1))
        .unwrap();
    spawner
        .spawn(button_task(sender.clone(), Button::Right, btn2))
        .unwrap();
    spawner
        .spawn(button_task(sender.clone(), Button::Up, btn3))
        .unwrap();
    spawner
        .spawn(button_task(sender.clone(), Button::Down, btn4))
        .unwrap();

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
    let mut image = Image::new(&raw_image_front, start_point);

    let raw_image_back: Bmp<Rgb565> =
        Bmp::from_slice(include_bytes!("../../assets/ferris_back1.bmp")).unwrap();

    image.draw(&mut display).unwrap();
    display.flush().await.unwrap();

    // LED is set to max, but can be modulated with pwm to change backlight brightness
    let mut backlight = Output::new(p.P0_03, Level::High, OutputDrive::Standard);

    backlight.set_high();
    let mut is_turned = false;
    loop {
        if let event = receiver.recv().await {
            match event {
                ButtonEvent::Pressed(id) => {
                    display.clear(Rgb565::BLACK).unwrap();

                    match id {
                        Button::Right => {
                            info!("Button 1 pressed");
                            //Timer::after(Duration::from_millis(100)).await;
                            start_point.x += 10;
                            
                        },
                        Button::Left => {
                            info!("Button 2 pressed");
                            //Timer::after(Duration::from_millis(100)).await;
                            start_point.x -= 10;
                           
                        },
                        Button::Up => {
                            info!("Button 3 pressed");
                            is_turned = true;
                            //Timer::after(Duration::from_millis(100)).await;
                            start_point.y -= 10;

                        },
                        Button::Down => {
                            info!("Button 4 pressed");
                            is_turned = false;
                            //Timer::after(Duration::from_millis(100)).await;
                            start_point.y += 10;
                           
                        }
                    }
                    if is_turned {
                        image = Image::new(&raw_image_back, start_point);
                    } else {
                        image = Image::new(&raw_image_front, start_point);
                    }
                    
                    image.draw(&mut display).unwrap();
                    display.flush().await.unwrap();

                }
                ButtonEvent::Released(id) => {
                    info!("Btn {:#?} released", id);
                }
            }
        };
    }


}

#[derive(Clone, Copy, Format)]
enum ButtonEvent {
    Pressed(Button),
    Released(Button),
}

#[derive(Clone, Copy, Format)]
enum Button {
    Up,
    Left,
    Down,
    Right,
}
