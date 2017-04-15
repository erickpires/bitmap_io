use std::fmt::Display;
use std::fmt::Formatter;

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
    fn new(h_size: u32, i_width: i32, i_height: i32, i_size: u32) -> BitmapInfoHeader {
        // NOTE(erick): So far we only support 32-bit uncompressed images
        BitmapInfoHeader {
            info_header_size   : h_size,
            image_width        : i_width,
            image_height       : i_height,
            n_planes           : 1,
            bits_per_pixel     : 32,
            compression_type   : CompressionType::Uncompressed as u32,
            image_size         : i_size,
            pixels_per_meter_x : 0,
            pixels_per_meter_y : 0,
            colors_used        : 0,
            colors_important   : 0,

            red_mask   : 0x0000ff00,
            green_mask : 0x00ff0000,
            blue_mask  : 0xff000000,
            alpha_mask : 0x000000ff,

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
}

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

fn interpret_image_data(data: &[u8],
                        bits_per_pixel: u16,
                        compression_type: u32) -> Vec<BitmapPixel> {
    let mut data_walker = BytesWalker::new(data);
    //NOTE(erick): This is only true while we don't handle compression
    let mut result = Vec::with_capacity(data.len());


    if compression_type == CompressionType::BitFields as u32 {
        assert_eq!(32, bits_per_pixel); // NOTE: This must be true

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

pub  struct Bitmap {
    pub file_header : BitmapFileHeader,
    pub info_header : BitmapInfoHeader,
    pub image_data  : Vec<BitmapPixel>,
}

impl Bitmap {
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
        println!("{}", info_header);
        assert_eq!(40, info_header.info_header_size);
        assert_eq!(1, info_header.n_planes);
        assert_eq!(CompressionType::Uncompressed as u32, info_header.compression_type);

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
                                              info_header.bits_per_pixel,
                                              info_header.compression_type);

        Bitmap {
            file_header : f_header,
            info_header : info_header,
            image_data  : image_data,
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
