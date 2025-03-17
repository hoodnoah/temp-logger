use embedded_hal::delay::DelayNs; // for timing
use embedded_hal::i2c::{I2c, SevenBitAddress};

// internal
use utils::compute_crc8; // CRC8 checksum helper function

#[derive(Debug)]
pub struct DHTReading {
    temperature: f32, // represented internally in Celsius
    humidity: f32,    // represented internally as percentage
}
impl DHTReading {
    // constructor for DHTReading struct
    pub fn new(temperature: f32, humidity: f32) -> Self {
        Self {
            temperature,
            humidity,
        }
    }

    // getter for humidity
    pub fn humidity(&self) -> f32 {
        self.humidity
    }

    // getters for temperature
    pub fn temperature_celsius(&self) -> f32 {
        self.temperature
    }

    pub fn temperature_fahrenheit(&self) -> f32 {
        self.temperature * 9.0 / 5.0 + 32.0
    }
}

#[derive(Debug)]
pub enum DHT20Error<E> {
    I2C(E),
    CrcMismatch,
    NotInitialized,
}

impl<E> From<E> for DHT20Error<E> {
    fn from(err: E) -> Self {
        DHT20Error::I2C(err)
    }
}

#[repr(u8)] // represent as u8; permits casting to a byte
enum OpCode {
    CheckStatus = 0x71,
    TriggerMeasurement = 0xAC,
    StatusReady = 0x80,
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
    pub fn init<D: DelayNs>(&mut self, delay: &mut D) -> Result<(), DHT20Error<E>> {
        delay.delay_ms(100); // wait for sensor to power up, no less than 100ms

        self.check_init(delay)
    }

    // request a reading from the sensor
    // returns a DHTReading struct containing the temperature and humidity
    pub fn take_reading<D: DelayNs>(&mut self, delay: &mut D) -> Result<DHTReading, DHT20Error<E>> {
        if !self.initialized {
            return Err(DHT20Error::NotInitialized);
        }

        self.trigger_measurement(delay)?; // trigger the measurement

        self.wait_for_ready(delay)?; // wait for measurement to be ready

        let data = self.read_measurement()?;

        // extract the humidity and temperature readings from the data
        let (raw_humidity, raw_temperature) = utils::extract_readings(&data);

        // convert the raw readings to percentage, Celsius
        let humidity = utils::convert_humidity(raw_humidity);
        let temperature = utils::convert_temperature(raw_temperature);

        // return the readings as a DHTReading struct
        Ok(DHTReading::new(temperature, humidity))
    }

    pub fn read_raw<D: DelayNs>(&mut self, delay: &mut D) -> Result<[u8; 6], DHT20Error<E>> {
        if !self.initialized {
            return Err(DHT20Error::NotInitialized);
        }

        self.trigger_measurement(delay)?; // trigger the measurement

        self.wait_for_ready(delay)?; // wait for measurement to be ready

        let data = self.read_measurement()?;

        Ok(data)
    }

    // polls the sensor to determine its initialization state
    fn check_init<D: DelayNs>(&mut self, delay: &mut D) -> Result<(), DHT20Error<E>> {
        let mut buffer = [0u8; 1]; // set up a buffer to hold response word (byte)

        // Send check_status opcode
        self.i2c
            .write_read(self.address, &[OpCode::CheckStatus as u8], &mut buffer)?;

        let status = buffer[0];

        // Ensure status is 0x18
        if (status & 0x18) != 0x18 {
            for reg in RESET_REGISTERS.iter() {
                self.reset_register(delay, *reg)?;
            }
        }

        // wait 10ms for the sensor to stabilize (prerequisite for taking a measurement)
        delay.delay_ms(10);

        // initialized
        self.initialized = true;
        Ok(())
    }

    // reset the sensor; undocumented by aosong, following along with
    // code from https://github.com/RobTillaart/DHT20/ as it's the best available documentation.
    fn reset_register<D: DelayNs>(&mut self, delay: &mut D, reg: u8) -> Result<(), DHT20Error<E>> {
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

    // trigger a measurement
    fn trigger_measurement<D: DelayNs>(&mut self, delay: &mut D) -> Result<(), DHT20Error<E>> {
        // 0x33 and 0x00 are two argument bytes to be sent to the sensor when triggering a measurement.
        let command = [OpCode::TriggerMeasurement as u8, 0x33, 0x00];
        self.i2c.write(self.address, &command)?;

        delay.delay_ms(80); // wait 80ms per the datasheet (minimum time to ready)

        Ok(())
    }

    // wait for measurement to be ready
    fn wait_for_ready<D: DelayNs>(&mut self, delay: &mut D) -> Result<(), DHT20Error<E>> {
        let mut buffer = [0u8; 1]; // buffer to hold status word (1 byte)

        // poll until ready
        loop {
            self.i2c
                .write_read(self.address, &[OpCode::CheckStatus as u8], &mut buffer)?;
            // buffer[0] means first (only) byte
            // mask out all but the 7th bit (0x80); if it's 0, we're ready.
            if buffer[0] & (OpCode::StatusReady as u8) == 0 {
                return Ok(()); // measurement complete
            }
            // otherwise, wait 10ms before polling again
            delay.delay_ms(0);
        }
    }

    // read the measurement values from the sensor
    // these must be parsed before usage
    fn read_measurement(&mut self) -> Result<[u8; 6], DHT20Error<E>> {
        let mut buffer = [0u8; 7]; // buffer to hold 6 data bytes and 1 CRC byte

        // read 7 bytes from the sensor
        self.i2c.read(self.address, &mut buffer)?;

        let crc = buffer[6]; // 7th byte is the CRC

        // compute CRC8
        let crc_check = compute_crc8(&buffer[..6]);
        if crc != crc_check {
            return Err(DHT20Error::CrcMismatch);
        }
        // return the 6 data bytes
        Ok(buffer[..6].try_into().unwrap()) // convert slice to array
    }
}
