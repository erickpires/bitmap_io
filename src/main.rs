extern crate bitmap_io;

use bitmap_io::*;

use std::fs::File;

fn main() {

    if let Ok(mut bmp_file) = File::open("test2.bmp") {
        let mut bitmap = Bitmap::from_file(&mut bmp_file).unwrap();

        println!("{}", bitmap.file_header);
        println!("{}", bitmap.info_header);

        // println!("Image has: {} pixels", bitmap.image_data.len());
        // let mut i = 0;
        // for _ in 0 .. bitmap.info_header.image_height {
        //     for _ in 0 .. bitmap.info_header.image_width {
        //         print!("{} ", bitmap.image_data[i]);
        //         i += 1;
        //     }

        //     println!("");
        // }

        bitmap.convert_to_bitfield_compression();
        bitmap.color_to_alpha(BitmapPixel::rgb(255, 0, 255));
        bitmap.mirror_vertically();
        bitmap.mirror_horizontally();
        // bitmap.crop_to_rect()

        if let Ok(mut out_file) = File::create("blah_test.bmp") {
            bitmap.into_file(&mut out_file);
        }

        if let Ok(mut out_file) = File::create("black.bmp") {
            let bitmap = Bitmap::new(150, 10);
            bitmap.into_file(&mut out_file);
        }

    }

    println!("Hello world");
}
