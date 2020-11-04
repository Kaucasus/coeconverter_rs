extern crate image;

use clap::arg_enum;
use image::GenericImageView;
use std::path::PathBuf;
use structopt::StructOpt;

arg_enum! {
    #[derive(Debug)]
    enum ArgsMode {
        HDMI,
        VGA,
        Gray,
        Bit
    }
}

/// Convert an image to a .coe file for Vivado/Xilinx Block Ram.
#[derive(StructOpt)]
struct Cli {
    /// The path to the image to read
    #[structopt(parse(from_os_str))]
    image: PathBuf,
    /// The video mode of the output
    #[structopt(possible_values = &ArgsMode::variants(), case_insensitive = true)]
    mode: ArgsMode,
    /// The alpha mode (-a is 1 bit, -aa is 8 bits)
    #[structopt(short, long, parse(from_occurrences))]
    alpha: u8,
    /// Output file, same basename as image if not present
    #[structopt(parse(from_os_str))]
    output: Option<PathBuf>,
    //add transparency here!
}

fn main() {
    let args = Cli::from_args();
    println!("Hello, world!");
    let img = image::open(&args.image).expect("Failed to open image.");

    println!(
        "Image dimensions are: {:?}, and mode is {:?}",
        img.dimensions(),
        img.color()
    );

    let mut coe_info = build_info(img.width(), img.height(), args.mode, args.alpha);

    let mut memory_init_vector = String::new();

    for pixel in img.pixels() {
        memory_init_vector += &pixel2str(pixel.2);
        //There's probably a better way to do this. We have an iterator after all...
        if pixel.0 == img.width() - 1 {
            if pixel.1 == img.height() - 1 {
                memory_init_vector += ";\n";
            } else {
                memory_init_vector += ",\n";
            }
        } else {
            memory_init_vector += " ";
        }
    }

    println!("{:};", memory_init_vector);
}

struct Mode {
    colour: ArgsMode,
    alpha: u8,
}

struct CoeInfo {
    mode: Mode, //mode info
    image_width: u32,
    image_height: u32,

    mem_width: u32,    //memory can go from 1 bit to 4608 bits
    mem_depth: u64, //depth is at least 2, maximum is platform dependent, can't be more then the maximum x and y of the image can handle, which are both u32
    adress_width: u32, //adress is always directly related to width and height but who truly cares?

    memory_init_radix: u8, //radix is either 2,10 or 16 (which I could use an enum for but really?)
    memory_init_vector: String, //The string to be actually used
}

fn build_info(image_width: u32, image_height: u32, colour: ArgsMode, alpha: u8) -> CoeInfo {
    let alpha_width = match alpha {
        0 => 0,
        1 => 1,
        _ => 8,
    };
    let colour_width = match colour {
        ArgsMode::HDMI => 24,
        ArgsMode::VGA => 8,
        ArgsMode::Gray => 8,
        ArgsMode::Bit => 1,
    };

    CoeInfo {
        image_width,
        image_height,
        mode: Mode { colour, alpha },
        mem_depth: (image_width as u64) * (image_height as u64),
        mem_width: alpha_width + colour_width,
        adress_width: (((image_width as u64) * (image_height as u64)) as f64)
            .sqrt()
            .ceil() as u32,
        memory_init_radix: if (alpha_width + colour_width) % 8 == 0 {
            16
        } else {
            2
        },
        memory_init_vector: String::new(),
    }
}

fn pixel2str(pixel: image::Rgba<u8>) -> String {
    format!("{:08b}{:08b}{:08b}", pixel[0], pixel[1], pixel[2])
}
