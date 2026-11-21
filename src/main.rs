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
use heapless::{String, Vec};
use panic_halt as _;
use ssd1306::{I2CDisplayInterface, Ssd1306, prelude::*};
use stm32f4xx_hal::{
    self as hal, i2c::I2c, nb, rcc::Config, serial::{Config as SerialConfig, Serial},
};

const MAX_COLS: usize = 12;
const MAX_ROWS: usize = 3;
const CHAR_HEIGHT_PX: i32 = 20;

struct ScreenText {
    lines: Vec<String<16>, MAX_ROWS>,
}

impl ScreenText {
    fn new() -> Self {
        let mut lines = Vec::new();
        let _ = lines.push(String::new()); // Start with one empty active line
        Self { lines }
    }
    fn start_new_line(&mut self) {
        if self.lines.len() >= MAX_ROWS {
            self.lines.remove(0);
        };
        let _ = self.lines.push(String::new());
    }
    fn push_char(&mut self, ch: char) {
        match ch {
            '\r' | '\n' => {
                self.start_new_line();
            }
            '\x08' | '\x7f' => {
                let popped_from_current =
                    self.lines.last_mut().
                    and_then(|line| line.pop())
                    .is_some();

                if !popped_from_current && self.lines.len() > 1 {
                    let _ = self.lines.pop();
                }
            }
            _ => {
                if ch.is_ascii_graphic() || ch == ' ' {
                    let needs_wrap = self
                        .lines
                        .last()
                        .map(|line| line.chars().count() > MAX_COLS)
                        .unwrap_or(true);
                    if needs_wrap {
                        self.start_new_line();
                    }

                    if let Some(last_line) = self.lines.last_mut() {
                        let _ = last_line.push(ch);
                    }
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

    let i2c = I2c::new(p.I2C1, (scl, sda), 400.kHz(), &mut rcc);

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
        let mut received_data = false;
        
        loop {
            match rx.read() {
                Ok(byte) => {
                    let _ = tx.write(byte);
                    screen_text.push_char(byte as char);
                    received_data = true;
                }
                Err(nb::Error::WouldBlock) => {
                    break;
                }
                Err(nb::Error::Other(_)) => {
                    break;
                }
            }
        }
        
        if received_data {
            let _ = display.clear(BinaryColor::Off);
            for (row_index, line) in screen_text.lines.iter().enumerate() {
                let y = (row_index as i32) * CHAR_HEIGHT_PX;
                Text::with_baseline(line, Point::new(0, y), text_style, Baseline::Top)
                    .draw(&mut display)
                    .unwrap();
            }
            let _ = display.flush();
        }
    }
}
