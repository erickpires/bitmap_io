extern crate bitmap_io;

use bitmap_io::*;

use std::fs::File;

#[allow(unused_must_use)]
fn main() {

    let mut bmp_file = File::open("test_24-uncompressed.bmp").unwrap();
    let test_24_uncompressed = Bitmap::from_file(&mut bmp_file).unwrap();

    let mut bmp_file = File::open("test_32-uncompressed.bmp").unwrap();
    let test_32_uncompressed = Bitmap::from_file(&mut bmp_file).unwrap();

    let mut bmp_file = File::open("test_32-bitfield.bmp").unwrap();
    let test_32_bitfield = Bitmap::from_file(&mut bmp_file).unwrap();

    let mut bmp_file = File::open("test_16-bitfield.bmp").unwrap();
    let mut test_16_bitfield = Bitmap::from_file(&mut bmp_file).unwrap();


    // let mut bmp_file = File::open("test.bmp").unwrap();
    // let bitmap = Bitmap::from_file(&mut bmp_file).unwrap();

    // println!("{}", bitmap.file_header);
    // println!("{}", bitmap.info_header);


    if let Ok(mut out_file) = File::create("test_24-uncompressed-result.bmp") {
        test_24_uncompressed.into_file(&mut out_file);
    }

    if let Ok(mut out_file) = File::create("test_32-uncompressed-result.bmp") {
        test_32_uncompressed.into_file(&mut out_file);
    }

    if let Ok(mut out_file) = File::create("test_32-bitfield-result.bmp") {
        test_32_bitfield.into_file(&mut out_file);
    }

    if let Ok(mut out_file) = File::create("test_16-bitfield-result.bmp") {
        test_16_bitfield.into_file(&mut out_file);
    }

    println!("Hello world");
}
