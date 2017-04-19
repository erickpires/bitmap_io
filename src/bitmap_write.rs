use BitmapPixel;
use BitmapPalette;
use mask_offset_and_shifted;

use map_zero_based;

pub fn write_32_bitfield(data: &mut Vec<u8>, pixels: &Vec<BitmapPixel>,
                         red_mask: u32, green_mask: u32,
                         blue_mask: u32, alpha_mask: u32) {
    let (red_offset, _)   = mask_offset_and_shifted(red_mask);
    let (green_offset, _) = mask_offset_and_shifted(green_mask);
    let (blue_offset, _)  = mask_offset_and_shifted(blue_mask);
    let (alpha_offset, _) = mask_offset_and_shifted(alpha_mask);

    for pixel in pixels {
        let pixel_value : u32 =
            (pixel.red as u32)   << red_offset   |
        (pixel.green as u32) << green_offset |
        (pixel.blue  as u32) << blue_offset  |
        (pixel.alpha as u32) << alpha_offset & alpha_mask;
        // NOTE(erick): we and with alpha_mask so we can support argb and
        // xrgb at the same time.

        push_u32(data, pixel_value);
    }
}

pub fn write_16_bitfield(data: &mut Vec<u8>, pixels: &Vec<BitmapPixel>,
                         image_width: i32, image_height: i32,
                         red_mask: u32, green_mask: u32,
                         blue_mask: u32, alpha_mask: u32) {
    let (red_offset, red_shifted)   = mask_offset_and_shifted(red_mask);
    let (green_offset, green_shifted) = mask_offset_and_shifted(green_mask);
    let (blue_offset, blue_shifted)  = mask_offset_and_shifted(blue_mask);
    let (alpha_offset, alpha_shifted) = mask_offset_and_shifted(alpha_mask);

    let mut pixel_iter = pixels.into_iter();

    let bytes_per_row = image_width * 2;
    let n_padding_bytes = pad_to_align!(bytes_per_row, 4);

    for _ in 0 .. image_height {
        for _ in 0 .. image_width {
            let mut pixel = pixel_iter.next().unwrap().clone();

            map_zero_based(&mut pixel.red, 0xff, red_shifted);
            map_zero_based(&mut pixel.green, 0xff, green_shifted);
            map_zero_based(&mut pixel.blue, 0xff, blue_shifted);
            map_zero_based(&mut pixel.alpha, 0xff, alpha_shifted);

            let pixel_value : u16 =
                (pixel.red as u16)   << red_offset   |
            (pixel.green as u16) << green_offset |
            (pixel.blue  as u16) << blue_offset  |
            (pixel.alpha as u16) << alpha_offset & alpha_mask as u16;
            // NOTE(erick): We and with alpha_mask so we can support ARGB and
            // XRGB at the same time.

            push_u16(data, pixel_value);
        }

        for _ in 0 .. n_padding_bytes {
            data.push(0x00);
        }
    }
}

pub fn write_32_uncompressed(data: &mut Vec<u8>, pixels: &Vec<BitmapPixel>) {
    for pixel in pixels {
        data.push(pixel.blue);
        data.push(pixel.green);
        data.push(pixel.red);
        data.push(0x00); // Padding
    }
}

pub fn write_24_uncompressed(data: &mut Vec<u8>, pixels: &Vec<BitmapPixel>,
                             image_width: i32, image_height: i32) {
    let mut pixel_iter = pixels.into_iter();

    let bytes_per_row = image_width * 3;
    let n_padding_bytes = pad_to_align!(bytes_per_row, 4);

    for _ in 0 .. image_height {
        for _ in 0 .. image_width {
            let pixel = pixel_iter.next().unwrap();

            data.push(pixel.blue);
            data.push(pixel.green);
            data.push(pixel.red);
        }

        for _ in 0 .. n_padding_bytes {
            data.push(0x00);
        }
    }
}

pub fn write_8_uncompressed(data: &mut Vec<u8>, pixels: &Vec<BitmapPixel>,
                             image_palette: &BitmapPalette,
                             image_width: i32, image_height: i32) {
    let mut pixel_iter = pixels.into_iter();

    let n_padding_bytes = pad_to_align!(image_width, 4);

    for _ in 0 .. image_height {
        for _ in 0 .. image_width {
            let pixel = pixel_iter.next().unwrap();

            let palette_index = pixel.find_closest_by_index(image_palette) as u8;
            data.push(palette_index);
        }

        for _ in 0 .. n_padding_bytes {
            data.push(0x00);
        }
    }
}

pub fn write_4_uncompressed(data: &mut Vec<u8>, pixels: &Vec<BitmapPixel>,
                            image_palette: &BitmapPalette,
                            image_width: i32, image_height: i32) {
    let mut pixel_iter = pixels.into_iter();

    let bytes_per_row = (image_width + 1) / 2;
    let n_padding_bytes = pad_to_align!(bytes_per_row, 4);

    for _ in 0 .. image_height {
        let mut pixels_written = 0;
        for _ in 0 .. image_width / 2 {
            let pixel0 = pixel_iter.next().unwrap();
            let pixel1 = pixel_iter.next().unwrap();

            let p0_index = pixel0.find_closest_by_index(image_palette) as u8;
            let p1_index = pixel1.find_closest_by_index(image_palette) as u8;

            let pixel_data = (p0_index << 4) | (p1_index & 0x0f);
            data.push(pixel_data);
            pixels_written += 2;
        }

        // NOTE(erick): We still have one pixel to write.
        if pixels_written < image_width {
            let pixel = pixel_iter.next().unwrap();
            let p_index = pixel.find_closest_by_index(image_palette) as u8;

            let pixel_data = p_index << 4;
            data.push(pixel_data);
        }

        for _ in 0 .. n_padding_bytes {
            data.push(0x00);
        }
    }
}

pub fn write_1_uncompressed(data: &mut Vec<u8>, pixels: &Vec<BitmapPixel>,
                            image_palette: &BitmapPalette,
                            image_width: i32, image_height: i32) {
    let pixels_slice = pixels.as_slice();

    let remaining_pixels_per_row = (image_width -
                                    (image_width / 8) * 8) as usize;

    let bits_per_row =
        image_width + pad_to_align!(image_width, 8);
    let bytes_per_row = bits_per_row / 8;
    let n_padding_bytes = pad_to_align!(bytes_per_row, 4);

    let mut total_pixels_written = 0;
    for _ in 0 .. image_height {
        for _ in 0 .. image_width / 8 {
            let pixels_block = &pixels_slice[total_pixels_written ..
                                             total_pixels_written + 8];

            let byte_data = byte_from_pixels(image_palette, pixels_block);
            data.push(byte_data);

            total_pixels_written += 8;
        }

        if remaining_pixels_per_row != 0 {
            let pixels_block = &pixels_slice[total_pixels_written ..
                                             total_pixels_written +
                                             remaining_pixels_per_row];

            let byte_data = byte_from_pixels(image_palette, pixels_block);
            data.push(byte_data);
            total_pixels_written += remaining_pixels_per_row;
        }

        for _ in 0 .. n_padding_bytes {
            data.push(0x00);
        }
    }

}

pub fn push_u32(v: &mut Vec<u8>, value: u32) {
    // NOTE(erick): Little-endian.
    v.push((value >>  0) as u8);
    v.push((value >>  8) as u8);
    v.push((value >> 16) as u8);
    v.push((value >> 24) as u8);
}
pub fn push_i32(v: &mut Vec<u8>, value: i32) {
    // NOTE(erick): Little-endian.
    v.push((value >>  0) as u8);
    v.push((value >>  8) as u8);
    v.push((value >> 16) as u8);
    v.push((value >> 24) as u8);
}
pub fn push_u16(v: &mut Vec<u8>, value: u16) {
    // NOTE(erick): Little-endian.
    v.push((value >>  0) as u8);
    v.push((value >>  8) as u8);
}

fn byte_from_pixels(palette: &BitmapPalette, pixels: &[BitmapPixel]) -> u8 {
    let mut mask = 0x80;
    let mut result = 0;

    for pixel in pixels {
        let p_index = pixel.find_closest_by_index(palette);
        if p_index != 0 {
            result |= mask;
        }

        mask = mask >> 1;
    }

    result
}
