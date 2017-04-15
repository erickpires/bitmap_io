use std::fmt::Display;
use std::fmt::Formatter;

use std::io::Write;
use std::io::Read;
use std::fs::File;

use std::intrinsics::transmute;

const BMP_MAGIC_NUMBER : u16 = 0x4d_42; // "MB": We are little-endian

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
}

#[allow(dead_code)]
// All hail multiple-cursors-mode
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
        // NOTE(erick): So far we only support 32-bit uncompressed images
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

        }
    }

    fn from_data(data: &[u8]) -> BitmapInfoHeader {
        let mut data_walker = BytesWalker::new(data);

        // TODO(erick): 'image_height' can be negative to indicate
        // top-down image. Handle this.
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
        };

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
        write!(f, "0x{:02x}{:02x}{:02x}{:02x}", self.red, self.green, self.blue, self.alpha)
    }
}

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
        BitmapPixel {
            red   : 0x00,
            green : 0x00,
            blue  : 0x00,
            alpha : 0xff,
        }
    }
    pub fn white() -> BitmapPixel {
        BitmapPixel {
            red   : 0xff,
            green : 0xff,
            blue  : 0xff,
            alpha : 0xff,
        }
    }
    pub fn red() -> BitmapPixel {
        BitmapPixel {
            red   : 0xff,
            green : 0x00,
            blue  : 0x00,
            alpha : 0xff,
        }
    }
    pub fn green() -> BitmapPixel {
        BitmapPixel {
            red   : 0x00,
            green : 0xff,
            blue  : 0x00,
            alpha : 0xff,
        }
    }
    pub fn blue() -> BitmapPixel {
        BitmapPixel {
            red   : 0x00,
            green : 0x00,
            blue  : 0xff,
            alpha : 0xff,
        }
    }
    pub fn transparent() -> BitmapPixel {
        BitmapPixel {
            red   : 0xff,
            green : 0xff,
            blue  : 0xff,
            alpha : 0x00,
        }
    }
}

fn bit_offset(mask: u32) -> u8 {
    match mask {
        0xff000000 => 24,
        0x00ff0000 => 16,
        0x0000ff00 => 8,
        0x000000ff => 0,
        _          => 0,
    }
}

fn interpret_image_data(data: &[u8],
                        info_header: &BitmapInfoHeader) -> Vec<BitmapPixel> {
    let bits_per_pixel   = info_header.bits_per_pixel;
    let compression_type = info_header.compression_type;

    let mut data_walker = BytesWalker::new(data);
    //NOTE(erick): This is only true while we don't handle compression
    let mut result = Vec::with_capacity(data.len());


    if compression_type == CompressionType::BitFields as u32 {
        assert_eq!(32, bits_per_pixel); // NOTE: This must be true

        let red_mask   = info_header.red_mask;
        let green_mask = info_header.green_mask;
        let blue_mask  = info_header.blue_mask;
        let alpha_mask = info_header.alpha_mask;

        let red_bit_offset   = bit_offset(red_mask);
        let green_bit_offset = bit_offset(green_mask);
        let blue_bit_offset  = bit_offset(blue_mask);
        let alpha_bit_offset = bit_offset(alpha_mask);

        while data_walker.has_data() {
            let pixel_value = data_walker.next_u32();

            let mut pixel = BitmapPixel {
                blue  : ((pixel_value & blue_mask)  >> blue_bit_offset) as u8,
                green : ((pixel_value & green_mask) >> green_bit_offset) as u8,
                red   : ((pixel_value & red_mask)   >> red_bit_offset) as u8,
                alpha : ((pixel_value & alpha_mask) >> alpha_bit_offset) as u8,
            };

            if alpha_mask == 0x00 {
                // NOTE(erick): We are in XRGB mode.
                pixel.alpha = 0xff;
            }

            result.push(pixel);
        }
    } else if bits_per_pixel == 24 {
        while data_walker.has_data() {
            let pixel = BitmapPixel {
                blue  : data_walker.next_u8(),
                green : data_walker.next_u8(),
                red   : data_walker.next_u8(),
                alpha : 0xff,
            };
            result.push(pixel);
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

    result
}

// TODO(erick): This function only supports 32-bit files with BitFields compression
fn pixels_into_data(pixels: &Vec<BitmapPixel>, data: &mut Vec<u8>,
                    bitmap_info: &BitmapInfoHeader) {
    let red_bit_offset   = bit_offset(bitmap_info.red_mask);
    let green_bit_offset = bit_offset(bitmap_info.green_mask);
    let blue_bit_offset  = bit_offset(bitmap_info.blue_mask);
    let alpha_bit_offset = bit_offset(bitmap_info.alpha_mask);

    for pixel in pixels {
        let pixel_value : u32 = (pixel.red as u32)   << red_bit_offset   |
                                (pixel.green as u32) << green_bit_offset |
                                (pixel.blue  as u32) << blue_bit_offset  |
                                (pixel.alpha as u32) << alpha_bit_offset;

        push_u32(data, pixel_value);
    }
}


pub  struct Bitmap {
    pub file_header : BitmapFileHeader,
    pub info_header : BitmapInfoHeader,
    pub image_data  : Vec<BitmapPixel>,
}

const FILE_HEADER_SIZE : u32 = 14;
const INFO_HEADER_SIZE : u32 = 56;  // Basic header with color masks

impl Bitmap {

    pub fn new(width: i32, height: i32) -> Bitmap {

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
        let i_data = vec![BitmapPixel::black(); n_pixels as usize];

        Bitmap {
            file_header : file_header,
            info_header : info_header,
            image_data  : i_data,
        }
    }

    // TODO(erick): Create a BitmapError and a bitmap_io::Result
    pub fn from_file(file: &mut File) -> std::io::Result<Bitmap> {
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        Ok(Bitmap::from_data(data))
    }

    pub fn from_data(data: Vec<u8>) -> Bitmap {
        let data_slice = data.as_slice();
        let f_header = BitmapFileHeader::from_data(&data_slice[0..14]);
        assert!(f_header.validate());

        // NOTE(erick): We only support the basic header so far.
        let info_header = BitmapInfoHeader::from_data(&data_slice[14..]);
        assert!(info_header.info_header_size == 40 ||
                info_header.info_header_size == 56);
        assert!(info_header.compression_type == CompressionType::Uncompressed as u32 ||
                info_header.compression_type == CompressionType::BitFields as u32);
        assert_eq!(1, info_header.n_planes);

        let mut image_size_in_bytes = info_header.image_size as usize;

        // NOTE(erick): 'image_size' may be zero when the image is uncompressed
        // so we calculate the size in this case.
        if info_header.compression_type == CompressionType::Uncompressed as u32 {
            // FIXME(erick): If bits_per_pixel is 4 this value is set to zero.
            image_size_in_bytes = (info_header.bits_per_pixel / 8) as usize *
                info_header.image_width as usize * info_header.image_height as usize;
        }

        let image_data_slice  = &data_slice[f_header.pixel_array_offset as usize
                              .. f_header.pixel_array_offset as usize
                              + image_size_in_bytes];
        // TODO(erick): Decompressed the image!!!!
        let image_data = interpret_image_data(image_data_slice,
                                              &info_header);

        Bitmap {
            file_header : f_header,
            info_header : info_header,
            image_data  : image_data,
        }
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

    pub fn into_file(&self, file: &mut File) -> std::io::Result<()> {
        let data = self.into_data();

        file.write_all(data.as_slice())
    }

    pub fn convert_to_bitfield_compression(&mut self) {
        let width  = self.info_header.image_width;
        let height = self.info_header.image_height;

        let n_pixels = (width * height) as u32;
        let image_data_size = n_pixels * 4;

        let p_offset = FILE_HEADER_SIZE + INFO_HEADER_SIZE;
        let file_size = p_offset + image_data_size;

        // NOTE(erick): It's easier to create new header than to
        // try to modify the existing ones.
        let file_header = BitmapFileHeader::new(file_size, p_offset);
        let info_header = BitmapInfoHeader::new(INFO_HEADER_SIZE,
                                               width, height,
                                               image_data_size,
                                               32, // TODO: Support other formats
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
