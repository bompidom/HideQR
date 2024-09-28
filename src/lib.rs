use std::{fs, io};
use std::io::Error;
use qrcodegen::*;
use image::{ImageBuffer, RgbImage, Rgb};
use rqrr::*;
use tempfile::NamedTempFile;

//The stenography is based on expanding functionality of Library QrCode
//which firstly lacked fundamental getter Methods, which are expanded by this trait
pub trait QrCodeFunctionalityExpansion{
    fn get_modules(&self) -> Vec<bool>;
}

impl QrCodeFunctionalityExpansion for QrCode{
    fn get_modules(&self) -> Vec<bool> {
        let mut m: Vec<bool> = vec![];
        for y in 0 .. self.size() {
            for x in 0 .. self.size() {
                m.push(self.get_module(x,y));
            }
        }
        m
    }
}

pub struct QrExtended{
    data: String,
    inner: QrCode,
    flat_vector_modules: Vec<bool>, //code modules stored as flat vector of booleans
    alignment_positions: Vec<(i32, i32)>,
}

/*
    A secret message is to be embedded by modifying modules (pixel) of the already valid QR Code in a specific pattern.
    Because of error correction, we are allowed to alter the code while it still being readable.

    Modification are hold by wrapper QrExtended, which has QrCode as wrappee.
*/
impl QrExtended {
    
    //Constructor wrapping constructor of QRCode library
    pub fn encode_text(text: &str, ecl: QrCodeEcc) -> Result<Self,DataTooLong> {
        match QrCode::encode_text(text, ecl){
            Ok(qr) =>
                Ok(QrExtended {
                    data: text.to_string(),
                    inner: qr,
                    flat_vector_modules: vec![],
                    alignment_positions: vec![],
                }),
            Err(e) => Err(e),
        }
    }

    /*
    Embedding bits in a QR code can be done in many different patterns. The simplest way is to place the bits in order, one after the other.
    Although this method is basic and easy to decode, it still works for embedding data.
    */

    pub fn embed(&mut self, text: &str) -> Result<(), Error> {
        self.load_pre_modified_code();
        self.populate_alignment_positions();

        let mut to_be_inserted_bits = Self::ascii_to_bits(text);
        let mut coords = self.zigzag_coordinates();

        //embed message length at appropriate location in qr code in bytes
        let mut byte_len_information: Vec<bool> = Self::u8_to_bool_vector(text.len() as u8);
        
        for i in 3..7{
            self.set_module(self.size() - 1, self.size() - i,  byte_len_information.remove(0));
            self.set_module(self.size() - 2, self.size() - i,  byte_len_information.remove(0));
        }
        
        //embedding message
        for _ in 0..coords.len() {
            let (x, y) = coords.remove(0);

            if to_be_inserted_bits.is_empty() {
                break; // No more bits to insert
            }

            // Check if there are coordinates available before trying to set a module
            if !coords.is_empty() {
                self.set_module(x, y, to_be_inserted_bits.remove(0));
            }
        }

        println!("QR Code generated. Checking readability...");

        //Check readability of original code and secret message
        self.check_readability(text)
    }

    fn get_altered_module_at(&self, x: i32, y: i32) -> bool {
        (0 .. self.size()).contains(&x) && (0 .. self.size()).contains(&y) && self.flat_vector_modules[(y * self.size() + x) as usize]
    }

    pub fn print_qr_pre_modification(&self){
        self.print_with(|x, y| self.inner.get_module(x, y));
    }

    pub fn print_qr_post_modification(&self){
        self.print_with(|x,y| self.get_altered_module_at(x, y));
    }

    //Helper function to avoid boilerplate code of printing post and pre QR code
    fn print_with<F>(&self, get_module_fn: F)
    where
        F: Fn(i32, i32) -> bool,
    {
        let border: i32 = 3; //space around the QR_code
        for y in -border..self.size() + border {
            for x in -border..self.size() + border {
                let c: char = if get_module_fn(x, y) { '█' } else { ' ' };
                print!("{0}{0}", c);
            }
            println!();
        }
        println!();
    }

    fn size(&self) -> i32{
        self.inner.size()
    }

    fn version(&self) -> qrcodegen::Version{
        self.inner.version()
    }

    fn load_pre_modified_code(&mut self){
        self.flat_vector_modules.append(&mut self.inner.get_modules());
    }

    fn set_module(&mut self, x: i32, y: i32, value: bool){
        let index = (y * self.size() + x) as usize;
        self.flat_vector_modules[index] = value;
    }

    /*
    The High Error Correction method used here provides up to 30% correction capability.
    This implies that the maximum length of the secret message could be 30% of the original data.
    However, due to the chance of matching bits, it’s possible for more than 30% to fit.
    Thus, this function checks that the generated code maintains readability for both the actual and secret data before saving it successfully.
    */
    fn check_readability(&self, secret_data: &str) -> Result<(), Error> {
        // Create temporary file holding the QR code
        let temp_file = NamedTempFile::new()?;
        let mut temp_path = temp_file.path().to_string_lossy().to_string();
        temp_path.push_str(".png");

        // Convert bool vector to PNG and handle potential errors
        self.bool_vector_to_png(&temp_path)
            .map_err(|e| Error::new(io::ErrorKind::Other, format!("Failed to create PNG: {}", e)))?;

        // Prepare check
        let img = image::open(&temp_path)
            .map_err(|e| Error::new(io::ErrorKind::InvalidData, format!("Failed to open image: {}", e)))?
            .to_luma8();

        let mut img = PreparedImage::prepare(img);
        let grids = img.detect_grids();

        // Test readability of original data
        let (_, content) = grids.first()
            .ok_or_else(|| Error::new(io::ErrorKind::InvalidData, "No grids detected."))?
            .decode()
            .map_err(|e| Error::new(io::ErrorKind::InvalidData, format!("Failed to decode original data: {}", e)))?;

        if self.data != content {
            return Err(Error::new(io::ErrorKind::InvalidData, "The original data of QR-code cannot be read."));
        } else {
            println!("The original data is still readable!");
        }

        // Test readability of secret data
        let (_, raw) = grids.first()
            .ok_or_else(|| Error::new(io::ErrorKind::InvalidData, "No grids detected."))?
            .get_raw_data()
            .map_err(|e| Error::new(io::ErrorKind::InvalidData, format!("Failed to get raw data: {}", e)))?;

        let reader = Reader::from_raw_data(raw);
        if reader.read() != secret_data {
            return Err(Error::new(io::ErrorKind::InvalidData, "The secret data of QR-code cannot be read."));
        } else {
            println!("The secret message is readable!");
        }

        // If the original and secret data are readable, delete the temporary QR code
        fs::remove_file(&temp_path).map_err(|e| Error::new(io::ErrorKind::Other, format!("Failed to delete temporary file: {}", e)))?;

        Ok(())
    }

        /*
        Helper Methods for determining if module touchable

        When inserting a secret code, it is important not to overwrite key QR code patterns.
        For example, changing the finder patterns (the large squares in the corners) would make it obvious that the code has been modified.

        To learn more about where these patterns are located, visit:
        https://scanova.io/blog/qr-code-structure/
        */
    fn module_can_be_overwritten(&self, x: i32, y: i32) -> bool{
        !self.is_finder_pattern(x, y) && !self.is_timing_pattern(x, y) && !self.is_alignment_pattern(x, y) && !self.is_format_pattern(x, y) && !self.is_version_pattern(x, y) && !self.is_length_or_mode_pattern(x, y)
    }

    fn is_finder_pattern(&self, x: i32, y: i32) -> bool{
        (x < 8 && y < 8) || (x < 8 && y >= self.size() - 8) || (x >= self.size() -8 && y < 8 )
    }

    fn is_timing_pattern(&self, x: i32, y: i32) -> bool{
        x == 6 || y == 6
    }

    fn is_format_pattern(&self, x: i32, y: i32) -> bool{
        (x == 8 && (y <= 8 || y >= self.size()-8)) || (y == 8 && (x <= 8 || x >= self.size()-8))
    }

    fn is_version_pattern(&self, x: i32, y: i32) -> bool {
        let bottom_left_version = (0..=5).contains(&x) && (y >= self.size() - 11 && y <= self.size() - 9);
        let top_right_version = (x >= self.size() - 11 && x <= self.size() - 9) && (0..=5).contains(&y);

        bottom_left_version || top_right_version
    }


    fn is_alignment_pattern(&self, x: i32, y: i32) -> bool{
        //checking if pixel is within 2 modules in x and y direction of center of alignment pattern
        for coord in &self.alignment_positions {
            if Self::module_distance(x, coord.0) <= 2 && Self::module_distance(y, coord.1) <= 2{
                return true;
            }
        }

        false
    }
    
    fn is_length_or_mode_pattern(&self, x: i32, y: i32) -> bool{
        (y >= self.size() - 6 && y <= self.size()) && (x == self.size()-1 || x == self.size() - 2)
    }

    fn module_distance(x1: i32, x2: i32) -> i32{
        if x1 >=  x2 {
            return x1 - x2;
        }
        x2 - x1
    }

    fn get_potential_alignment_positions(&self) -> Vec<i32> {
        let mut alignment_pos: Vec<i32> = Vec::new();

        if self.version().value() <= 1{
            return alignment_pos;
        }


        let interval: i32 = ((self.version().value() / 7) + 1) as i32;
        let distance: i32 = (4 * self.version().value() + 4) as i32;
        let mut step = (distance as f64 / interval as f64).round() as i32;
        step = if step % 2 != 0 { step + 1 } else { step }; // rounding step to the next largest even number

        alignment_pos.push(6); // push the first value to the vector
        for i in 1..interval+1 {
            alignment_pos.push(6 + distance - step * (interval - i));
        }
        alignment_pos
    }

    /*
    The alignment pattern (a small square with a dot inside) has varying positions depending on the version of the QR code
    Calculating these positions may yield invalid positions that need to be filtered out
    Once calculated the valid positions are stored in "self.alignment_positions" for the lifetime of the QR code
    */
    fn populate_alignment_positions(&mut self){
        let v = self.get_potential_alignment_positions();

        self.alignment_positions = v.iter()
            .flat_map(|&x| v.iter().map(move |&y| (x, y)))
            .filter(|e| !self.is_finder_pattern(e.0, e.1)) //center of potential alignment pattern cannot overlap other patterns
            .collect();
    }

    fn u8_to_bool_vector(byte: u8) -> Vec<bool> {
        let mut bits = Vec::new();

        for i in (0..8).rev() {
            bits.push((byte >> i) & 1 != 0);
        }

        bits
    }
    
    fn ascii_to_bits(ascii: &str) -> Vec<bool> {
        let mut bits = Vec::new();

        for byte in ascii.bytes() {
            for i in 0..8 {
                bits.push((byte & (1 << (7 - i))) != 0);
            }
        }
        bits
    }

    fn zigzag_coordinates(&self) -> Vec<(i32, i32)> {
        let mut coords = Vec::new(); // Vector to store (x, y) coordinates in zigzag pattern
        let mut right: i32 = self.size() - 1; // Start at the rightmost column

        // Traverse the QR code grid in a zigzag pattern
        while right >= 1 {
            if right == 6 {
                right = 5; // Skip column 6 (timing pattern)
            }
            for vert in 0..self.size() { // Iterate vertically through the columns
                for j in 0..2 {
                    let x = right - j; // Current x-coordinate (either `right` or `right - 1`)
                    let upward = (right + 1) & 2 == 0; // Direction of traversal (upward or downward)
                    let y = if upward { self.size() - 1 - vert } else { vert }; // Compute y based on direction

                    // Skip function modules (e.g., finder patterns, alignment patterns)
                    if  self.module_can_be_overwritten(x, y){
                        coords.push((x, y)); // Add (x, y) to the result vector
                    }
                }
            }
            right -= 2; // Move to the next pair of columns to the left
        }

        coords // Return the vector of coordinates
    }

    pub fn bool_vector_to_png(&self, path: &str) -> Result<(), String> {
        let width = self.size() as u32;
        let module_size = 10;
        let border = 20;

        let img_width = width * module_size + border * 2;
        let mut img: RgbImage = ImageBuffer::from_pixel(img_width, img_width, Rgb([255, 255, 255]));

        for y in 0..width {
            for x in 0..width {
                let color = if self.get_altered_module_at(x as i32, y as i32) {
                    Rgb([0, 0, 0]) // black for dark modules
                } else {
                    Rgb([255, 255, 255]) // white for light modules
                };

                // Fill the corresponding module area with the color
                for dy in 0..module_size {
                    for dx in 0..module_size {
                        let pixel_x = x * module_size + dx + border;
                        let pixel_y = y * module_size + dy + border;

                        // Error handling for pixel placement
                        if pixel_x >= img.width() || pixel_y >= img.height() {
                            return Err(format!(
                                "Error: Attempted to access pixel out of bounds at ({}, {}).",
                                pixel_x, pixel_y
                            ));
                        }

                        img.put_pixel(pixel_x, pixel_y, color);
                    }
                }
            }
        }

        img.save(path).map_err(|e| format!("Error: The QR-Code could not be saved at specified path '{}': {}", path, e))?;

        Ok(())
    }
    
}

pub struct Reader{
    modules: Vec<bool>,
}

impl Reader{
    pub fn from_raw_data(raw: RawData) -> Self{
        let mut bit_vector = Self::bytes_to_bits(&raw.data);
        let len = (Self::bools_to_decimal(&bit_vector[4..12]) * 8) as usize;
        bit_vector.drain(0..12); //are not code
        bit_vector.drain(len..); //drain modules that are not part of secret message
        
        Reader{
            modules: bit_vector,
        }
    }

    
    pub fn read(&self) -> String{
        Self::bits_to_ascii(&self.modules)
    }


    fn bools_to_decimal(bits: &[bool]) -> u8 {
        let mut value = 0;

        for (i, &bit) in bits.iter().enumerate() {
            if bit {
                value += 1 << (bits.len() - 1 - i);
            }
        }

        value
    }

    fn bytes_to_bits(bytes: &[u8]) -> Vec<bool> {
        let mut bits = Vec::new();

        for byte in bytes {
            for i in (0..8).rev() {
                bits.push(((byte >> i) & 1u8) != 0u8);
            }
        }

        bits
    }

    fn bits_to_ascii(bits: &[bool]) -> String {
        let mut result = String::new();

        for chunk in bits.chunks(8) {
            if chunk.len() < 8 {
                break;
            }

            let byte = chunk.iter()
                .enumerate()
                .map(|(i, &bit)| if bit { 1 << (7 - i) } else { 0 })
                .sum::<u8>();

            result.push(byte as char); // Convert byte to char and append to result
        }

        result
    }
}