use std::fmt::Display;
use std::fmt::Formatter;

use std::io::Write;
use std::io::Read;
use std::fs::File;

use std::cmp::max;
use std::ops::Range;

use std::convert;
use std::intrinsics::transmute;

const BMP_MAGIC_NUMBER : u16 = 0x4d_42; // "MB": We are little-endian

const FILE_HEADER_SIZE : u32 = 14;
const INFO_HEADER_SIZE : u32 = 56;  // Basic header with color masks

#[derive(Debug)]
pub enum BitmapError {
    InvalidBitmap,
    UnsupportedInfoHeaderSize(u32),
    UnsupportedNumberOfPlanes(u16),
    UnsupportedCompressionType(CompressionType),
    InvalidOperation,
    BitmapIOError(std::io::Error),
}

impl convert::From<std::io::Error> for BitmapError {
    fn from(err: std::io::Error) -> BitmapError {
        BitmapError::BitmapIOError(err)
    }
}

type BitmapResult<T> = Result<T, BitmapError>;

#[derive(Debug)]
pub struct BitmapFileHeader {
    pub magic_number       : u16,
    pub file_size          : u32,
    pub reserved1          : u16,      //Must be zero
    pub reserved2          : u16,      // Must be zero
    pub pixel_array_offset : u32,
}

#[allow(dead_code)]
impl Display for BitmapFileHeader {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "BitmapFileheader: {{\n")?;
        write!(f, "\t magic_number: 0x{:x},\n", self.magic_number)?;
        write!(f, "\t file_size: {},\n", self.file_size)?;
        write!(f, "\t reserved1: {},\n", self.reserved1)?;
        write!(f, "\t reserved2: {},\n", self.reserved2)?;
        write!(f, "\t pixel_array_offset: {},\n", self.pixel_array_offset)?;
        write!(f, "}}")
    }
}

#[allow(dead_code)]
impl BitmapFileHeader {
    fn new(f_size: u32, p_offset: u32) -> BitmapFileHeader {
        BitmapFileHeader {
            magic_number       : BMP_MAGIC_NUMBER,
            file_size          : f_size,
            reserved1          : 0,      //Must be zero
            reserved2          : 0,      // Must be zero
            pixel_array_offset : p_offset,
        }
    }

    fn validate(&self) -> bool {
        self.magic_number == BMP_MAGIC_NUMBER &&
            self.reserved1 == 0 &&
            self.reserved2 == 0
    }

    fn from_data(data: &[u8]) -> BitmapFileHeader {
        let mut data_walker = BytesWalker::new(data);

        BitmapFileHeader {
            magic_number       : data_walker.next_u16(),
            file_size          : data_walker.next_u32(),
            reserved1          : data_walker.next_u16(),
            reserved2          : data_walker.next_u16(),
            pixel_array_offset : data_walker.next_u32(),
        }
    }

    fn into_data(&self, data: &mut Vec<u8>) {
        push_u16(data, self.magic_number);
        push_u32(data, self.file_size);
        push_u16(data, self.reserved1);
        push_u16(data, self.reserved2);
        push_u32(data, self.pixel_array_offset);
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum CompressionType {
    Uncompressed = 0x0000,
    Rle8         = 0x0001,
    Rle4         = 0x0002,
    BitFields    = 0x0003,
    Jpeg         = 0x0004,
    Png          = 0x0005,
    CMYK         = 0x000B,
    CmykRle8     = 0x000C,
    CmykRle4     = 0x000D,

    Invalid, // Should never happen
}

impl convert::From<u32> for CompressionType {
    fn from(num: u32) -> CompressionType {
        match num {
            0x0000 => CompressionType::Uncompressed,
            0x0001 => CompressionType::Rle8,
            0x0002 => CompressionType::Rle4,
            0x0003 => CompressionType::BitFields,
            0x0004 => CompressionType::Jpeg,
            0x0005 => CompressionType::Png,
            0x000B => CompressionType::CMYK,
            0x000C => CompressionType::CmykRle8,
            0x000D => CompressionType::CmykRle4,
            _      => CompressionType::Invalid,
        }
    }
}

// NOTE(erick): This is the simplest ImageInfoHeader possible.
// We will probably find BitmapInfoV5Header in the wild and
// should _probably_ handle then. The type of header can
// theoretically be determined my looking at the header size
// a.k.a., the first four bytes.
#[derive(Debug)]
pub struct BitmapInfoHeader {
    pub info_header_size   : u32,
    pub image_width        : i32,
    pub image_height       : i32,
    pub n_planes           : u16,
    pub bits_per_pixel     : u16,
    pub compression_type   : u32,
    pub image_size         : u32, // WARNING: May be zero on uncompressed images
    pub pixels_per_meter_x : i32,
    pub pixels_per_meter_y : i32,
    pub colors_used        : u32,
    pub colors_important   : u32,

    pub red_mask   : u32,
    pub green_mask : u32,
    pub blue_mask  : u32,
    pub alpha_mask : u32,

    // NOTE(erick): Variables that are not in the
    // actual Header
    pub is_top_down : bool,
}

#[allow(dead_code)]
impl Display for BitmapInfoHeader {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "BitmapFileheader: {{\n")?;

        write!(f, "\t info_header_size: {}\n"   , self.info_header_size)?;
        write!(f, "\t image_width: {}\n"        , self.image_width)?;
        write!(f, "\t image_height: {}\n"       , self.image_height)?;
        write!(f, "\t n_planes: {}\n"           , self.n_planes)?;
        write!(f, "\t bits_per_pixel: {}\n"     , self.bits_per_pixel)?;
        write!(f, "\t compression_type: {}\n"   , self.compression_type)?;
        write!(f, "\t image_size: {}\n"         , self.image_size)?;
        write!(f, "\t pixels_per_meter_x: {}\n" , self.pixels_per_meter_x)?;
        write!(f, "\t pixels_per_meter_y: {}\n" , self.pixels_per_meter_y)?;
        write!(f, "\t colors_used: {}\n"        , self.colors_used)?;
        write!(f, "\t colors_important: {}\n"   , self.colors_important)?;

        write!(f, "\t red_mask: 0x{:08x}\n"  , self.red_mask)?;
        write!(f, "\t green_mask: 0x{:08x}\n", self.green_mask)?;
        write!(f, "\t blue_mask: 0x{:08x}\n" , self.blue_mask)?;
        write!(f, "\t alpha_mask: 0x{:08x}\n", self.alpha_mask)?;

        write!(f, "}}")
    }
}

#[allow(dead_code)]
impl BitmapInfoHeader {
    fn new(h_size: u32, i_width: i32, i_height: i32, i_size: u32,
           bits_per_pixel: u16,
           compression: CompressionType) -> BitmapInfoHeader {
        BitmapInfoHeader {
            info_header_size   : h_size,
            image_width        : i_width,
            image_height       : i_height,
            n_planes           : 1,
            bits_per_pixel     : bits_per_pixel,
            compression_type   : compression as u32,
            image_size         : i_size,
            pixels_per_meter_x : 0,
            pixels_per_meter_y : 0,
            colors_used        : 0,
            colors_important   : 0,

            // NOTE(erick): Copying gimp here.
            red_mask   : 0xff000000,
            green_mask : 0x00ff0000,
            blue_mask  : 0x0000ff00,
            alpha_mask : 0x000000ff,

            is_top_down : false,

        }
    }

    fn from_data(data: &[u8]) -> BitmapInfoHeader {
        let mut data_walker = BytesWalker::new(data);

        let mut result = BitmapInfoHeader {
            info_header_size   : data_walker.next_u32(),
            image_width        : data_walker.next_i32(),
            image_height       : data_walker.next_i32(),
            n_planes           : data_walker.next_u16(),
            bits_per_pixel     : data_walker.next_u16(),
            compression_type   : data_walker.next_u32(),
            image_size         : data_walker.next_u32(),
            pixels_per_meter_x : data_walker.next_i32(),
            pixels_per_meter_y : data_walker.next_i32(),
            colors_used        : data_walker.next_u32(),
            colors_important   : data_walker.next_u32(),

            red_mask   : 0,
            green_mask : 0,
            blue_mask  : 0,
            alpha_mask : 0,

            is_top_down : false,
        };

        if result.image_height < 0 {
            result.is_top_down = true;
            result.image_height *= -1;
        }

        if result.info_header_size > 40 {
            // NOTE(erick): We have masks to read
            result.red_mask   = data_walker.next_u32();
            result.green_mask = data_walker.next_u32();
            result.blue_mask  = data_walker.next_u32();
            result.alpha_mask = data_walker.next_u32();
        }

        result
    }

    fn into_data(&self, data: &mut Vec<u8>) {
        push_u32(data, self.info_header_size);
        push_i32(data, self.image_width);
        push_i32(data, self.image_height);
        push_u16(data, self.n_planes);
        push_u16(data, self.bits_per_pixel);
        push_u32(data, self.compression_type);
        push_u32(data, self.image_size);
        push_i32(data, self.pixels_per_meter_x);
        push_i32(data, self.pixels_per_meter_y);
        push_u32(data, self.colors_used);
        push_u32(data, self.colors_important);

        if self.info_header_size > 40 {
            push_u32(data, self.red_mask);
            push_u32(data, self.green_mask);
            push_u32(data, self.blue_mask);
            push_u32(data, self.alpha_mask);
        }

    }
}

#[derive(Clone, Debug)]
pub struct BitmapPixel {
    pub blue  : u8,
    pub green : u8,
    pub red   : u8,
    pub alpha : u8,
}

impl Display for BitmapPixel {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "0x{:02x}{:02x}{:02x}{:02x}",
               self.red, self.green, self.blue, self.alpha)
    }
}

impl Copy for BitmapPixel {}

#[allow(dead_code)]
impl BitmapPixel {
    pub fn rgba(r: u8, g: u8, b: u8, a: u8) -> BitmapPixel {
        BitmapPixel {
            red   : r,
            green : g,
            blue  : b,
            alpha : a,
        }
    }

    pub fn rgb(r: u8, g: u8, b: u8) -> BitmapPixel {
        BitmapPixel::rgba(r, g, b, 0xff)
    }

    pub fn rgba_u32(color: u32) -> BitmapPixel {
        BitmapPixel::rgba((color >> 24) as u8,
                          (color >> 16) as u8,
                          (color >>  8) as u8,
                          (color)       as u8)
    }

    pub fn rgb_u32(color: u32) -> BitmapPixel {
        BitmapPixel::rgba_u32(color | 0xff)
    }

    pub fn same_color_as(&self, other: &BitmapPixel) -> bool {
        self.red == other.red &&
            self.green == other.green &&
            self.blue == other.blue
    }

    pub fn black() -> BitmapPixel {
        BitmapPixel::rgba(0x00, 0x00, 0x00, 0xff)
    }
    pub fn white() -> BitmapPixel {
        BitmapPixel::rgba(0xff, 0xff, 0xff, 0xff)
    }
    pub fn red() -> BitmapPixel {
        BitmapPixel::rgba(0xff, 0x00, 0x00, 0xff)
    }
    pub fn green() -> BitmapPixel {
        BitmapPixel::rgba(0x00, 0xff, 0x00, 0xff)
    }
    pub fn blue() -> BitmapPixel {
        BitmapPixel::rgba(0x00, 0x00, 0xff, 0xff)
    }
    pub fn transparent() -> BitmapPixel {
        BitmapPixel::rgba(0xff, 0xff, 0xff, 0x00)
    }
}

fn mask_offset_and_shifted(mut mask: u32) -> (u8, u32) {
    if mask == 0 {
        // Early-out
        return (0, 0);
    }

    // Shift right until we find the first one.
    let mut offset = 0;
    while mask & 0x01 == 0 {
        mask = mask >> 1;
        offset += 1;
    }

    (offset, mask)
}

// TODO(erick): Floating-point is slow. We have enough
// precision to do it using fixed-point math.
fn map_zero_based(value: &mut u8, from: u32, to: u32) {
    // Don't do useless work and don't divide by zero.
    if from == to || from == 0 { return; }

    let t = (*value as f32) / from as f32;
    *value = (to as f32 * t) as u8;
}

fn interpret_image_data(data: &[u8],
                        info_header: &BitmapInfoHeader) -> Vec<BitmapPixel> {
    let bits_per_pixel   = info_header.bits_per_pixel;
    let compression_type = info_header.compression_type;

    let mut data_walker = BytesWalker::new(data);
    //NOTE(erick): This is only true while we don't handle compression
    let mut result = Vec::with_capacity(data.len());

    if compression_type == CompressionType::BitFields as u32 {
        let red_mask   = info_header.red_mask;
        let green_mask = info_header.green_mask;
        let blue_mask  = info_header.blue_mask;
        let alpha_mask = info_header.alpha_mask;

        let (red_offset,   red_shifted)   = mask_offset_and_shifted(red_mask);
        let (green_offset, green_shifted) = mask_offset_and_shifted(green_mask);
        let (blue_offset,  blue_shifted)  = mask_offset_and_shifted(blue_mask);
        let (alpha_offset, alpha_shifted) = mask_offset_and_shifted(alpha_mask);

        if bits_per_pixel == 32 {
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
        } else if bits_per_pixel == 16 {
            let mut pixels_read = 0;
            while data_walker.has_data() {
                if pixels_read == info_header.image_width {
                    // NOTE(erick): We have to align rows to
                    // 4 bytes values.
                    pixels_read = 0;
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
                // HACK: Since your map function is incorrect, we hardcode
                // the alpha mapping.
                // if pixel.alpha == 1 {
                //     pixel.alpha = 0xff;
                // }

                if alpha_mask == 0x00 {
                    // NOTE(erick): We are in XRGB mode.
                    pixel.alpha = 0xff;
                }

                result.push(pixel);
                pixels_read += 1;
            }
        } else {
            panic!("BitField is only compatible with 16 and 32 bit. Got: {}",
                   bits_per_pixel);
        }
    } else if compression_type == CompressionType::Uncompressed as u32 {
        if bits_per_pixel == 24 {
            let mut column_index = 0;
            while data_walker.has_data() {
                if column_index == info_header.image_width {
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
        } else if bits_per_pixel == 32 {
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
        } else {
            panic!("We don't support {} bits images yet", bits_per_pixel);
        }
    } else {
        panic!("We don't support {:?} compression yet",
               CompressionType::from(compression_type));
    }

    result
}

fn pixels_into_data(pixels: &Vec<BitmapPixel>, data: &mut Vec<u8>,
                    bitmap_info: &BitmapInfoHeader) {
    // TODO(erick): Support 16-bit BitFields images.
    if bitmap_info.compression_type == CompressionType::BitFields as u32 {
        assert_eq!(32, bitmap_info.bits_per_pixel);

        let (red_bit_offset, _)   = mask_offset_and_shifted(bitmap_info.red_mask);
        let (green_bit_offset, _) = mask_offset_and_shifted(bitmap_info.green_mask);
        let (blue_bit_offset, _)  = mask_offset_and_shifted(bitmap_info.blue_mask);
        let (alpha_bit_offset, _) = mask_offset_and_shifted(bitmap_info.alpha_mask);

        for pixel in pixels {
            let pixel_value : u32 = (pixel.red as u32)   << red_bit_offset   |
            (pixel.green as u32) << green_bit_offset |
            (pixel.blue  as u32) << blue_bit_offset  |
            (pixel.alpha as u32) << alpha_bit_offset & bitmap_info.alpha_mask;
            // NOTE(erick): We and with alpha_mask so we can support ARGB and
            // XRGB at the same time.

            push_u32(data, pixel_value);
        }
    } else if bitmap_info.compression_type == CompressionType::Uncompressed as u32 {
        if bitmap_info.bits_per_pixel == 32 {
            for pixel in pixels {
                data.push(pixel.blue);
                data.push(pixel.green);
                data.push(pixel.red);
                data.push(0x00); // Padding
            }
        } else if bitmap_info.bits_per_pixel == 24 {
            let mut pixel_iter = pixels.into_iter();

            let bytes_per_row = bitmap_info.image_width * 3;
            let n_padding_bytes = (4 - (bytes_per_row % 4)) % 4;

            for _ in 0 .. bitmap_info.image_height {
                for _ in 0 .. bitmap_info.image_width {
                    let pixel = pixel_iter.next().unwrap();

                    data.push(pixel.blue);
                    data.push(pixel.green);
                    data.push(pixel.red);
                }

                for _ in 0 .. n_padding_bytes {
                    data.push(0x00);
                }
            }
        } else {
            panic!("pixels_to_data: Unsupported bpp: {}",
                   bitmap_info.bits_per_pixel);
        }
    } else {
        panic!("pixels_to_data: Unsupported compression: {:?}",
               bitmap_info.compression_type);
    }
}

pub  struct Bitmap {
    pub file_header : BitmapFileHeader,
    pub info_header : BitmapInfoHeader,
    pub image_data  : Vec<BitmapPixel>,
}

impl Bitmap {

    pub fn lazy_new(width: i32, height: i32) -> Bitmap {
        let n_pixels = (width * height) as u32;
        // NOTE(erick): Only true when using 32-bit pixels and no compression.
        let image_data_size = n_pixels * 4;

        let p_offset = FILE_HEADER_SIZE + INFO_HEADER_SIZE;
        let file_size = p_offset + image_data_size;

        let file_header = BitmapFileHeader::new(file_size, p_offset);
        let info_header = BitmapInfoHeader::new(INFO_HEADER_SIZE,
                                               width, height,
                                               image_data_size,
                                               32, // TODO: Support other formats
                                               CompressionType::BitFields);

        Bitmap {
            file_header : file_header,
            info_header : info_header,
            image_data  : Vec::new(),
        }
    }

    pub fn new(width: i32, height: i32) -> Bitmap {
        let n_pixels = (width * height) as u32;

        let mut result = Bitmap::lazy_new(width, height);
        result.image_data = vec![BitmapPixel::transparent(); n_pixels as usize];

        result
    }

    pub fn from_file(file: &mut File) -> BitmapResult<Bitmap> {
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;

        Bitmap::from_data(data)
    }

    pub fn from_data(data: Vec<u8>) -> BitmapResult<Bitmap> {
        let data_slice = data.as_slice();
        let f_header =
            BitmapFileHeader::from_data(&data_slice[0..FILE_HEADER_SIZE as usize]);
        if !f_header.validate() {
            return Err(BitmapError::InvalidBitmap);
        }

        // NOTE(erick): We only support the basic header so far.
        let info_header =
            BitmapInfoHeader::from_data(&data_slice[FILE_HEADER_SIZE as usize ..]);

        println!("{}", f_header);
        println!("{}", info_header);

        let i_header_size = info_header.info_header_size;
        if i_header_size != 40 && i_header_size != 56 {
            return Err(BitmapError::
                       UnsupportedInfoHeaderSize(i_header_size))
        }

        let compression_type = CompressionType::from(info_header.compression_type);
        match compression_type {
            CompressionType::Uncompressed | CompressionType::BitFields => {},
            _ => {
                return Err(BitmapError::
                           UnsupportedCompressionType(compression_type))
            },
        }

        if info_header.n_planes != 1 {
            return Err(BitmapError::
                       UnsupportedNumberOfPlanes(info_header.n_planes));
        }

        let mut image_size_in_bytes = info_header.image_size as usize;

        // NOTE(erick): 'image_size' may be zero when the image is uncompressed
        // so we calculate the size in this case.
        if info_header.compression_type == CompressionType::Uncompressed as u32 {
            // FIXME(erick): If bits_per_pixel is 4 this value is set to zero.
            image_size_in_bytes = (info_header.bits_per_pixel / 8) as usize *
                info_header.image_width as usize * info_header.image_height as usize;

            if info_header.bits_per_pixel == 24 {
                // NOTE(erick): We need to add the padding bytes.
                let padding_per_row = (4 - (3 * info_header.image_width % 4)) % 4;
                image_size_in_bytes += (info_header.image_height * padding_per_row) as usize;
            }
        }

        let image_data_slice  = &data_slice[f_header.pixel_array_offset as usize ..
                                            f_header.pixel_array_offset as usize +
                                            image_size_in_bytes];

        // TODO(erick): Decompressed the image!!!!
        let image_data = interpret_image_data(&image_data_slice,
                                              &info_header);

        let result = Bitmap {
            file_header : f_header,
            info_header : info_header,
            image_data  : image_data,
        };

        Ok(result)
    }

    pub fn into_data(&self) -> Vec<u8> {
        let mut result = Vec::new();

        self.file_header.into_data(&mut result);
        self.info_header.into_data(&mut result);

        let data_size = result.len();

        // Padding the data
        for _ in data_size .. self.file_header.pixel_array_offset as usize {
            result.push(0x00);
        }

        pixels_into_data(&self.image_data, &mut result, &self.info_header);

        result
    }

    pub fn into_file(&self, file: &mut File) -> BitmapResult<()> {
        let data = self.into_data();

        // NOTE(erick): For some reason io::Error was not been
        // converted to BitmapIOError(io_error).
        if let Err(io_error) = file.write_all(data.as_slice()) {
            return Err(BitmapError::BitmapIOError(io_error));
        }
        Ok(())
    }

    pub fn convert_to_32_bitfield_compression(&mut self) {
        let width  = self.info_header.image_width;
        let height = self.info_header.image_height;

        let n_pixels = (width * height) as u32;
        let image_data_size = n_pixels * 4;

        let p_offset = FILE_HEADER_SIZE + INFO_HEADER_SIZE;
        let file_size = p_offset + image_data_size;

        if self.info_header.is_top_down {
            self.mirror_vertically();
        }

        // NOTE(erick): It's easier to create new header than to
        // try to modify the existing ones.
        let file_header = BitmapFileHeader::new(file_size, p_offset);
        let info_header = BitmapInfoHeader::new(INFO_HEADER_SIZE,
                                               width, height,
                                               image_data_size,
                                               32,
                                               CompressionType::BitFields);

        self.file_header = file_header;
        self.info_header = info_header;
    }

    pub fn color_to_alpha(&mut self, color: BitmapPixel) {
        for pixel in &mut self.image_data {
            if pixel.same_color_as(&color) {
                pixel.alpha = 0x00;
            }
        }
    }

    // NOTE(erick): We can probably have a lazy version of this function
    // if we use the 'is_to_down' flag every time we read from the the
    // image_data.
    pub fn mirror_vertically(&mut self) {
        let data_slice = self.image_data.as_mut_slice();
        let stride = self.info_header.image_width as usize;

        for row_index in 0 .. (self.info_header.image_height / 2) as usize {
            let mirrored_row_index = self.info_header.image_height as usize
                - row_index  - 1;

            let top_data_index = row_index * stride ;
            let bottom_data_index = mirrored_row_index * stride;

            let top_region    = top_data_index .. top_data_index + stride;
            let bottom_region = bottom_data_index .. bottom_data_index + stride;

            swap_slice_regions(data_slice, top_region, bottom_region);
        }
    }

    pub fn mirror_horizontally(&mut self) {
        let data_slice = self.image_data.as_mut_slice();
        let stride = self.info_header.image_width as usize;

        for row_index in 0 .. (self.info_header.image_height) as usize {
            let data_index = row_index * stride;

            let row_slice = &mut data_slice[data_index .. data_index + stride];
            mirror_slice(row_slice);
        }
    }

    pub fn crop_to_rect(&self, x0: u32, y0: u32,
                        width: u32, height: u32) -> BitmapResult<Bitmap> {
        if x0 >= self.info_header.image_width as u32 ||
            y0 >= self.info_header.image_height as u32 {
                return Err(BitmapError::InvalidOperation)
            }

        if x0 + width > self.info_header.image_width as u32 ||
            y0 + height > self.info_header.image_height as u32 {
                return Err(BitmapError::InvalidOperation)
            }


        let mut result = Bitmap::lazy_new(width as i32, height as i32);
        result.copy_rect_from(self, x0, y0, width, height);

        Ok(result)
    }

    pub fn merge_horizontally(image0: &Bitmap, image1: &Bitmap) -> Bitmap {
        let result_width = image0.info_header.image_width +
            image1.info_header.image_width;
        let result_height = max(image0.info_header.image_height,
                                image1.info_header.image_height);

        let mut result = Bitmap::new(result_width, result_height);
        result.replace_rect_with_rect_from(image0,
                                           0, 0,
                                           0, 0,
                                           image0.info_header.image_width as u32,
                                           image0.info_header.image_height as u32);
        result.replace_rect_with_rect_from(image1,
                                           0, 0,
                                           image0.info_header.image_width as u32, 0,
                                           image1.info_header.image_width as u32,
                                           image1.info_header.image_height as u32);

        result
    }

    pub fn merge_vertically(image0: &Bitmap, image1: &Bitmap) -> Bitmap {
        let result_height = image0.info_header.image_height +
            image1.info_header.image_height;
        let result_width = max(image0.info_header.image_width,
                               image1.info_header.image_width);

        let mut result = Bitmap::new(result_width, result_height);
        result.replace_rect_with_rect_from(image0,
                                           0, 0,
                                           0, 0,
                                           image0.info_header.image_width as u32,
                                           image0.info_header.image_height as u32);
        result.replace_rect_with_rect_from(image1,
                                           0, 0,
                                           0, image0.info_header.image_height as u32,
                                           image1.info_header.image_width as u32,
                                           image1.info_header.image_height as u32);

        result
    }

    //
    // Private stuff.
    //
    fn replace_rect_with_rect_from(&mut self, other: &Bitmap,
                                   src_x0 : u32, src_y0 : u32,
                                   dest_x0: u32, dest_y0: u32,
                                   width: u32, height: u32) {

        let src_stride  = other.info_header.image_width as usize;
        let dest_stride = self.info_header.image_width  as usize;

        let mut current_dest_y = dest_y0 as usize;
        for current_src_y in src_y0 as usize .. (src_y0 + height) as usize {
            let mut current_dest_x = dest_x0 as usize;

            for current_src_x in src_x0 as usize .. (src_x0 + width) as usize {
                let src_data_index  = current_src_y * src_stride + current_src_x;
                let dest_data_index = current_dest_y * dest_stride + current_dest_x;

                let data = other.image_data[src_data_index];
                self.image_data[dest_data_index] = data;

                current_dest_x += 1;
            }

            current_dest_y += 1;
        }
    }

    fn copy_rect_from(&mut self, other: &Bitmap,
                      x0: u32, y0: u32, width: u32, height: u32) {
        assert_eq!(0, self.image_data.len());

        let stride = other.info_header.image_width as usize;

        for row_index in y0 as usize .. (y0 + height) as usize {
            for column_index in x0 as usize .. (x0 + width) as usize {
                let data_index = row_index * stride + column_index;
                let data = other.image_data[data_index];

                self.image_data.push(data);
            }
        }
    }
}

fn swap_slice_regions<T>(slice: &mut [T],
                         mut r0: Range<usize>,
                         mut r1: Range<usize>) where T: Copy {
    loop {
        let i0 = r0.next();
        let i1 = r1.next();

        if i0.is_none() || i1.is_none() {
            break;
        }

        let i0 = i0.unwrap();
        let i1 = i1.unwrap();

        let tmp = slice[i0];
        slice[i0] = slice[i1];
        slice[i1] = tmp;
    }
}

fn mirror_slice<T>(slice: &mut [T]) where T: Copy {
    for index_left in 0 .. slice.len() / 2 {
        let index_right = slice.len() - index_left - 1;

        let tmp = slice[index_left];
        slice[index_left] = slice[index_right];
        slice[index_right] = tmp;
    }
}

struct BytesWalker<'a> {
    data          : &'a [u8],
    current_index : usize,
}

impl<'a> BytesWalker<'a> {
    fn new(d: &[u8]) -> BytesWalker {
        BytesWalker {
            data          : d,
            current_index : 0,
        }
    }

    fn has_data(&self) -> bool {
        self.current_index < self.data.len()
    }

    fn next_u8(&mut self) -> u8 {
        let result = self.data[self.current_index];
        self.current_index += 1;

        result
    }

    // NOTE(erick): It would be nice to use generics to
    // generate this functions, but I don't know of
    // a way to get the size of a type at compile time.
    // WARNING(erick): Theses functions only work
    // because the bitmap format uses little-endianness
    // and we are running on an little-endian machine.
    // Sooner or later this will have to be fixed.
    fn next_u16(&mut self) -> u16 {
        let mut bytes = [0; 2];
        bytes.clone_from_slice(&self.data[self.current_index .. self.current_index + 2]);
        self.current_index += 2;

        unsafe { transmute(bytes) }
    }

    fn next_u32(&mut self) -> u32 {
        let mut bytes = [0; 4];
        bytes.clone_from_slice(&self.data[self.current_index .. self.current_index + 4]);
        self.current_index += 4;

        unsafe { transmute(bytes) }
    }

    fn next_i32(&mut self) -> i32 {
        let mut bytes = [0; 4];
        bytes.clone_from_slice(&self.data[self.current_index .. self.current_index + 4]);
        self.current_index += 4;

        unsafe { transmute(bytes) }
    }

    fn align_with_u32(&mut self) {
        let rem = self.current_index % 4;

        if rem != 0 {
            self.current_index += 4 - rem;
        }
    }
}

fn push_u32(v: &mut Vec<u8>, value: u32) {
    // NOTE(erick): Little-endian.
    v.push((value >>  0) as u8);
    v.push((value >>  8) as u8);
    v.push((value >> 16) as u8);
    v.push((value >> 24) as u8);
}
fn push_i32(v: &mut Vec<u8>, value: i32) {
    // NOTE(erick): Little-endian.
    v.push((value >>  0) as u8);
    v.push((value >>  8) as u8);
    v.push((value >> 16) as u8);
    v.push((value >> 24) as u8);
}
fn push_u16(v: &mut Vec<u8>, value: u16) {
    // NOTE(erick): Little-endian.
    v.push((value >>  0) as u8);
    v.push((value >>  8) as u8);
}
