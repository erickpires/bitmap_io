extern crate bitmap_io;

use bitmap_io::*;

use std::fs::File;

fn main() {

    if let Ok(mut bmp_file) = File::open("test.bmp") {
        let bitmap = Bitmap::from_file(&mut bmp_file).unwrap();

        println!("{}", bitmap.file_header);
        println!("{}", bitmap.info_header);

    }

    println!("Hello world");
}
