extern crate image;

use image::GenericImageView;

fn main() {
    println!("Hello, world!");
    let img = image::open("tests/base_test_image.png").unwrap();

    println!(
        "Image dimensions are: {:?}, and mode is {:?}",
        img.dimensions(),
        img.color()
    );

    let mut memory_init_vector = String::new();

    for pixel in img.pixels() {
        memory_init_vector += &pixel_to_str(pixel.2);
        //There's probably an easier way to do this. We have an iterator after all...
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

#[derive(Debug)]
struct Mode {
    memwidth: i32,
    area: i32,
    alpha: u8,
}

fn pixel_to_str(pixel: image::Rgba<u8>) -> String {
    format!("{:08b}{:08b}{:08b}", pixel[0], pixel[1], pixel[2])
}
