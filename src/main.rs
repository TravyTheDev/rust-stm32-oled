#![deny(unsafe_code)]
#![no_main]
#![no_std]

use crate::hal::{pac, prelude::*};
use cortex_m_rt as rt;
use embedded_graphics::{
    mono_font::{MonoTextStyleBuilder, ascii::FONT_10X20},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};
use heapless::String;
use panic_halt as _;
use ssd1306::{I2CDisplayInterface, Ssd1306, prelude::*};
use stm32f4xx_hal::{
    self as hal, block,
    i2c::I2c,
    rcc::Config,
    serial::{Config as SerialConfig, Serial},
};
// use core::fmt::Write;

const FONT_HEIGHT: i32 = 20;
const LINE_SPACING: i32 = 2;
const ROW_STEP: i32 = FONT_HEIGHT + LINE_SPACING;
const MAX_Y: i32 = 64 - FONT_HEIGHT;

struct ScreenText {
    buf: String<128>,
    row_y: i32,
    col_x: i32,
}

impl ScreenText {
    fn new() -> Self {
        Self {
            buf: String::new(),
            row_y: 0,
            col_x: 0,
        }
    }
    fn next_line(&mut self) {
        self.col_x = 0;
        if self.row_y < MAX_Y {
            self.row_y += ROW_STEP;
        };
    }
    fn push_char(&mut self, ch: char) {
        match ch {
            '\r' | '\n' => {
                self.next_line();
            }
            '\x08' | '\x7f' => {
                self.buf.pop();
            }
            _ => {
                if ch.is_ascii_graphic() || ch == ' ' {
                    let _ = self.buf.push(ch);
                }
            }
        }
    }
}

#[rt::entry]
fn main() -> ! {
    let p = pac::Peripherals::take().unwrap();
    let mut rcc = p.RCC.freeze(Config::hse(8.MHz()));
    let gpioa = p.GPIOA.split(&mut rcc);

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
        .font(&FONT_10X20)
        .text_color(BinaryColor::On)
        .build();

    let mut screen_text = ScreenText::new();
    display.flush().unwrap();
    loop {
        let byte = block!(rx.read()).unwrap();
        block!(tx.write(byte)).unwrap();
        screen_text.push_char(byte as char);
        let _ = display.clear(BinaryColor::Off);

        Text::with_baseline(
            &screen_text.buf,
            Point::new(screen_text.col_x, screen_text.row_y),
            text_style,
            Baseline::Top,
        )
        .draw(&mut display)
        .unwrap();
        display.flush().unwrap();
    }
}
