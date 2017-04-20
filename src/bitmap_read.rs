use BytesWalker;
use BitmapPixel;
use BitmapPalette;
use mask_offset_and_shifted;

use map_zero_based;

#[macro_export]
macro_rules! pad_to_align {
    ($value:expr, $alignment:expr) => (
        ($alignment - ($value % $alignment)) % $alignment
    )
}

pub fn read_32_bitfield(data_walker: &mut BytesWalker,
                    result: &mut Vec<BitmapPixel>,
                    red_mask: u32,
                    green_mask: u32,
                    blue_mask: u32,
                    alpha_mask: u32) {
    let (red_offset,   _)   = mask_offset_and_shifted(red_mask);
    let (green_offset, _) = mask_offset_and_shifted(green_mask);
    let (blue_offset,  _)  = mask_offset_and_shifted(blue_mask);
    let (alpha_offset, _) = mask_offset_and_shifted(alpha_mask);

    while data_walker.has_data() {
        let pixel_value = data_walker.next_u32();

        let mut pixel = BitmapPixel {
            blue  : ((pixel_value  >> blue_offset)  & 0xff) as u8,
            green : ((pixel_value  >> green_offset) & 0xff) as u8,
            red   : ((pixel_value  >> red_offset)   & 0xff) as u8,
            alpha : ((pixel_value  >> alpha_offset) & 0xff) as u8,
        };

        if alpha_mask == 0x00 {
            // NOTE(erick): We are in XRGB mode.
            pixel.alpha = 0xff;
        }

        result.push(pixel);
    }
}

pub fn read_16_bitfield(data_walker: &mut BytesWalker,
                        result: &mut Vec<BitmapPixel>,
                        image_width: i32,
                        red_mask: u32,
                        green_mask: u32,
                        blue_mask: u32,
                        alpha_mask: u32) {
    let (red_offset,   red_shifted)   = mask_offset_and_shifted(red_mask);
    let (green_offset, green_shifted) = mask_offset_and_shifted(green_mask);
    let (blue_offset,  blue_shifted)  = mask_offset_and_shifted(blue_mask);
    let (alpha_offset, alpha_shifted) = mask_offset_and_shifted(alpha_mask);

    let mut column_index = 0;
    while data_walker.has_data() {
        if column_index == image_width {
            // NOTE(erick): We have to align rows to
            // 4 bytes values.
            column_index = 0;
            data_walker.align_with_u32();
        }

        // NOTE(erick): The file can have some padding at the end.
        if !data_walker.has_data() {
            break;
        }

        let pixel_value = data_walker.next_u16() as u32;

        let mut pixel = BitmapPixel {
            blue  : ((pixel_value & blue_mask)  >> blue_offset)  as u8,
            green : ((pixel_value & green_mask) >> green_offset) as u8,
            red   : ((pixel_value & red_mask)   >> red_offset)   as u8,
            alpha : ((pixel_value & alpha_mask) >> alpha_offset) as u8,
        };

        map_zero_based(&mut pixel.red   , red_shifted, 0xff);
        map_zero_based(&mut pixel.green , green_shifted, 0xff);
        map_zero_based(&mut pixel.blue  , blue_shifted, 0xff);
        map_zero_based(&mut pixel.alpha , alpha_shifted, 0xff);

        if alpha_mask == 0x00 {
            // NOTE(erick): We are in XRGB mode.
            pixel.alpha = 0xff;
        }

        result.push(pixel);
        column_index += 1;
    }
}

pub fn read_32_uncompressed(data_walker: &mut BytesWalker,
                            result: &mut Vec<BitmapPixel>) {
    // NOTE(erick): We only have alpha when the
    // compression_type is BitFields. The last byte is
    // here only for padding.
    while data_walker.has_data() {
        let pixel = BitmapPixel {
            blue  : data_walker.next_u8(),
            green : data_walker.next_u8(),
            red   : data_walker.next_u8(),
            alpha : 0xff,
        };
        // NOTE(erick): We have to discard the padding byte.
        data_walker.next_u8();
        result.push(pixel);
    }
}

pub fn read_24_uncompressed(data_walker: &mut BytesWalker,
                            result: &mut Vec<BitmapPixel>,
                            image_width: i32) {
    let mut column_index = 0;
    while data_walker.has_data() {
        if column_index == image_width {
            // NOTE(erick): We have to align rows to
            // 4 bytes values.
            column_index = 0;
            data_walker.align_with_u32();

            // NOTE(erick): The file can have some padding at the end.
            if !data_walker.has_data() {
                break;
            }
        }

        let pixel = BitmapPixel {
            blue  : data_walker.next_u8(),
            green : data_walker.next_u8(),
            red   : data_walker.next_u8(),
            alpha : 0xff,
        };

        result.push(pixel);
        column_index += 1;
    }
}

pub fn read_16_uncompressed(data_walker: &mut BytesWalker,
                            result: &mut Vec<BitmapPixel>,
                            image_width: i32) {
    let mut column_index = 0;
    while data_walker.has_data() {
        if column_index == image_width {
            // NOTE(erick): We have to align rows to
            // 4 bytes values.
            column_index = 0;
            data_walker.align_with_u32();

            // NOTE(erick): The file can have some padding at the end.
            if !data_walker.has_data() {
                break;
            }
        }

        let pixel_data = data_walker.next_u16();
        let mut pixel = BitmapPixel {
            blue  : (pixel_data & 0x1f) as u8,
            green : ((pixel_data >> 5)   & 0x1f) as u8,
            red   : ((pixel_data  >> 10) & 0x1f) as u8,
            alpha : 0xff,
        };

        map_zero_based(&mut pixel.red, 0x1f, 0xff);
        map_zero_based(&mut pixel.green, 0x1f, 0xff);
        map_zero_based(&mut pixel.blue, 0x1f, 0xff);

        result.push(pixel);
        column_index += 1;
    }
}

pub fn read_8_uncompressed(data_walker: &mut BytesWalker,
                           result: &mut Vec<BitmapPixel>,
                           image_width: i32,
                           image_palette: &BitmapPalette) {
    let mut column_index = 0;
    while data_walker.has_data() {
        if column_index == image_width {
            column_index = 0;
            data_walker.align_with_u32();

            if !data_walker.has_data() {
                break;
            }
        }
        let pixel_index = data_walker.next_u8() as usize;
        let pixel = image_palette[pixel_index];

        result.push(pixel);
        column_index += 1;
    }
}

pub fn read_4_uncompressed(data_walker: &mut BytesWalker,
                           result: &mut Vec<BitmapPixel>,
                           image_width: i32,
                           image_palette: &BitmapPalette) {
    let mut column_index = 0;
    while data_walker.has_data() {
        if column_index >= image_width {
            column_index = 0;
            data_walker.align_with_u32();

            if !data_walker.has_data() {
                break;
            }
        }
        let pixels_indexes = data_walker.next_u8();
        let p0_index = (pixels_indexes >> 4) as usize;
        let p1_index = (pixels_indexes & 0x0f) as usize;

        let pixel0 = image_palette[p0_index];
        let pixel1 = image_palette[p1_index];

        result.push(pixel0);
        column_index += 1;

        if column_index < image_width {
            result.push(pixel1);
            column_index += 1;
        }
    }
}

pub fn read_1_uncompressed(data_walker: &mut BytesWalker,
                           result: &mut Vec<BitmapPixel>,
                           image_width: i32, image_height: i32,
                           image_palette: &BitmapPalette) {
    for _ in 0 .. image_height {
        let mut column_index = 0;
        for _ in 0 .. image_width / 8 {
            let pixels_byte = data_walker.next_u8();
            append_pixels_from_byte(&image_palette,
                                    result,
                                    pixels_byte, 8);

            column_index += 8;
        }

        let remaining_pixels = image_width - column_index;
        if remaining_pixels > 0 {
            let pixels_byte = data_walker.next_u8();
            append_pixels_from_byte(&image_palette,
                                    result,
                                    pixels_byte,
                                    remaining_pixels);
        }

        data_walker.align_with_u32()
    }
}

fn append_pixels_from_byte(palette: &BitmapPalette,
                        vec:&mut Vec<BitmapPixel>,
                        byte: u8, n_bits: i32) {
    let mut mask = 0x80;

    for _ in 0 .. n_bits {
        if byte & mask == 0 {
            vec.push(palette[0]);
        } else {
            vec.push(palette[1]);
        }

        mask = mask >> 1;
    }
}
