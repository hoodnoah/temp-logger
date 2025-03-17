#![cfg_attr(not(test), no_std)]

#[cfg(test)]
extern crate std;

// the following are the constants for CRC-8/NRSC-5
const CRC8_INITIAL: u8 = 0xFF;
const CRC8_POLYNOMIAL: u8 = 0x31; // CRC[7:0] = 1 + X^4 + X^5 + X^8

/// Computes the CRC-8/NRSC-5 checksum for the given data, an array of bytes.
///
pub fn compute_crc8(data: &[u8]) -> u8 {
    let mut crc = CRC8_INITIAL;

    // perform the polynomial division
    for &byte in data {
        crc ^= byte; // XOR byte into CRC

        // for each bit of the given byte...
        for _ in 0..8 {
            // If the most significant bit is set...
            if crc & 0x80 != 0 {
                // Apply polynomial
                crc = (crc << 1) ^ CRC8_POLYNOMIAL;
            } else {
                // otherwise shift left
                crc <<= 1;
            }
        }
    }

    // CRC contains the remainder of the polynomial division
    crc
}

// extracts the humidity and temperature readings from the given data
// the data is expected to be 6 bytes long
// The first byte is the status byte; we discard this status since it is read in a different step of the process.
// The second byte is the humidity MSB, followed by the humidity LSB.
// The fourth byte contains the final 4 bits of the humidity value, and the first 4 bits of the temp.
// The fifth byte is the temperature MSB, and the sixth byte is the temperature LSB.
// The last byte is the CRC, which we don't need to extract; it was checked in another step.
pub fn extract_readings(data: &[u8]) -> (u32, u32) {
    // shift the MSB of the humidity value into the top 8 bits of a u32.
    // Then the next byte into the next 8 bits, and the last 4 bits into the next 4 bits.
    let raw_humidity: u32 =
        ((data[1] as u32) << 12) | ((data[2] as u32) << 4) | ((data[3] as u32) >> 4);

    // shift the last 4 bits of the fourth byte into the top 4 bits of a u32.
    // we AND it with 0x0f to clear the top 4 bits of the byte, since those are humidity values.
    // shift the next byte into the next 8 bits, and the last byte into the final 8 bits.
    let raw_temperature: u32 =
        ((data[3] as u32 & 0x0f) << 16) | ((data[4] as u32) << 8) | (data[5] as u32);

    (raw_humidity, raw_temperature)
}

// given the raw signal from the DHT20, converts to a percentage
// relative humidity
pub fn convert_humidity(humidity: u32) -> f32 {
    // convert the raw humidity signal to a %RH per the datasheet
    // 0x100000 is 2^20, the scale factor for the 20-bit value
    let humidity = humidity as f32 / 0x100000 as f32;
    // multiply by 100 to get full percentage RH
    humidity * 100.0
}

// given the raw signal from the DHT20, converts to a temperature value
// in degrees Celsius
pub fn convert_temperature(temperature: u32) -> f32 {
    // convert the raw temperature signal to a C per the datasheet
    // 0x100000 is 2^20, the scale factor for the 20-bit value
    let temperature = temperature as f32 / 0x100000 as f32;
    // multiply by 200 to get full temperature in C
    temperature * 200.0 - 50.0
}

#[cfg(test)]
mod tests_crc8 {
    use super::*;

    #[test]
    fn test_crc8_beef42() {
        let data = [0xBE, 0xEF, 0x42]; // simple example data
        let expected_crc = 0x04; // expected CRC, from online calculator
        assert_eq!(compute_crc8(&data), expected_crc);
    }

    #[test]
    fn test_crc8_empty() {
        let data: [u8; 0] = [];

        assert_eq!(compute_crc8(&data), CRC8_INITIAL); // should return the initial CRC value
    }

    #[test]
    fn test_crc8_single_byte() {
        let data = [0x42]; // single byte data
        let expected_crc = 0xF3; // expected CRC, from online calculator
        assert_eq!(compute_crc8(&data), expected_crc);
    }

    #[test]
    fn test_crc8_bit_flipping() {
        let data1 = [0x12, 0x34, 0x56];
        let data2 = [0x12, 0x32, 0x57]; // bit flipped
        assert_ne!(compute_crc8(&data1), compute_crc8(&data2));
    }
}

#[cfg(test)]
mod tests_convert_humidity {
    use super::*;

    #[test]
    fn test_convert_50_percent_rh() {
        let raw_humidity = 0x80000; // 50% RH in 20-bit format

        let expected_humidity = 50.0; // expected humidity in percentage
        let converted_humidity = convert_humidity(raw_humidity);
        assert_eq!(converted_humidity, expected_humidity);
        assert!(converted_humidity >= 0.0 && converted_humidity <= 100.0); // check bounds
    }
}

#[cfg(test)]
mod tests_convert_temperature {
    use super::*;

    #[test]
    fn test_convert_25_degrees_c() {
        let raw_temperature = 0x00060000; // 25 degrees C in 20-bit format

        let expected_temperature = 25.0; // expected temperature in Celsius
        let actual_temperature = convert_temperature(raw_temperature);

        assert_eq!(actual_temperature, expected_temperature);
        assert!(actual_temperature >= -40.0 && actual_temperature <= 80.0); // check bounds
    }
}

#[cfg(test)]
mod tests_extract_readings {
    use super::*;

    #[test]
    fn test_extract_known_humidity() {
        let data: [u8; 8] = [0x18, 0x80, 0x00, 0x00, 0x06, 0x66, 0x66, 0x00];

        let (humidity, _) = extract_readings(&data);

        assert_eq!(humidity, 0x80000); // expected humidity in 20-bit format
    }

    #[test]
    fn test_extract_known_temperature() {
        // first byte is status, second byte is humidity MSB, third byte is humidity LSB, third byte is
        // shared 4 bits between humidity and temperature.
        let data: [u8; 7] = [0x18, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00];

        let (_, temperature) = extract_readings(&data);
        assert_eq!(temperature, 0x60000); // expected temperature in 20-bit format
    }
}
