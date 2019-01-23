#![no_main]
#![no_std]

#[allow(unused)]
use panic_semihosting;

use core::fmt::Write;
use stm32f0xx_hal as hal;

use crate::hal::delay::Delay;
use crate::hal::prelude::*;
use crate::hal::serial::{RxPin, Serial, TxPin};
use crate::hal::stm32;
use crate::hal::stm32::USART1;
use embedded_hal::digital::{InputPin, OutputPin};

use nb::block;

use cortex_m::peripheral::Peripherals;
use cortex_m_rt::entry;

#[entry]
fn main() -> ! {
    if let (Some(p), Some(cp)) = (stm32::Peripherals::take(), Peripherals::take()) {
        cortex_m::interrupt::free(move |cs| {
            let mut flash = p.FLASH;
            let mut rcc = p.RCC.configure().sysclk(8.mhz()).freeze(&mut flash);

            let gpioa = p.GPIOA.split(&mut rcc);

            /* (Re-)configure PA1 as output */
            let _led = gpioa.pa1.into_push_pull_output(cs);
            let tx = gpioa.pa9.into_alternate_af1(cs);
            let rx = gpioa.pa10.into_alternate_af1(cs);
            let mut clk = gpioa.pa3.into_push_pull_output_hs(cs);
            let mut data = gpioa.pa4.into_open_drain_output(cs);
            data.internal_pull_up(cs, true);
            data.set_high();

            // Get delay provider
            let mut delay = Delay::new(cp.SYST, &rcc);

            let mut serial = Serial::usart1(p.USART1, (tx, rx), 115_200.bps(), &mut rcc);

            let mut count = 0;
            loop {
                let read = block!(serial.read()).unwrap();
                match read {
                    0b00000000 => {
                        count += 1;
                        if count == 20 {
                            serial.write_str("BBIO1").unwrap();
                            count = 0;
                        }
                    }
                    0b00000101 => {
                        serial.write_str("RAW1").unwrap();
                        raw_wire_mode(&mut serial, &mut clk, &mut data, &mut delay);
                    }
                    _ =>
                    // Someone tries to use this as something else, just ignore it
                    {
                        ()
                    }
                };
            }
        });
    }

    loop {
        continue;
    }
}

// Implement http://dangerousprototypes.com/docs/Raw-wire_(binary) in here
// Only do the stuff that openocd needs
fn raw_wire_mode<TXPIN, RXPIN, CLK, DATA>(
    serial: &mut Serial<USART1, TXPIN, RXPIN>,
    clk: &mut CLK,
    data: &mut DATA,
    delay: &mut Delay,
) where
    TXPIN: TxPin<USART1>,
    RXPIN: RxPin<USART1>,
    CLK: OutputPin,
    DATA: InputPin + OutputPin,
{
    loop {
        let read = block!(serial.read()).unwrap();
        if read == 0b00000000 {
            serial.write_str("BBIO1").unwrap();
            return;
        } else if read == 0b00000001 {
            serial.write_str("RAW1").unwrap();
        } else if read == 0b00000110 {
            let c = read_byte(clk, data, delay);
            block!(serial.write(c)).unwrap();
        } else if read == 0b00000111 {
            clk.set_high();
            data.set_high();
            delay.delay_us(1 as u8);
            clk.set_low();
            delay.delay_us(1 as u8);
            serial
                .write(if data.is_high() { 0b1 } else { 0b0 })
                .unwrap();
        }
        // SKIPPED
        else if read == 0b00001001 {
            clk.set_high();
            delay.delay_us(1 as u8);
            clk.set_low();
            delay.delay_us(1 as u8);
            serial.write(0x01).unwrap();
        } else if read & 0b11111110 == 0b00001010 {
            if read & 0b1 == 1 {
                clk.set_high();
            } else {
                clk.set_low();
            }
            serial.write(0x01).unwrap();
        } else if read & 0b11111110 == 0b00001100 {
            if read & 0b1 == 1 {
                data.set_high();
            } else {
                data.set_low();
            }
            serial.write(0x01).unwrap();
        } else if read & 0xF0 == 0b00010000 {
            // Bulk transfer
            let count = read & 0x0F;
            // It's not completely clear from the documentation, but this should be acknowledged as well
            block!(serial.write(0x01)).unwrap();
            for _ in 0..count + 1 {
                let read = block!(serial.read()).unwrap();
                write_byte(read, clk, data, delay);
                block!(serial.write(0x01)).unwrap();
            }
        } else if read & 0xF0 == 0b00100000 {
            let count = read & 0x0F;
            for _ in 0..count + 1 {
                clk.set_high();
                delay.delay_us(1 as u8);
                clk.set_low();
                delay.delay_us(1 as u8);
            }
            serial.write(0x01).unwrap();
        } else if read & 0xF0 == 0b0100_0000 {
            // Configure peripherals
            // Just ignore it
            serial.write(0x01).unwrap();
        } else if read & 0xF0 == 0b1000_0000 {
            // Configure mode
            // Just ignore it
            serial.write(0x01).unwrap();
        } else if read & 0xF0 == 0b0110_0000 {
            // Configure speed
            // Just ignore it
            serial.write(0x01).unwrap();
        } else {
            panic!("Unexpected item");
        }
    }
}

fn write_byte<CLK, DATA>(mut c: u8, clk: &mut CLK, data: &mut DATA, delay: &mut Delay)
where
    CLK: OutputPin,
    DATA: InputPin + OutputPin,
{
    for _ in 0..8 {
        // Emulate the buspirate waveforms
        if c & 0b1 == 1 {
            data.set_high();
        } else {
            data.set_low();
        }
        delay.delay_us(1 as u8);
        clk.set_high();
        delay.delay_us(1 as u8);
        clk.set_low();
        delay.delay_us(1 as u8);
        c >>= 1;
    }
}

fn read_byte<CLK, DATA>(clk: &mut CLK, data: &mut DATA, delay: &mut Delay) -> u8
where
    CLK: OutputPin,
    DATA: InputPin + OutputPin,
{
    data.set_high();
    let mut c = 0;
    for _ in 0..8 {
        clk.set_high();
        delay.delay_us(1 as u8);
        c >>= 1;
        c |= if data.is_high() { 0x80 } else { 0 };
        clk.set_low();
        delay.delay_us(1 as u8);
    }
    c
}
