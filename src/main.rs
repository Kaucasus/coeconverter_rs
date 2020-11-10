extern crate image;

use clap::arg_enum;
use image::GenericImageView;
use std::path::PathBuf;
use structopt::StructOpt;

use std::fs::File;
use std::io::prelude::*;

arg_enum! {
    #[derive(Debug)]
    enum Mode {
        HDMI,
        HDMIA,
        HDMIa,
        VGA,
        VGAA,
        VGAa,
        Gray,
        GrayA,
        Graya,
        Bit,
        Bita
    }
}

/// Convert an image to a .coe file for Vivado/Xilinx Block Ram.
#[derive(StructOpt)]
struct Cli {
    /// The path to the image to read
    #[structopt(parse(from_os_str))]
    image: PathBuf,
    /// The video mode of the output
    #[structopt(possible_values = &Mode::variants(), case_insensitive = true)]
    mode: Mode,
    /// The transparancy treshold if alpha is 1 bit
    #[structopt(short, long, default_value = "127")]
    threshold: u8,
    /// Output file, same basename as image if not present
    #[structopt(short, long, parse(from_os_str))]
    output: Option<PathBuf>,
}

fn main() {
    let args = Cli::from_args();
    let mut path = PathBuf::new();
    match args.output {
        Some(x) => path.push(x),
        None => path.push(&args.image),
    };

    path.set_extension("coe");

    let img = image::open(&args.image).expect("Failed to open image.");

    let coe_info = build_info(img, args.mode, args.threshold);

    let display = path.display();

    let mut file = match File::create(&path) {
        Err(why) => panic!("couldn't create {}: {}", display, why),
        Ok(file) => file,
    };

    match file.write_all(build_file(coe_info).as_bytes()) {
        Err(why) => panic!("couldn't write to {}: {}", display, why),
        Ok(_) => println!("successfully wrote to {}", display),
    }
}

struct CoeInfo {
    mode: Mode, //mode info
    image_width: u32,
    image_height: u32,

    mem_width: u32,    //memory can go from 1 bit to 4608 bits
    mem_depth: u64, //depth is at least 2, maximum is platform dependent, can't be more then the maximum x and y of the image can handle, which are both u32
    adress_width: u32, //adress is always directly related to width and height but who truly cares?

    memory_init_radix: u8, //radix is either 2,10 or 16 (which I could use an enum for but really?)
    memory_init_vector: String, //the actual memory vector
}

fn build_info(image: image::DynamicImage, mode: Mode, threshold: u8) -> CoeInfo {
    let image_width = image.width();
    let image_height = image.height();

    let mode_width = match mode {
        Mode::HDMI => 24,
        Mode::HDMIa => 25,
        Mode::HDMIA => 32,
        Mode::VGA => 8,
        Mode::VGAa => 9,
        Mode::VGAA => 16,
        Mode::Gray => 8,
        Mode::Graya => 9,
        Mode::GrayA => 16,
        Mode::Bit => 1,
        Mode::Bita => 2,
    };

    CoeInfo {
        memory_init_vector: build_memory_vector(image, &mode, threshold),
        image_width,
        image_height,
        mode,
        mem_depth: (image_width as u64) * (image_height as u64),
        mem_width: mode_width,
        adress_width: (((image_width as u64) * (image_height as u64)) as f64)
            .sqrt()
            .ceil() as u32,
        memory_init_radix: if mode_width % 8 == 0 { 16 } else { 2 },
    }
}

fn build_file(coe: CoeInfo) -> String {
    format!(
        ";This is a .COE file in a {mode} image generated via the coeconvertor_rs tool.\n\
        ;The image has a width: {image_width} and height: {image_height}\n\
        ;The memory has a width={mem_width}, and depth={mem_depth}\n\
        ;(So that means addra is {address_width} if minimum area 8kx2 is used) \n\n\
        memory_initialization_radix={radix}\n\
        memory_initialization_vector={vector}",
        mode = coe.mode,
        image_width = coe.image_width,
        image_height = coe.image_height,
        mem_width = coe.mem_width,
        mem_depth = coe.mem_depth,
        address_width = coe.adress_width,
        radix = coe.memory_init_radix,
        vector = coe.memory_init_vector
    )
}

fn build_memory_vector(image: image::DynamicImage, mode: &Mode, threshold: u8) -> String {
    let mut memory_init_vector = String::new();

    for pixel in image.pixels() {
        memory_init_vector += &pixel2str(pixel.2, mode, threshold);
        //There's probably a better way to do this. We have an iterator after all...
        if pixel.0 == image.width() - 1 {
            if pixel.1 == image.height() - 1 {
                memory_init_vector += ";\n";
            } else {
                memory_init_vector += ",\n";
            }
        } else {
            memory_init_vector += " ";
        }
    }
    memory_init_vector
}

fn pixel2str(pixel: image::Rgba<u8>, mode: &Mode, threshold: u8) -> String {
    match mode {
        Mode::HDMI => format!("{:02x}{:02x}{:02x}", pixel[0], pixel[1], pixel[2]),
        Mode::HDMIa => format!(
            "{:08b}{:08b}{:08b}{:01b}",
            pixel[0],
            pixel[1],
            pixel[2],
            if pixel[3] >= threshold { 1 } else { 0 }
        ),
        Mode::HDMIA => format!(
            "{:02x}{:02x}{:02x}{:02x}",
            pixel[0], pixel[1], pixel[2], pixel[3]
        ),
        Mode::VGA => format!("{:08x}", to_vga((pixel[0], pixel[1], pixel[2]))),
        Mode::VGAa => format!(
            "{:024b}{:01b}",
            to_vga((pixel[0], pixel[1], pixel[2])),
            if pixel[3] >= threshold { 1 } else { 0 }
        ),
        Mode::VGAA => format!(
            "{:02x}{:02x}",
            to_vga((pixel[0], pixel[1], pixel[2])),
            pixel[3]
        ),
        Mode::Gray => format!("{:08x}", to_gray((pixel[0], pixel[1], pixel[2]))),
        Mode::Graya => format!(
            "{:024b}{:01b}",
            to_gray((pixel[0], pixel[1], pixel[2])),
            if pixel[3] >= threshold { 1 } else { 0 }
        ),
        Mode::GrayA => format!(
            "{:02x}{:02x}",
            to_gray((pixel[0], pixel[1], pixel[2])),
            pixel[3]
        ),
        Mode::Bit => format!(
            "{:01b}",
            if to_bit((pixel[0], pixel[1], pixel[2])) {
                1
            } else {
                0
            }
        ),
        Mode::Bita => format!(
            "{:01b}{:01b}",
            if to_bit((pixel[0], pixel[1], pixel[2])) {
                1
            } else {
                0
            },
            if pixel[3] >= threshold { 1 } else { 0 }
        ),
    }
}

fn to_vga(rgb: (u8, u8, u8)) -> u8 {
    ((rgb.0 & 0b1110_0000) + ((rgb.1 & 0b1110_0000) >> 3) + ((rgb.2 & 0b1100_0000) >> 6))
}

fn to_gray(rgb: (u8, u8, u8)) -> u8 {
    //Not entirely accurate gray but otherwise really cost intensive
    ((0.2126 / 255.0) * rgb.0 as f32
        + (0.7152 / 255.0) * rgb.1 as f32
        + (0.0722 / 255.0) * rgb.2 as f32) as u8
}

fn to_bit(rgb: (u8, u8, u8)) -> bool {
    //This gives an awful result since it's very simple
    ((rgb.0 >= 128) && (rgb.1 >= 128))
        || ((rgb.1 >= 128) && (rgb.2 >= 128))
        || ((rgb.0 >= 128) && (rgb.2 >= 128))
}
