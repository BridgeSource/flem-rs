#![no_std]

use core::fmt::{Debug, Error, Formatter};

pub mod buffer;
pub mod traits;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Status {
    Ok,
    PacketReceived,
    PacketBuilding,
    GetByteFinished,
    VersionLength,
    PacketOverflow,
    HeaderBytesNotFound,
    GetByteIssue,
    ChecksumError,
    UnspecifiedError,
    UnrecognizedRequest,
    InvalidDataLengthDetected,
}

const FLEM_ID_NAME_SIZE: usize = 25;

/// Const ID Size:
///     - 25 bytes Name buffer
///     - 2 bytes for packet size
///     - 3 bytes for major, minor, patch
const FLEM_ID_SIZE: usize = FLEM_ID_NAME_SIZE + (u16::BITS as usize / 8 as usize) + 3;
#[repr(C)]
pub struct DataId {
    major: u8,
    minor: u8,
    patch: u8,
    max_packet_size: u16,
    name: [char; FLEM_ID_NAME_SIZE as usize],
}

impl DataId {
    pub fn new(name: &str, major: u8, minor: u8, patch: u8, packet_size: usize) -> DataId {
        let mut id = DataId {
            major: major,
            minor: minor,
            patch: patch,
            name: ['\0'; FLEM_ID_NAME_SIZE as usize],
            max_packet_size: packet_size as u16,
        };

        let version_size: usize = name.len();

        assert!(
            version_size <= FLEM_ID_NAME_SIZE,
            "Version should be 25 characters or less"
        );

        for a in 0..version_size {
            id.name[a as usize] = name.as_bytes()[a as usize] as char;
        }
        id
    }

    pub fn from(data: &[u8]) -> Option<DataId> {
        let mut buffer = ['\0'; FLEM_ID_NAME_SIZE as usize];
        let mut packet_length_buffer = [0 as u8; 2];
        let mut major: u8 = 0;
        let mut minor: u8 = 0;
        let mut patch: u8 = 0;

        let mut name_counter = 0;
        let mut packet_size_counter = 0;

        for (index, byte) in data.iter().enumerate() {
            match index {
                0 => {
                    major = *byte;
                }
                1 => {
                    minor = *byte;
                }
                2 => {
                    patch = *byte;
                }
                j if (j == 3 || j == 4) => {
                    packet_length_buffer[packet_size_counter] = *byte;
                    packet_size_counter += 1;
                }
                i if (5 <= i && i < FLEM_ID_NAME_SIZE + 5) => {
                    buffer[name_counter] = *byte as char;
                    name_counter += 1;
                }
                _ => {}
            }
        }

        Some(DataId {
            major,
            minor,
            patch,
            name: buffer,
            max_packet_size: u16::from_le_bytes(packet_length_buffer),
        })
    }

    pub fn get_name(&self) -> &[char; FLEM_ID_NAME_SIZE] {
        &self.name
    }

    pub fn get_major(&self) -> u8 {
        self.major
    }

    pub fn get_minor(&self) -> u8 {
        self.minor
    }

    pub fn get_patch(&self) -> u8 {
        self.patch
    }

    pub fn get_max_packet_size(&self) -> u16 {
        self.max_packet_size
    }

    pub fn as_u8_array(&self) -> &[u8] {
        let stream: &[u8] = unsafe {
            ::core::slice::from_raw_parts((self as *const DataId) as *const u8, FLEM_ID_SIZE)
        };
        stream
    }
}

#[derive(Copy, Clone)]
#[repr(C, packed)]
pub struct Packet<const T: usize> {
    header: u16,
    checksum: u16,
    request: u16,
    response: u16,
    length: u16,
    data: [u8; T],
    internal_counter: u32,
    data_length_counter: usize,
    status: Status,
}

pub mod response {
    pub const ASYNC: u16 = 0x0000;
    pub const SUCCESS: u16 = 0x0001;
    pub const UNKNOWN_REQUEST: u16 = 0xFFFE;
    pub const CHECKSUM_ERROR: u16 = 0xFFFF;
}

/// Pre-defined requests
pub mod request {
    pub const ID: u16 = 0x0001;
}

pub const FLEM_HEADER_SIZE: usize = 10;
pub const FLEM_HEADER: u16 = 0x5555;
const CRC16_TAB: [u16; 256] = [
    0x0000, 0xc0c1, 0xc181, 0x0140, 0xc301, 0x03c0, 0x0280, 0xc241, 0xc601, 0x06c0, 0x0780, 0xc741,
    0x0500, 0xc5c1, 0xc481, 0x0440, 0xcc01, 0x0cc0, 0x0d80, 0xcd41, 0x0f00, 0xcfc1, 0xce81, 0x0e40,
    0x0a00, 0xcac1, 0xcb81, 0x0b40, 0xc901, 0x09c0, 0x0880, 0xc841, 0xd801, 0x18c0, 0x1980, 0xd941,
    0x1b00, 0xdbc1, 0xda81, 0x1a40, 0x1e00, 0xdec1, 0xdf81, 0x1f40, 0xdd01, 0x1dc0, 0x1c80, 0xdc41,
    0x1400, 0xd4c1, 0xd581, 0x1540, 0xd701, 0x17c0, 0x1680, 0xd641, 0xd201, 0x12c0, 0x1380, 0xd341,
    0x1100, 0xd1c1, 0xd081, 0x1040, 0xf001, 0x30c0, 0x3180, 0xf141, 0x3300, 0xf3c1, 0xf281, 0x3240,
    0x3600, 0xf6c1, 0xf781, 0x3740, 0xf501, 0x35c0, 0x3480, 0xf441, 0x3c00, 0xfcc1, 0xfd81, 0x3d40,
    0xff01, 0x3fc0, 0x3e80, 0xfe41, 0xfa01, 0x3ac0, 0x3b80, 0xfb41, 0x3900, 0xf9c1, 0xf881, 0x3840,
    0x2800, 0xe8c1, 0xe981, 0x2940, 0xeb01, 0x2bc0, 0x2a80, 0xea41, 0xee01, 0x2ec0, 0x2f80, 0xef41,
    0x2d00, 0xedc1, 0xec81, 0x2c40, 0xe401, 0x24c0, 0x2580, 0xe541, 0x2700, 0xe7c1, 0xe681, 0x2640,
    0x2200, 0xe2c1, 0xe381, 0x2340, 0xe101, 0x21c0, 0x2080, 0xe041, 0xa001, 0x60c0, 0x6180, 0xa141,
    0x6300, 0xa3c1, 0xa281, 0x6240, 0x6600, 0xa6c1, 0xa781, 0x6740, 0xa501, 0x65c0, 0x6480, 0xa441,
    0x6c00, 0xacc1, 0xad81, 0x6d40, 0xaf01, 0x6fc0, 0x6e80, 0xae41, 0xaa01, 0x6ac0, 0x6b80, 0xab41,
    0x6900, 0xa9c1, 0xa881, 0x6840, 0x7800, 0xb8c1, 0xb981, 0x7940, 0xbb01, 0x7bc0, 0x7a80, 0xba41,
    0xbe01, 0x7ec0, 0x7f80, 0xbf41, 0x7d00, 0xbdc1, 0xbc81, 0x7c40, 0xb401, 0x74c0, 0x7580, 0xb541,
    0x7700, 0xb7c1, 0xb681, 0x7640, 0x7200, 0xb2c1, 0xb381, 0x7340, 0xb101, 0x71c0, 0x7080, 0xb041,
    0x5000, 0x90c1, 0x9181, 0x5140, 0x9301, 0x53c0, 0x5280, 0x9241, 0x9601, 0x56c0, 0x5780, 0x9741,
    0x5500, 0x95c1, 0x9481, 0x5440, 0x9c01, 0x5cc0, 0x5d80, 0x9d41, 0x5f00, 0x9fc1, 0x9e81, 0x5e40,
    0x5a00, 0x9ac1, 0x9b81, 0x5b40, 0x9901, 0x59c0, 0x5880, 0x9841, 0x8801, 0x48c0, 0x4980, 0x8941,
    0x4b00, 0x8bc1, 0x8a81, 0x4a40, 0x4e00, 0x8ec1, 0x8f81, 0x4f40, 0x8d01, 0x4dc0, 0x4c80, 0x8c41,
    0x4400, 0x84c1, 0x8581, 0x4540, 0x8701, 0x47c0, 0x4680, 0x8641, 0x8201, 0x42c0, 0x4380, 0x8341,
    0x4100, 0x81c1, 0x8081, 0x4040,
];

impl<const T: usize> Packet<T> {
    /// Creates a new Packet with a data buffer of const T: usize bytes
    ///
    /// # Example
    /// ```
    /// pub fn main() {
    ///     let rx = flem::Packet::<100>::new(); // Create new packet that can send / receive up to 100 bytes per packet
    ///
    /// }
    /// ```
    pub fn new() -> Self {
        assert!(T < u16::MAX as usize, "<T> should be u16::MAX or less"); // Bounds check T, must be less than u16::MAX
        return Self {
            header: 0,
            checksum: 0,
            request: 0,
            response: 0,
            length: 0,
            data: [0u8; T],
            internal_counter: 0,
            data_length_counter: 0,
            status: Status::Ok,
        };
    }

    /// Convenience function to response with data. The response byte is automatically set to SUCCESS.
    pub fn pack_data(&mut self, request: u16, data: &[u8]) -> Result<(), Status> {
        self.reset_lazy();
        self.request = request;
        match self.add_data(data) {
            Ok(_) => {
                self.response = response::SUCCESS;
                self.pack();
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    /// Convenience function to respond quickly if an error occurs (without data).
    pub fn pack_error(&mut self, request: u16, error: u16, data: &[u8]) -> Result<(), Status> {
        self.reset_lazy();
        self.request = request;
        self.response = error;
        match self.add_data(data) {
            Ok(_) => {
                self.response = error;
                self.pack();
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    /// Convenience function to respond with the ID. If communicating with UTF-8 partners, ascii should be true. This
    /// can only be used if the data packets are 30 bytes or longer (or twice that if ascii = false).
    ///
    /// # Arguments
    ///
    /// * `ascii` - Packages the ID as a UTF-8 ID. Used when talking to C/C++ partners.
    pub fn pack_id(&mut self, id: &DataId, ascii: bool) -> Result<(), Status> {
        self.reset_lazy();
        self.request = request::ID;
        self.response = response::SUCCESS;

        if ascii {
            self.add_data(&[id.get_major(); 1])?;
            self.add_data(&[id.get_minor(); 1])?;
            self.add_data(&[id.get_patch(); 1])?;
            self.add_data(&id.max_packet_size.to_le_bytes())?;

            let mut char_array: [u8; FLEM_ID_NAME_SIZE] = [0; FLEM_ID_NAME_SIZE];
            for (index, unicode) in id.name.iter().enumerate() {
                char_array[index] = *unicode as u8;
            }

            // Add the ASCII converted array
            self.add_data(&char_array)?;
        } else {
            // Send over the array as unicode
            self.add_data(id.as_u8_array())?;
        }

        self.pack();

        Ok(())
    }

    /// Pack a packet up: adds header and computes checksum.
    ///
    /// # Example
    ///
    /// ```
    /// pub fn main() {
    ///     use flem::{Packet};
    ///
    ///     const PACKET_SIZE: usize = 64; // 64 byte packet
    ///
    ///     const FLEM_EXAMPLE_REQUEST: u16 = 0xF;
    ///
    ///     let mut rx = Packet::<PACKET_SIZE>::new();
    ///
    ///     let mut data = [0 as u8; PACKET_SIZE];
    ///
    ///     /* Add data as needed to the data buffer */
    ///
    ///     rx.add_data(&data);
    ///     rx.set_request(FLEM_EXAMPLE_REQUEST);
    ///     
    ///     assert_ne!(rx.get_header(), 0x5555, "Packet header hasn't been set, should NOT be 0x5555");
    ///     
    ///     rx.pack();
    ///
    ///     assert_eq!(rx.get_header(), 0x5555, "Packet header has been set, should be 0x5555");
    ///
    ///     /* Send data */
    ///
    /// }
    /// ```
    ///
    pub fn pack(&mut self) {
        self.checksum(true);
        self.header = FLEM_HEADER;
    }

    /// Returns a copy of the data part of the packet as a byte array
    pub fn get_data(&self) -> [u8; T] {
        return self.data;
    }

    /// Adds data to a packet if there is room.
    pub fn add_data(&mut self, data: &[u8]) -> Result<(), Status> {
        if data.len() + self.length as usize > T {
            self.status = Status::PacketOverflow;
            Err(Status::PacketOverflow)
        } else {
            for i in 0..data.len() {
                self.data[i + self.length as usize] = data[i];
            }
            self.length += data.len() as u16;

            self.status = Status::Ok;
            Ok(())
        }
    }

    /// Computes the Checksum on the packet and compares to the sent checksum. Returns true if
    /// there is a match, otherwise false.
    pub fn validate(&mut self) -> bool {
        let crc = self.checksum(false);
        return crc == self.checksum;
    }

    /// Construct a packet one byte at a time. An internal counter keeps track of where the byte should go.
    /// The current return value is the Status and should be one of the following:
    /// - HeaderBytesNotFound - The packet header was not found
    /// - ChecksumError - The computed checksum does not match the sent checksum
    /// - PacketOverflow - Data is being added beyond length of the packet
    /// - PacketBuilding - This should be the default most of the time and indicates the packet is being built without issues so far.
    /// - PacketReceived - All data bytes have been received and the checksum has been validated
    ///
    /// # Arguments
    ///
    /// * `byte` - A single byte to add to a packet.
    ///
    /// # Example
    /// ```
    /// pub fn main() {
    ///     use flem::{Packet};
    ///
    ///     const PACKET_SIZE: usize = 64; // 64 byte packet
    ///
    ///     const FLEM_EXAMPLE_REQUEST: u16 = 0xF;
    ///
    ///     let mut rx = Packet::<PACKET_SIZE>::new();
    ///     let mut tx = Packet::<PACKET_SIZE>::new();
    ///
    ///     let mut data = [0 as u8; PACKET_SIZE];
    ///
    ///     /* Add data as needed to the data buffer */
    ///
    ///     tx.add_data(&data);
    ///     tx.set_request(FLEM_EXAMPLE_REQUEST);
    ///     tx.pack();
    ///
    ///
    ///     /* Send data */
    ///     
    ///     let tx_as_u8_array = tx.bytes();
    ///
    ///     // We are sending bytes across a hardware bus
    ///     let mut packet_received = false;
    ///     for byte in tx_as_u8_array {
    ///         // The received is getting bytes on the hardware bus
    ///         match rx.construct(*byte) {
    ///             Ok(_) => {
    ///                 packet_received = true;
    ///             },
    ///             Err(status) => {
    ///                 /* Handle other cases here */
    ///             }
    ///         }
    ///     }
    ///
    ///     assert!(packet_received, "Packet should have been constructed and validated.");
    ///
    /// }
    /// ```
    pub fn construct(&mut self, byte: u8) -> Result<(), Status> {
        let local_internal_counter = self.internal_counter;

        match local_internal_counter {
            0 => {
                if byte != 0x55 {
                    self.internal_counter = 0;
                    self.status = Status::HeaderBytesNotFound;
                    return Err(self.status);
                }
                self.header = byte as u16;
            }
            1 => {
                if byte != 0x55 {
                    self.internal_counter = 0;
                    self.status = Status::HeaderBytesNotFound;
                    return Err(self.status);
                }
                self.header |= (byte as u16) << 8;
            }
            2 => {
                self.checksum = byte as u16;
            }
            3 => {
                self.checksum |= (byte as u16) << 8;
            }
            4 => {
                self.request = byte as u16;
            }
            5 => {
                self.request |= (byte as u16) << 8;
            }
            6 => {
                self.response = byte as u16;
            }
            7 => {
                self.response |= (byte as u16) << 8;
            }
            8 => {
                self.length = byte as u16;
            }
            9 => {
                self.length |= (byte as u16) << 8;
                self.data_length_counter = 0;
                if self.length == 0 {
                    if self.validate() {
                        self.status = Status::PacketReceived;
                        return Ok(());
                    } else {
                        self.status = Status::ChecksumError;
                        return Err(self.status);
                    }
                }

                if self.length as usize > T {
                    self.status = Status::InvalidDataLengthDetected;
                    return Err(self.status);
                }
            }
            i if (FLEM_HEADER_SIZE as u32 <= i && i < FLEM_HEADER_SIZE as u32 + T as u32) => {
                if self.data_length_counter < self.length as usize {
                    self.data[self.data_length_counter] = byte;
                } else {
                    self.status = Status::PacketOverflow;
                    return Err(self.status);
                }
                self.data_length_counter += 1;
                if self.length as usize == self.data_length_counter {
                    if self.validate() {
                        self.status = Status::PacketReceived;
                        return Ok(());
                    } else {
                        self.status = Status::ChecksumError;
                        return Err(self.status);
                    }
                }
            }
            _ => {
                self.status = Status::PacketOverflow;
                return Err(self.status);
            }
        }

        self.internal_counter += 1;
        self.status = Status::PacketBuilding;

        Err(self.status)
    }

    /// This function treats the entire packet as a byte array and uses internal
    /// counters to determine the next byte. Keep calling this until either an
    /// error occurs or status is Status::GetByteFinished.
    ///
    /// It is often easier to use .bytes(), but this function is meant to be used
    /// in an async nature, for example an interrupt driven UART transmit FIFO.
    ///
    /// The return value is a Result composed of the byte requested if everything is going
    /// well, or a Status as an Error indicating all bytes have been gotten.
    ///
    /// # Example
    /// ```
    /// pub fn main() {
    ///    use flem::{Packet};
    ///    use heapless;
    ///    const PACKET_SIZE: usize = 64; // 64 byte packet
    ///    const FLEM_EXAMPLE_REQUEST: u16 = 0xF;
    ///    
    ///    let mut rx = Packet::<PACKET_SIZE>::new();
    ///    let mut tx = Packet::<PACKET_SIZE>::new();
    ///    
    ///    let mut data = [0 as u8; PACKET_SIZE];
    ///    
    ///    /* Add data as needed to the data buffer */
    ///    tx.add_data(&data);
    ///    tx.set_request(FLEM_EXAMPLE_REQUEST);
    ///    tx.pack();
    ///
    ///    /* Send data */
    ///    let mut tx_fifo_queue = heapless::spsc::Queue::<u8, 8>::new();
    ///    let mut keep_sending = true;
    ///    let mut packet_received = false;
    ///    let mut status = flem::Status::Ok;
    ///    
    ///    while keep_sending {
    ///        if !tx_fifo_queue.is_full() && status != flem::Status::GetByteFinished {
    ///            match tx.get_byte() {
    ///                Ok(byte) => {
    ///                    tx_fifo_queue.enqueue(byte).unwrap();
    ///                },                
    ///                Err(x) => {
    ///                    /* Tx code should stop transmitting */
    ///                    status = x;
    ///
    ///                }
    ///            }
    ///        }else{
    ///            // Queue is full, Tx the data, Rx on the other end
    ///            while !tx_fifo_queue.is_empty() {
    ///                match rx.construct(tx_fifo_queue.dequeue().unwrap()) {
    ///                    Ok(_) => {
    ///                        packet_received = true;
    ///                        keep_sending = false;
    ///                    },
    ///                    Err(status) => {
    ///                        /* Catch other statuses here on the Rx side */
    ///                    }
    ///                }
    ///            }
    ///        }
    ///    }
    ///
    ///    assert!(packet_received, "Packet should have been transferred");
    ///
    ///    // This test is redundant, since the checksums passed, still nice to see
    ///
    ///    let rx_bytes = rx.bytes();
    ///    let tx_bytes = tx.bytes();
    ///
    ///    for i in 0..rx_bytes.len() {
    ///        assert_eq!(rx_bytes[i], tx_bytes[i], "Rx and Tx packets don't match");
    ///    }
    ///}
    /// ```
    pub fn get_byte(&mut self) -> Result<u8, Status> {
        let bytes = self.bytes();
        let cnt = self.internal_counter;
        match cnt {
            i if (i < self.length() as u32) => {
                let byte = bytes[self.internal_counter as usize];
                self.internal_counter += 1;
                self.status = Status::Ok;
                Ok(byte)
            }
            _ => {
                self.status = Status::GetByteFinished;
                Err(self.status)
            }
        }
    }

    /// Sets the Flem request field
    pub fn set_request(&mut self, request: u16) {
        self.request = request;
    }

    /// Gets the Flem request field
    pub fn get_request(&self) -> u16 {
        self.request
    }

    /// Returns the stored checksum value
    pub fn get_checksum(&self) -> u16 {
        self.checksum
    }

    /// Sets the Flem response field
    pub fn set_response(&mut self, response: u16) {
        self.response = response;
    }

    /// Gets the Flem response field
    pub fn get_response(&self) -> u16 {
        self.response
    }

    /// Gets the status byte from the packet
    pub fn get_status(&mut self) -> Status {
        self.status
    }

    /// Get the header byte as u16
    pub fn get_header(&self) -> u16 {
        self.header
    }

    pub fn get_data_length(&self) -> usize {
        self.data_length_counter
    }

    /// Returns the _entire_ packet as a u8 byte array
    pub fn bytes(&self) -> &[u8] {
        let stream: &[u8] = unsafe {
            ::core::slice::from_raw_parts(
                (self as *const Packet<T>) as *const u8,
                self.length() as usize,
            )
        };

        return stream;
    }

    /// Computes a CRC16 IBM style checksum on the packet, except the header
    /// and checksum bytes
    pub fn checksum(&mut self, store: bool) -> u16 {
        let mut crc: u16 = 0;
        let bytes: &[u8] = self.bytes();
        let psize: u16 = bytes.len() as u16;

        //Skip the first 4 bytes, 2 header and 2 checksum
        for i in 4..psize {
            let ptr = bytes[i as usize] as u16;
            let lut_index = (crc ^ ptr) as u8;
            let mut tmp_crc = CRC16_TAB[lut_index as usize];
            tmp_crc ^= crc >> 8;
            crc = tmp_crc;
        }

        if store {
            self.checksum = crc;
        }

        return crc;
    }

    /// Resets the packet to all 0's, but does not clear the data array. Much faster than
    /// zeroing out the packet's data buffer. **Packets should be cleared before reusing, both Rx and Tx.**
    pub fn reset_lazy(&mut self) {
        self.checksum = 0;
        self.request = 0;
        self.response = 0;
        self.length = 0;
        self.internal_counter = 0;
        self.status = Status::Ok;
        self.data_length_counter = 0;
    }

    /// Resets the packet. The data array is cleared only if clear_data is true. **Packets should be
    /// cleared before reusing, both Rx and Tx.**
    ///
    /// # Arguments
    ///
    /// * `clear_data` - Zero out the data array.
    pub fn reset(&mut self) {
        self.reset_lazy();
        for i in 0..T {
            self.data[i] = 0;
        }
        self.data_length_counter = 0;
    }

    /// Length of the packet, **including the header and data.**
    ///
    /// # Example
    /// ```
    ///
    /// pub fn main() {
    ///     const PACKET_SIZE: usize = 100;
    ///
    ///     let mut tx = flem::Packet::<PACKET_SIZE>::new();
    ///
    ///     assert_eq!(tx.length() as usize, flem::FLEM_HEADER_SIZE as usize, "Length should be only {} bytes for the header", flem::FLEM_HEADER_SIZE);
    ///
    ///     let data = [0 as u8; PACKET_SIZE];
    ///
    ///     tx.add_data(&data);
    ///
    ///     assert_eq!(tx.length() as usize, PACKET_SIZE + flem::FLEM_HEADER_SIZE as usize, "Length should be {} bytes (packet size) + {} bytes for the header", PACKET_SIZE, flem::FLEM_HEADER_SIZE);
    /// }
    /// ```
    pub fn length(&self) -> usize {
        let mut x: usize = FLEM_HEADER_SIZE as usize;
        x += self.length as usize;
        return x;
    }
}

impl<const T: usize> Debug for Packet<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        let header = self.header;
        let checksum = self.checksum;
        let request = self.request;
        let response = self.response;
        let length = self.length;

        f.debug_struct("Packet")
            .field("header", &header)
            .field("checksum", &checksum)
            .field("request", &request)
            .field("response", &response)
            .field("length", &length)
            .field("status", &self.status)
            .finish()
    }
}
