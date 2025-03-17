#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_hal::{delay::Delay, main};
use esp_println::println;

// internal
use arduino_blink::dht20::dht20;

#[main]
fn main() -> ! {
    // Initialize the ESP32 peripherals
    println!("Initializing ESP32 peripherals...");
    let peripherals = esp_hal::init(esp_hal::Config::default());
    println!("Done.");

    // Set up I2C
    println!("Setting up I2C...");
    let i2c = match esp_hal::i2c::master::I2c::new(
        peripherals.I2C0,
        esp_hal::i2c::master::Config::default(),
    ) {
        Ok(i2c) => i2c
            .with_scl(peripherals.GPIO12)
            .with_sda(peripherals.GPIO11),
        Err(e) => {
            println!("Error initializing I2c: {:?}", e);
            loop {}
        }
    };
    println!("Done.");

    // Set up the DHT20 sensor
    println!("Setting up DHT20 sensor...");
    let mut delay = Delay::new();
    let mut dht20 = dht20::Dht20::new(i2c);

    if let Err(e) = dht20.init(&mut delay) {
        println!("Failed to initialize the DHT20 sensor: {:?}", e);
        loop {}
    }
    println!("Done.");

    loop {
        // Read temperature and humidity
        match dht20.take_reading(&mut delay) {
            Ok(reading) => {
                println!("Temperature: {}°F", reading.temperature_fahrenheit());
                println!("Humidity: {}%", reading.humidity());
            }
            Err(e) => {
                println!("Failed to read from the DHT20 sensor: {:?}", e);
            }
        }

        // match dht20.read_raw(&mut delay) {
        //     Ok(data) => {
        //         println!("Raw data: {:02X?}", data);
        //     }
        //     Err(e) => {
        //         println!("Failed to read from the DHT20 sensor: {:?}", e);
        //     }
        // }

        // Wait for a second before the next reading
        delay.delay_millis(10000);
    }
}
