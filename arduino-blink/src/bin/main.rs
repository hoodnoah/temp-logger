#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_hal::{
    delay::Delay,
    gpio::{Level, Output},
    main
};
use esp_println::println;

#[main]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());

    println!("Hello, world!");

    // Set GPIO0 as output, set state high initially
    let mut led = Output::new(peripherals.GPIO0, Level::Low);

    led.set_high();

    // Initialize the Delay peripheral, using it to toggle LED
    let delay = Delay::new();

    loop {
        led.toggle();
        delay.delay_millis(500);
    }
}
