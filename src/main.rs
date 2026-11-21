#![deny(unsafe_code)]
#![no_main]
#![no_std]

use crate::hal::{pac, prelude::*};
use cortex_m_rt as rt;
use embedded_graphics::{
    mono_font::{MonoTextStyleBuilder, ascii::FONT_6X10},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};
use panic_halt as _;
use ssd1306::{I2CDisplayInterface, Ssd1306, prelude::*};
use stm32f4xx_hal::{
    self as hal, block,
    i2c::I2c,
    rcc::Config,
    serial::{Config as SerialConfig, Serial},
};
use heapless::String;
use core::fmt::Write;

#[rt::entry]
fn main() -> ! {
    let p = pac::Peripherals::take().unwrap();
    let mut rcc = p.RCC.freeze(Config::hse(8.MHz()));
    let gpioa = p.GPIOA.split(&mut rcc);
    let (_, (ch1, ..)) = p.TIM2.pwm_hz(1.kHz(), &mut rcc);
    let mut pwm = ch1.with(gpioa.pa5);
    let max_duty = pwm.get_max_duty();
    pwm.set_duty(0);
    pwm.enable();

    let tx_pin = gpioa.pa2.into_alternate();
    let rx_pin = gpioa.pa3.into_alternate();

    let serial: Serial<_, u8> = Serial::new(
        p.USART2,
        (tx_pin, rx_pin),
        SerialConfig::default().baudrate(115200.bps()),
        &mut rcc,
    )
    .unwrap();

    let (mut tx, mut rx) = serial.split();

    let gpiob = p.GPIOB.split(&mut rcc);
    let scl = gpiob.pb6.internal_pull_up(true);
    let sda = gpiob.pb7.internal_pull_up(true);

    let i2c = I2c::new(p.I2C1, (scl, sda), 100.kHz(), &mut rcc);

    let interface = I2CDisplayInterface::new(i2c);
    let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();
    display.init().unwrap();

    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(BinaryColor::On)
        .build();

    let mut buf: String<32> = String::new();
    let _ = write!(&mut buf, "Brightness is 0");

    Text::with_baseline(&buf, Point::zero(), text_style, Baseline::Top)
        .draw(&mut display)
        .unwrap();

    display.flush().unwrap();

    loop {
        let byte = block!(rx.read()).unwrap();
        block!(tx.write(byte)).unwrap();

        let brightness: u32;

        match byte {
            b'1' => {
                brightness = 25;
            }
            b'2' => {
                brightness = 50;
            }
            b'3' => {
                brightness = 75;
            }
            b'4' => {
                brightness = 100;
            }
            _ => {
                brightness = 0;
            }
        }
        let duty = (max_duty as u32 * brightness) / 100;
        pwm.set_duty(duty as u16);
        display.clear(BinaryColor::Off);
        buf.clear();
        let _ = write!(&mut buf, "Brightness is {}", brightness);

        Text::with_baseline(&buf, Point::zero(), text_style, Baseline::Top)
            .draw(&mut display)
            .unwrap();
        display.flush().unwrap();
    }
}
