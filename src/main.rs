use std::{thread, time::Duration};

use hc_sr04::{HcSr04, Unit};
use rppal::{gpio::*, i2c::I2c};

const CS: u8 = 23;
const DIO: u8 = 24;
const CLK: u8 = 25;
const FREQUENCY: u32 = 50_000; // 50kHz
const LCD_ADDRESS: u16 = 0x27;

pub struct Lcd {
    i2c: I2c,
}

impl Lcd {
    pub fn new() -> Self {
        let mut i2c = I2c::new().unwrap();
        i2c.set_slave_address(LCD_ADDRESS).unwrap();
        Self { i2c }
    }

    pub fn init(&mut self) {
        self.send_command(0x33);
        thread::sleep(Duration::from_millis(5));
        self.send_command(0x32);
        thread::sleep(Duration::from_millis(5));
        self.send_command(0x28);
        thread::sleep(Duration::from_millis(5));
        self.send_command(0x0c);
        thread::sleep(Duration::from_millis(5));
        self.send_command(0x01);
        self.write_byte(0x08);
    }

    pub fn write_byte(&mut self, data: u8) {
        assert!(self.i2c.write(&[data | 0x08]).unwrap() == 1);
    }

    pub fn send_command(&mut self, comm: u8) {
        let mut buf;
        buf = comm & 0xf0;
        buf |= 0x04; // RS = 0, RW = 0, EN = 1
        self.write_byte(buf);
        thread::sleep(Duration::from_millis(2));
        buf &= 0xfb; // Make EN = 0
        self.write_byte(buf);

        // Send bit3-0 secondly
        buf = (comm & 0x0f) << 4;
        buf |= 0x04; // RS = 0, RW = 0, EN = 1
        self.write_byte(buf);
        thread::sleep(Duration::from_millis(2));
        buf &= 0xfb; // Make EN = 0
        self.write_byte(buf);
    }

    pub fn send_data(&mut self, data: u8) {
        // Send bit7-4 firstly
        let mut buf = data & 0xF0;
        buf |= 0x05; // RS = 1, RW = 0, EN = 1
        self.write_byte(buf);
        thread::sleep(Duration::from_millis(2));
        buf &= 0xFB; // Make EN = 0
        self.write_byte(buf);

        // Send bit3-0 secondly
        buf = (data & 0x0F) << 4;
        buf |= 0x05; // RS = 1, RW = 0, EN = 1
        self.write_byte(buf);
        thread::sleep(Duration::from_millis(2));
        buf &= 0xFB; // Make EN = 0
        self.write_byte(buf);
    }

    pub fn clear(&mut self) {
        self.send_command(0x01);
    }

    pub fn write(&mut self, mut x: u8, mut y: u8, data: &[u8]) {
        x = x.min(15);
        y = y.min(1);

        let addr = 0x80 + 0x40 * y + x;
        self.send_command(addr);

        for &byte in data {
            self.send_data(byte);
        }
    }
}

impl Default for Lcd {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Adc0834 {
    cs: OutputPin,
    clk: OutputPin,
    dio: IoPin,
    frequency: u32,
}

impl Adc0834 {
    pub fn new(cs: OutputPin, clk: OutputPin, dio: Pin, frequency: u32) -> Self {
        Adc0834 {
            cs,
            clk,
            dio: dio.into_io(Mode::Output),
            frequency,
        }
    }

    pub fn read(&mut self, channel: u8) -> u8 {
        self.cs.set_low();

        self.dio.set_mode(Mode::Output);

        // Start bit
        self.set_clock_low();
        self.dio.set_high();
        self.set_clock_high();

        // SGL/DIF
        self.set_clock_low();
        self.dio.set_high();
        self.set_clock_high();

        // ODD/SIGN
        self.set_clock_low();
        self.dio.write((channel % 2).into());
        self.set_clock_high();

        // SELECT1
        self.set_clock_low();
        self.dio.write((channel > 1).into());
        self.set_clock_high();

        // Allow the MUX to settle for 1/2 clock cycle
        self.set_clock_low();

        // Switch DIO pin to input to read data
        self.dio.set_mode(Mode::Input);

        // Read data from MSB to LSB
        let mut value1: u8 = 0;
        for _ in 0..8 {
            self.set_clock_high();
            self.set_clock_low();
            value1 <<= 1;
            value1 |= self.dio.read() as u8;
        }

        // Read data from LSB to MSB
        let mut value2: u8 = 0;
        for i in 0..8 {
            let bit = (self.dio.read() as u8) << i;
            value2 |= bit;
            self.set_clock_high();
            self.set_clock_low();
        }

        // Set CS pin to high to clear all internal registers
        self.cs.set_high();

        // Done reading, set DIO pin back to output
        self.dio.set_mode(Mode::Output);

        if value1 == value2 {
            value1
        } else {
            0
        }
    }

    fn set_clock_high(&mut self) {
        self.clk.set_high();
        self.tick();
    }

    fn set_clock_low(&mut self) {
        self.clk.set_low();
        self.tick();
    }

    fn tick(&self) {
        let period = Duration::from_secs(1) / self.frequency;
        let period_half = period / 2;
        thread::sleep(period_half);
    }
}

fn main() {
    let gpio = Gpio::new().unwrap();
    let cs = gpio.get(CS).unwrap().into_output();
    let dio = gpio.get(DIO).unwrap();
    let clk = gpio.get(CLK).unwrap().into_output();
    let mut hcsr = HcSr04::new(9, 10, None).unwrap();

    // let lcd = Lcd::new(0, 0, 0, 0);
    let mut display = Lcd::new();
    display.init();
    display.write(0, 0, b"Hello");
    display.write(0, 1, b"World");

    let mut adc = Adc0834::new(cs, clk, dio, FREQUENCY);

    loop {
        let value1 = adc.read(0);
        let value2 = adc.read(1);
        println!("Read value: {value1}, {value2}");

        let joystick_info = format!("{}:{}", value1, value2);
        let hcsr_info = match hcsr.measure_distance(Unit::Meters).unwrap() {
            Some(dist) => dist.to_string(),
            _ => String::from("Nothing measured"),
        };
        display.clear();
        display.write(0, 0, joystick_info.as_bytes());
        display.write(0, 1, hcsr_info.as_bytes());
        thread::sleep(Duration::from_millis(50));
    }
}
