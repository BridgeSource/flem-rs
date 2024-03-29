use flem;
use flem::*;

use std::iter::FromIterator;

// Size of packet, including the Header (8 byte)
// So a size of 108 would leave 100 bytes for data
const FLEM_PACKET_SIZE: usize = 100;

pub mod host_requests {
    pub const GET_DATA: u16 = 10;
}

pub mod client_requests {
    // If the client is to make requests, this is a good place to define them
}

fn main() {
    let client_flem_id = DataId::new("Example Project 25 chars.", 0, 1, 0, FLEM_PACKET_SIZE);

    // There should typically be at least 1 packet each for Rx / Tx. You can
    // also look into heapless Queues for embedded.
    let mut host_tx = flem::Packet::<FLEM_PACKET_SIZE>::new();
    let mut host_rx = flem::Packet::<FLEM_PACKET_SIZE>::new();

    // There should typically be at least 1 packet each for Rx / Tx. You can
    // also look into heapless Queues for embedded.
    let mut client_tx = flem::Packet::<FLEM_PACKET_SIZE>::new();
    let mut client_rx = flem::Packet::<FLEM_PACKET_SIZE>::new();

    println!("Packet data length: {}", client_rx.get_data().len());

    println!("Packet length: {}", client_rx.length());

    host_tx.reset();
    host_tx.set_request(flem::request::ID); // Change this for different responses from the client
    host_tx.pack(); // Pack runs checksum and after that it is ready to send

    // Simulates byte-by-byte tranmission
    for _i in 0..host_tx.length() {
        let mut next_byte: u8 = 0;
        match host_tx.get_byte() {
            Ok(byte) => {
                next_byte = byte;
            }
            Err(_) => {
                assert!(false, "get_byte() finished");
            }
        }

        /* Hardware bus / protocol (I2C, UART, etc) goes here */

        //Transmit from host / receive on client
        match client_rx.construct(next_byte) {
            Ok(_) => {
                println!("Packet received successfully!");
            }
            Err(status) => {
                if status != Status::PacketBuilding {
                    println!("Packet error occurred!");
                }
            }
        }
    }
    host_tx.reset_lazy(); // Reset the host_tx so it can be used again

    /* Process request on the client side */
    client_tx.reset_lazy();
    match client_rx.get_request() {
        request::ID => {
            client_tx.pack_id(&client_flem_id, true).unwrap();
        }
        host_requests::GET_DATA => {
            // Custom command implemented for this project (Project X)
            let project_x_data = [0 as u8; 40];
            client_tx
                .pack_data(client_rx.get_request(), &project_x_data)
                .unwrap_or_else(|error| {
                    println!("Error packing the data with code: {:?}", error);
                });
            println!("Request received: FlemRequestProjectX::GET_DATA");
        }
        _ => {
            client_tx
                .pack_error(
                    client_rx.get_request(),
                    flem::response::UNKNOWN_REQUEST,
                    &[],
                )
                .unwrap_or_else(|error| {
                    println!("Error packing the error with code: {:?}", error);
                });
        }
    }
    client_rx.reset_lazy(); // Reset the client_rx packet so it can be used again

    /* Send response back to host */
    for byte in client_tx.bytes() {
        // ** Byte is transmitting over hardware **

        // ** Byte received by host, construct the
        match host_rx.construct(*byte) {
            Ok(_) => {
                // Determine what to do with the received packet
                match host_rx.get_request() {
                    request::ID => {
                        let host_size_data_id = flem::DataId::from(&host_rx.get_data()).unwrap();
                        println!(
                            "DataId Message: {}, max packet size: {}, Major: {}, Minor: {}, Patch: {}", 
                            String::from_iter(host_size_data_id.get_name().iter()),
                            host_size_data_id.get_max_packet_size(),
                            host_size_data_id.get_major(),
                            host_size_data_id.get_minor(),
                            host_size_data_id.get_patch()
                        );
                    }
                    host_requests::GET_DATA => {
                        // Custom command implemented for this project (Project X)
                        // Do something with the requested data
                    }
                    _ => {
                        // Uh oh
                    }
                }

                host_rx.reset_lazy(); // Reset the host_rx so it can be used again
            }
            Err(status) => {
                /* Catch other errors here */

                if status != Status::PacketBuilding {
                    println!("Packet error occurred!");
                    // Usually good to reset the packet after an issue
                    host_rx.reset_lazy();
                }
            }
        }
    }
    client_tx.reset_lazy(); // Reset the client_tx so it can be used again
}
