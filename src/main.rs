use std::path::Path;

use image::{GenericImage, GenericImageView, ImageBuffer, RgbaImage};
use qrcode_generator::QrCodeEcc;
use std::time::Instant;

static WHITE: image::Rgba<u8> = image::Rgba([255, 255, 255, 255]);
static BLACK: image::Rgba<u8> = image::Rgba([0, 0, 0, 255]);

struct QrImage {
    img: RgbaImage,
    num_modules: u32,
    module_size: u32,
    qr_offset: u32,
}

impl QrImage {
    fn new(num_modules: u32, image_size: u32) -> QrImage {
        let qr_len = num_modules + 8;
        let module_size = image_size / qr_len as u32;
        let qr_offset = module_size * 4 + (image_size - qr_len * module_size) / 2;
        dbg!(qr_len, module_size, qr_offset);

        let mut img = ImageBuffer::from_fn(image_size, image_size, |x, y| WHITE);

        QrImage {
            img: img,
            num_modules: num_modules,
            module_size: module_size,
            qr_offset: qr_offset,
        }
    }

    fn add_finder_pattern(&mut self, x_offset: u32, y_offset: u32) {
        // a finder pattern is a seven by seven box included in all corners but the bottom right
        self.draw_box(x_offset, y_offset, 7 * self.module_size, BLACK);

        self.draw_box(
            x_offset + self.module_size,
            y_offset + self.module_size,
            5 * self.module_size,
            WHITE,
        );

        self.draw_box(
            x_offset + self.module_size * 2,
            y_offset + self.module_size * 2,
            3 * self.module_size,
            BLACK,
        );
    }

    fn draw_box(&mut self, x_offset: u32, y_offset: u32, size: u32, colour: image::Rgba<u8>) {
        for x in x_offset..x_offset + size {
            for y in y_offset..y_offset + size {
                self.draw_pixel(x + self.qr_offset, y + self.qr_offset, colour);
            }
        }
    }

    fn draw_circle(
        &mut self,
        x_offset: u32,
        y_offset: u32,
        diameter: u32,
        colour: image::Rgba<u8>,
    ) {
        let radius = diameter as f32 / 2.0;
        let center_x = x_offset as f32 + radius;
        let center_y = y_offset as f32 + radius;

        for x in x_offset..x_offset + diameter {
            for y in y_offset..y_offset + diameter {
                let distance = (center_x - x as f32).powf(2.0) + (center_y - y as f32).powf(2.0);

                //dbg!(x, y, distance.sqrt());
                if distance < radius * radius {
                    self.draw_pixel(x + self.qr_offset, y + self.qr_offset, colour);
                }
            }
        }
    }

    fn draw_pixel(&mut self, x: u32, y: u32, colour: image::Rgba<u8>) {
        *self.img.get_pixel_mut(x, y) = colour;
    }

    fn write_to_file(&self) {
        self.img.save("src/static/test.png").unwrap();
    }
}

fn is_in_finder_pattern(x: u32, y: u32, num_modules: u32) -> bool {
    (x < 7 && y < 7) || (x < 7 && y >= num_modules - 7) || (x >= num_modules - 7 && y < 7)
}

fn draw_image(matrix: &Vec<Vec<bool>>, final_image_size: u32) {
    let mut img = QrImage::new(matrix.len() as u32, final_image_size * 4);

    img.add_finder_pattern(0, 0);
    img.add_finder_pattern((img.num_modules - 7) * img.module_size, 0);
    img.add_finder_pattern(0, (img.num_modules - 7) * img.module_size);

    for (row_index, row) in matrix.iter().enumerate() {
        for (col_index, cell) in row.iter().enumerate() {
            if *cell && !is_in_finder_pattern(col_index as u32, row_index as u32, img.num_modules) {
                img.draw_box(
                    row_index as u32 * img.module_size,
                    col_index as u32 * img.module_size,
                    img.module_size,
                    BLACK,
                );

                // panic!("panic");
            }
        }
    }

    let mut central_section = img.num_modules / 3;

    if central_section % 2 == 0 {
        central_section += 1;
    }

    img.draw_box(
        (img.num_modules - central_section) * img.module_size / 2,
        (img.num_modules - central_section) * img.module_size / 2,
        central_section * img.module_size,
        WHITE,
    );

    let mut overlay = image::open(&Path::new("./src/central_images/test_red.png"))
        .ok()
        .expect("Opening overlay failed");

    let central_section_offset = (img.num_modules - central_section) * img.module_size / 2;
    let central_section_size = ((central_section * img.module_size) as f32 * 0.9) as u32;

    let mut width_multiplier = 1.0;
    let mut height_multiplier = 1.0;

    if overlay.height() > overlay.width() {
        width_multiplier = overlay.height() as f64 / overlay.width() as f64
    } else {
        height_multiplier = overlay.width() as f64 / overlay.height() as f64
    }

    let resized_overlay = image::imageops::resize(
        &mut overlay,
        (central_section_size as f64 * width_multiplier) as u32,
        (central_section_size as f64 * height_multiplier) as u32,
        image::imageops::FilterType::Lanczos3,
    );

    dbg!(resized_overlay.width(), resized_overlay.height());

    image::imageops::overlay(
        &mut img.img,
        &resized_overlay,
        central_section_offset as u32
            + img.qr_offset
            + ((central_section * img.module_size) as f32 * 0.05) as u32
            + ((1.0 - width_multiplier) * central_section_size as f64 * 0.5) as u32,
        central_section_offset as u32
            + img.qr_offset
            + ((central_section * img.module_size) as f32 * 0.05) as u32
            + ((1.0 - height_multiplier) * central_section_size as f64 * 0.5) as u32,
    );

    img.img = image::imageops::resize(
        &mut img.img,
        final_image_size / 4,
        final_image_size / 4,
        image::imageops::FilterType::Lanczos3,
    );

    img.write_to_file();
}

fn main() {
    //let result: Vec<Vec<bool>> = qrcode_generator::to_matrix("Hell", QrCodeEcc::High).unwrap();
    let result: Vec<Vec<bool>> = qrcode_generator::to_matrix("Hello", QrCodeEcc::High).unwrap();

    let blocks = " ▀▄█".chars().collect::<Vec<char>>();
    for rows in result.chunks(2) {
        match rows.len() {
            1 => {
                for x in &rows[0] {
                    print!("{}", blocks[*x as usize]);
                }
            }
            2 => {
                for (top, bottom) in rows[0].iter().zip(rows[1].iter()) {
                    let mut offset = 0;

                    if *top {
                        offset += 1
                    };
                    if *bottom {
                        offset += 2
                    };

                    print!("{}", blocks[offset]);
                }
            }
            _ => {
                panic!("unexpected number of rows in chunk")
            }
        }

        println!("");
    }

    let now = Instant::now();
    draw_image(&result, 2048);

    let elapsed = now.elapsed();
    println!("Elapsed: {:.2?}", elapsed);
}
