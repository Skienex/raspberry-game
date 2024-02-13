use std::{thread, time::Duration};

use rppal::gpio::*;

const CS: u8 = 23;
const DO: u8 = 24;
const CLK: u8 = 25;
const FREQUENCY: u32 = 50_000; // 50kHz

pub struct Adc0834 {
    cs: OutputPin,
    clk: OutputPin,
    dio: Pin,
    frequency: u32,
}

impl Adc0834 {
    pub fn new(cs: OutputPin, clk: OutputPin, dio: Pin, frequency: u32) -> Self {
        Adc0834 {
            cs,
            clk,
            dio,
            frequency,
        }
    }

    pub fn read(&mut self, channel: u8) -> u32 {
        self.cs.set_low();

        // let do_ = self.dio.into_output();
    }
}

fn set_clock(clk: &mut OutputPin, high: bool) {
    if high {
        clk.set_high();
    } else {
        clk.set_low();
    }
    let one_cycle: Duration = Duration::from_secs(1) / FREQUENCY;
    let half_cycle: Duration = one_cycle / 2;
    thread::sleep(half_cycle);
}

fn main() {
    let gpio = Gpio::new().unwrap();
    let mut cs = gpio.get(CS).unwrap().into_output();
    let mut do_ = gpio.get(DO).unwrap().into_input();
    let mut clk = gpio.get(CLK).unwrap().into_output();

    loop {
        cs.set_low();
        clk.set_low();

        // <---

        let mut value = 0u32;
        for _ in 0..8 {
            set_clock(&mut clk, true);
            set_clock(&mut clk, false);
            value <<= 1;
            value |= do_.is_high() as u32;
        }
        println!("Read value: {value}");

        // let a: u8 = "Hallo";
    }
}
