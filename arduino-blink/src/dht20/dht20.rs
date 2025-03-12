use embedded_hal::delay::DelayNs; // for timing
use embedded_hal::i2c::{I2c, SevenBitAddress};

#[repr(u8)] // represent as u8; permits casting to a byte
enum OpCode {
    CheckInit = 0x71,
    TriggerMeasurement = 0xAC,
}

const I2C_ADDRESS: SevenBitAddress = 0x38; // DHT20 default I2C address per datasheet
const RESET_REGISTERS: [u8; 3] = [0x1B, 0x1C, 0x1E];

pub struct Dht20<I2C> {
    i2c: I2C,
    address: SevenBitAddress,
    initialized: bool,
}

impl<I2C, E> Dht20<I2C>
where
    I2C: I2c<Error = E>,
{
    // constructor for Dht20 struct
    pub fn new(i2c: I2C) -> Self {
        Self {
            i2c, // dependency injection; receive the I2C instance
            address: I2C_ADDRESS,
            initialized: false,
        }
    }

    // initialize the sensor, return nothing
    pub fn init<D: DelayNs>(&mut self, delay: &mut D) -> Result<(), E> {
        delay.delay_ms(100); // wait for sensor to power up, no less than 100ms

        self.check_init(delay)
    }

    // polls the sensor to determine its initialization state
    fn check_init<D: DelayNs>(&mut self, delay: &mut D) -> Result<(), E> {
        let mut buffer = [0u8; 1]; // set up a buffer to hold response word (byte)

        // Send check_status opcode
        self.i2c
            .write_read(self.address, &[OpCode::CheckInit as u8], &mut buffer)?;

        let status = buffer[0];

        // Ensure status is 0x18
        if (status & 0x18) != 0x18 {
            for reg in RESET_REGISTERS.iter() {
                self.reset_register(delay, *reg)?;
            }
        }

        // initialized
        self.initialized = true;
        Ok(())
    }

    // reset the sensor; undocumented by aosong, following along with
    // code from https://github.com/RobTillaart/DHT20/ as it's the best available documentation.
    fn reset_register<D: DelayNs>(&mut self, delay: &mut D, reg: u8) -> Result<(), E> {
        let mut buffer = [0u8; 3]; // buffer to hold 3 response words (bytes)

        // Write 0x00, 0x00 to the register - clear the values
        self.i2c.write(self.address, &[reg, 0x00, 0x00])?;

        // delay for stability's sake
        delay.delay_ms(5);

        // Read back 3 bytes from the register
        self.i2c.write_read(self.address, &[reg], &mut buffer)?;
        delay.delay_ms(5);

        // Write modified values back to register; we're OR-ing them w/ 0xB0.
        // Undocumented, just copying from RobTillaart's code.
        self.i2c
            .write(self.address, &[0xB0 | reg, buffer[1], buffer[2]])?;
        delay.delay_ms(5);

        Ok(())
    }
}
