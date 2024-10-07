use hide_qr::{QrExtended, Reader};
use qrcodegen::QrCodeEcc;
use rqrr::*;
pub fn print_help() {
    println!("Command Line Tool Usage:");
    println!("-create [txt-message1] [txt-message2] [file_location (optional)]: Creates a file at the specified location with two text messages.");
    println!("-read [file location] : Reads and displays the content of the specified file.");
    println!("-help : Displays this help message.");
}

pub fn create_qr_code(file_location: &str, message1: &str, message2: &str) {
    let mut qr = match QrExtended::encode_text(message1, QrCodeEcc::High) {
        Ok(qr) => qr,
        Err(_) => {
            panic!("Error DataTooLong: Data is too long to be stored in a QR-Code.");
        }
    };

    match qr.embed(message2) {
        Ok(()) => println!("Successfully embedded the secret message into the QR-Code!"),
        Err(e) => panic!(
            "{} Please shorten the secret message or increase the dummy data.",
            e,
        ),
    }

    match qr.bool_vector_to_png(file_location) {
        Ok(()) => println!("QR code saved successfully at '{}'.", file_location),
        Err(e) => eprintln!("{}", e),
    }
}

pub fn read_secret_message_from_qr_code_file(file_location: &str) {
    let img = image::open(file_location).unwrap().to_luma8();
    let mut img = PreparedImage::prepare(img);
    let grids = img.detect_grids();
    let (_, raw) = grids[0].get_raw_data().unwrap();
    let reader = Reader::from_raw_data(raw);
    println!("\nEmbedded secret message: {}", reader.read());
}

pub fn read_actual_code_from_qr_code_file(file_location: &str) {
    let img = image::open(file_location).unwrap().to_luma8();
    let mut img = PreparedImage::prepare(img);
    let grids = img.detect_grids();
    let (_, content) = grids[0].decode().unwrap();
    println!("The QR-Code holds the data: {}", content);
}
