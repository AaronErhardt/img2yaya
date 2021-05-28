use clap::{App, Arg};
use image::io::Reader as ImageReader;
use image::GrayImage;

use std::fs;
use std::io::{self, Write};

const DEFAULT_WIDTH: u32 = 16;

fn main() -> Result<(), image::ImageError> {
    // read arguments with clap
    let args = App::new("img2yaya")
        .version(clap::crate_version!())
        .about("Image to yayagram converter")
        .arg(
            Arg::with_name("width")
                .short("w")
                .long("width")
                .help("Sets width of yayagram board")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("height")
                .short("h")
                .long("height")
                .help("Sets height of yayagram board")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("threshold")
                .short("t")
                .long("threshold")
                .value_name("0-255")
                .help("Sets 8-bit grayscale threshold value for filling or not filling fields")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("invert")
                .short("i")
                .long("invert")
                .help("Inverts yayagram fields"),
        )
        .arg(
            Arg::with_name("INPUT")
                .help("Sets the input image file")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("OUTPUT")
                .help("Sets the output yayagram file. If not set output to stdout.")
                .index(2),
        )
        .get_matches();

    // get input file path, INPUT is required so unwrap is safe here
    let path = args.value_of("INPUT").unwrap();
    // read image and convert to 8-bit grayscale
    let img = ImageReader::open(path)?.decode()?.into_luma8();

    // check if output path was specified, otherwise write to stdout
    let mut writer: Box<dyn io::Write> = if let Some(path) = args.value_of("OUTPUT") {
        Box::new(fs::File::create(path).unwrap())
    } else {
        Box::new(io::stdout())
    };

    // calculate step sizes
    let img_width = img.width();
    let img_height = img.height();

    let (x_step_size, y_step_size) = match (args.value_of("width"), args.value_of("height")) {
        (None, None) => {
            // if nothing was specified, just use the default width as reference value
            let step_size = img_width / DEFAULT_WIDTH;
            (step_size, step_size)
        }
        (Some(width), None) => {
            let step_size = img_width / str::parse::<u32>(width).expect("Invalid value for width");
            (step_size, step_size)
        }
        (None, Some(height)) => {
            let step_size =
                img_height / str::parse::<u32>(height).expect("Invalid value for height");
            (step_size, step_size)
        }
        (Some(width), Some(height)) => (
            img_width / str::parse::<u32>(width).expect("Invalid value for width"),
            img_height / str::parse::<u32>(height).expect("Invalid value for height"),
        ),
    };

    let threshold: u8 = str::parse(args.value_of("threshold").unwrap_or("100")).unwrap_or(100);
    let invert = args.is_present("invert");

    // all set, so let's create a yayagram!
    create_yayagram(
        &mut writer,
        &img,
        x_step_size,
        y_step_size,
        threshold,
        invert,
    );

    Ok(())
}

/// Calculate the average value of (grayscale 8-bit) pixels in a rectangle starting from the start
/// coordinates and dimensions specified by width and height
#[inline]
fn local_average(img: &GrayImage, x_start: u32, y_start: u32, width: u32, height: u32) -> u8 {
    let mut sum: u32 = 0;
    for x in x_start..(x_start + width) {
        for y in y_start..(y_start + height) {
            sum += img.get_pixel(x, y).0[0] as u32;
        }
    }

    let avg = sum / (width * height);
    avg as u8
}

/// Create yayagram file from image and write to disk or stdout.
fn create_yayagram<W: io::Write>(
    writer: &mut W,
    img: &GrayImage,
    x_step_size: u32,
    y_step_size: u32,
    threshold: u8,
    invert: bool,
) {
    let width = img.width();
    let height = img.height();
    let x_steps = width / x_step_size;
    let y_steps = height / y_step_size;

    let mut buf_writer = io::BufWriter::new(writer);
    let mut line_buf: Vec<u8> = vec![0; x_steps as usize * 4];

    // write top line
    buf_writer.write_all(b"+").unwrap();
    buf_writer
        .write_all("-".repeat(x_steps as usize * 4).as_bytes())
        .unwrap();
    buf_writer.write_all(b"+\n").unwrap();

    for y in 0..y_steps {
        buf_writer.write_all(b"|").unwrap();
        for x in 0..x_steps {
            let byte: u8 = if (local_average(
                img,
                x * x_step_size,
                y * y_step_size,
                x_step_size,
                y_step_size,
            ) > threshold)
                != invert
            {
                b'1'
            } else {
                b' '
            };

            // write field 4 times to match yayagram format
            let field_start = x * 4;
            let field_end = field_start + 4;
            for i in field_start..field_end {
                line_buf[i as usize] = byte;
            }
        }
        // write line twice to match yayagram format
        buf_writer.write_all(&line_buf).unwrap();
        buf_writer.write_all(b"|\n|").unwrap();
        buf_writer.write_all(&line_buf).unwrap();
        buf_writer.write_all(b"|\n").unwrap();
    }

    // write bottom line
    buf_writer.write_all(b"+").unwrap();
    buf_writer
        .write_all("-".repeat(x_steps as usize * 4).as_bytes())
        .unwrap();
    buf_writer
        .write_all(b"+\n\n1: filled\nAutomatically generated using img2yaya\n")
        .unwrap();
}
