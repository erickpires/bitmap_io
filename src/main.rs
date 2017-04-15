extern crate bitmap_io;

use bitmap_io::*;

use std::fs::File;

fn main() {

    if let Ok(mut bmp_file) = File::open("player.bmp") {
        let bitmap = Bitmap::from_file(&mut bmp_file).unwrap();

        println!("{}", bitmap.file_header);
        println!("{}", bitmap.info_header);


        let mut i = 0;
        for _ in 0 .. bitmap.info_header.image_height {
            for _ in 0 .. bitmap.info_header.image_width {
                print!("{} ", bitmap.image_data[i]);
                i += 1;
            }

            println!("");
        }

    }

    println!("Hello world");
}
