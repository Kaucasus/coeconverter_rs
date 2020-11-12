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
        VGA,
        Gray,
        Bit
    }
}

arg_enum! {
    #[derive(Debug)]
    enum Style {
        Spaces,
        Commas
    }
}

arg_enum! {
    #[derive(Debug)]
    enum Representation {
    Hex,
    Dec,
    Bin
}}

/// Convert an image to a .coe file for Vivado/Xilinx Block Ram.
#[derive(StructOpt)]
struct Cli {
    /// The path to the image to read
    #[structopt(parse(from_os_str))]
    image: PathBuf,
    /// The video mode of the output
    #[structopt(possible_values = &Mode::variants(), case_insensitive = true)]
    mode: Mode,
    /// The alpha amount based on occurence. Single occurence 1 bit, multiple 8 bit
    #[structopt(short, long, parse(from_occurrences))]
    alpha: u8,
    /// The transparancy treshold if alpha is 1 bit
    #[structopt(short, long, default_value = "127")]
    threshold: u8,
    /// Output file, same basename as image if not present
    #[structopt(short, long, parse(from_os_str))]
    output: Option<PathBuf>,
    /// The representation of the vector. Bin is required for least amount of bitwidth!
    #[structopt(short, long, possible_values = &Representation::variants(), case_insensitive = true, default_value="Bin")]
    representation: Representation,
    /// The style of the coe file
    #[structopt(short, long, possible_values = &Style::variants(), case_insensitive = true, default_value="Spaces")]
    style: Style,
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

    let coe_info = build_info(
        img,
        args.mode,
        args.alpha,
        args.threshold,
        args.style,
        args.representation,
    );

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

    mem_width: u32,     //memory can go from 1 bit to 4608 bits
    mem_depth: u64, //depth is at least 2, maximum is platform dependent, can't be more then the maximum x and y of the image can handle, which are both u32
    address_width: u32, //address is always directly related to width and height but who truly cares?

    memory_init_radix: u8,      //radix is either 2,10 or 16
    memory_init_vector: String, //the actual memory vector
}

fn build_info(
    image: image::DynamicImage,
    mode: Mode,
    alpha: u8,
    threshold: u8,
    style: Style,
    rep: Representation,
) -> CoeInfo {
    let image_width = image.width();
    let image_height = image.height();

    let mode_width: u8 = match mode {
        Mode::HDMI => 24,
        Mode::VGA => 8,
        Mode::Gray => 8,
        Mode::Bit => 1,
    };

    let alpha_width: u8 = match alpha {
        0 => 0,
        1 => 1,
        _ => 8,
    };

    let mem_width: u32 = (mode_width + alpha_width) as u32;
    // TODO: Make it so memory width and integers and everything is correctly initialised

    CoeInfo {
        memory_init_vector: build_memory_vector(image, &mode, alpha, threshold, &rep, style),
        image_width,
        image_height,
        mode,
        mem_depth: (image_width as u64) * (image_height as u64),
        mem_width,
        address_width: (((image_width as u64) * (image_height as u64)) as f64)
            .sqrt()
            .ceil() as u32,
        memory_init_radix: match rep {
            Representation::Bin => 2,
            Representation::Dec => 10,
            Representation::Hex => 16,
        },
    }
}

fn build_file(coe: CoeInfo) -> String {
    format!(
        ";This is a .COE file in a {mode} image generated via the coeconvertor_rs tool.\n\
        ;The image has a width: {image_width} and height: {image_height}\n\
        ;The memory has a width={mem_width}, and depth={mem_depth}\n\
        ;(So that means addra is {address_width} if minimum area 8kx2 is used) \n\
        memory_initialization_radix={radix}\n\
        memory_initialization_vector={vector}",
        mode = coe.mode,
        image_width = coe.image_width,
        image_height = coe.image_height,
        mem_width = coe.mem_width,
        mem_depth = coe.mem_depth,
        address_width = coe.address_width,
        radix = coe.memory_init_radix,
        vector = coe.memory_init_vector
    )
}

fn build_memory_vector(
    image: image::DynamicImage,
    mode: &Mode,
    alpha: u8,
    threshold: u8,
    rep: &Representation,
    style: Style,
) -> String {
    let mut memory_init_vector = String::new();

    for pixel in image.pixels() {
        memory_init_vector += &pixel2str(pixel.2, mode, alpha, threshold, rep);
        //There's probably a better way to do this. We have an iterator after all...
        if pixel.0 == image.width() - 1 {
            if pixel.1 == image.height() - 1 {
                memory_init_vector += ";\n";
            } else {
                match style {
                    Style::Spaces => memory_init_vector += ",\n",
                    Style::Commas => memory_init_vector += ",",
                };
            }
        } else {
            match style {
                Style::Spaces => memory_init_vector += " ",
                Style::Commas => memory_init_vector += ",",
            };
        }
    }
    memory_init_vector
}

fn pixel_bit_twiddle(pixel: image::Rgba<u8>, mode: &Mode, alpha: u8, threshold: u8) -> u32 {
    let colour = match mode {
        Mode::HDMI => {
            ((pixel[0] as u32) << 16) + ((pixel[1] as u32) << 8) + ((pixel[2] as u32) << 0)
        }
        Mode::VGA => to_vga(pixel[0], pixel[1], pixel[2]) as u32,
        Mode::Gray => to_gray(pixel[0], pixel[1], pixel[2]) as u32,
        Mode::Bit => to_bit(pixel[0], pixel[1], pixel[2]) as u32,
    };

    match alpha {
        0 => colour,
        1 => (colour << 1) + ((pixel[3] >= threshold) as u32),
        _ => (colour << 8) + (pixel[3] as u32),
    }
}

fn pixel2str(
    pixel: image::Rgba<u8>,
    mode: &Mode,
    alpha: u8,
    threshold: u8,
    rep: &Representation,
) -> String {
    match rep {
        Representation::Bin => format!("{:b}", pixel_bit_twiddle(pixel, mode, alpha, threshold)),
        Representation::Dec => format!("{}", pixel_bit_twiddle(pixel, mode, alpha, threshold)),
        Representation::Hex => format!("{:X}", pixel_bit_twiddle(pixel, mode, alpha, threshold)),
    }
}

fn to_vga(red: u8, green: u8, blue: u8) -> u8 {
    (red & 0b1110_0000) + ((green & 0b1110_0000) >> 3) + ((blue & 0b1100_0000) >> 6)
}

fn to_gray(red: u8, green: u8, blue: u8) -> u8 {
    //Not entirely accurate gray but otherwise really cost intensive
    ((0.2126 / 255.0) * red as f32
        + (0.7152 / 255.0) * green as f32
        + (0.0722 / 255.0) * blue as f32) as u8
}

fn to_bit(red: u8, green: u8, blue: u8) -> bool {
    //This gives an awful result since it's very simple
    ((red >= 128) && (green >= 128))
        || ((red >= 128) && (blue >= 128))
        || ((green >= 128) && (blue >= 128))
}
