use std::path::Path;

use image::GenericImageView;
use qrcode_generator::QrCodeEcc;
use std::io::Cursor;
use std::io::SeekFrom;
use std::time::Instant;
use svg::node::element;
use svg::Document;

static WHITE: image::Rgba<u8> = image::Rgba([255, 255, 255, 255]);
static BLACK: image::Rgba<u8> = image::Rgba([0, 0, 0, 255]);
static BOX_SIZE: u32 = 100;

struct QrImage {
    // img: RgbaImage,
    img: Option<Document>,
    num_modules: u32,
    module_size: u32,
    qr_offset: u32,
}

impl QrImage {
    fn new(num_modules: u32, image_size: u32) -> QrImage {
        let qr_len = num_modules + 8;
        // let module_size = image_size / qr_len as u32;
        let module_size = 100;
        let qr_offset = module_size * 4;
        dbg!(qr_len, module_size, qr_offset);

        let background = element::Rectangle::new()
            .set("width", qr_len * BOX_SIZE)
            .set("height", qr_len * BOX_SIZE)
            .set("style", "fill:rgb(255,255,255);");

        let img = Document::new()
            .set("viewBox", (0, 0, qr_len * BOX_SIZE, qr_len * BOX_SIZE))
            //.set("shape-rendering", "crispEdges")
            .add(background);

        QrImage {
            img: Some(img),
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
        let colour_str = format!(
            "rgba({},{},{},{})",
            colour[0], colour[1], colour[2], colour[3]
        );
        let r = element::Rectangle::new()
            .set("x", self.qr_offset + x_offset)
            .set("y", self.qr_offset + y_offset)
            .set("width", size)
            .set("height", size)
            .set("fill", colour_str);

        let tmp = self.img.take().unwrap();
        self.img = Some(tmp.add(r));
    }

    fn draw_circle(
        &mut self,
        x_offset: u32,
        y_offset: u32,
        diameter: u32,
        colour: image::Rgba<u8>,
    ) {
        let colour_str = format!(
            "rgba({},{},{},{})",
            colour[0], colour[1], colour[2], colour[3]
        );
        let radius = diameter as f64 / 2.0;

        let circle = element::Circle::new()
            .set("cx", self.qr_offset as f64 + x_offset as f64 + radius)
            .set("cy", self.qr_offset as f64 + y_offset as f64 + radius)
            .set("r", radius)
            .set("fill", colour_str);

        let tmp = self.img.take().unwrap();
        self.img = Some(tmp.add(circle));
    }

    fn write_to_file(&mut self) {
        let tmp = self.img.take().unwrap();
        svg::save("image.svg", &tmp).unwrap();
        self.img = Some(tmp);
        // self.img.save("src/static/test.png").unwrap();
    }

    fn write_to_png(&mut self, size: u32) {
        let tmp = self.img.take().unwrap();
        let mut svg = Cursor::new(Vec::new());
        svg::write(&mut svg, &tmp);
        self.img = Some(tmp);

        let svg_data = svg.into_inner();

        let mut opt = usvg::Options::default();
        let rtree = usvg::Tree::from_data(&svg_data, &opt.to_ref()).unwrap();

        //let pixmap_size = rtree.svg_node().size.to_screen_size();
        //let mut pixmap = tiny_skia::Pixmap::new(pixmap_size.width(), pixmap_size.height()).unwrap();
        let mut pixmap = tiny_skia::Pixmap::new(size, size).unwrap();
        resvg::render(&rtree, usvg::FitTo::Height(size), pixmap.as_mut()).unwrap();
        pixmap.save_png("qr.png").unwrap();
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
                img.draw_circle(
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

    // img.draw_box(
    //     (img.num_modules - central_section) * img.module_size / 2,
    //     (img.num_modules - central_section) * img.module_size / 2,
    //     central_section * img.module_size,
    //     WHITE,
    // );

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

    // image::imageops::overlay(
    //     &mut img.img,
    //     &resized_overlay,
    //     central_section_offset as u32
    //         + img.qr_offset
    //         + ((central_section * img.module_size) as f32 * 0.05) as u32
    //         + ((1.0 - width_multiplier) * central_section_size as f64 * 0.5) as u32,
    //     central_section_offset as u32
    //         + img.qr_offset
    //         + ((central_section * img.module_size) as f32 * 0.05) as u32
    //         + ((1.0 - height_multiplier) * central_section_size as f64 * 0.5) as u32,
    // );

    // img.img = image::imageops::resize(
    //     &mut img.img,
    //     final_image_size / 4,
    //     final_image_size / 4,
    //     image::imageops::FilterType::Lanczos3,
    // );

    img.write_to_png(final_image_size);
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
    draw_image(&result, 200);

    let elapsed = now.elapsed();
    println!("Elapsed: {:.2?}", elapsed);
}
