extern crate svg;

use std::f32;

use svg::Document;
use svg::node::Node;
use svg::node::element::{Rectangle, Group, Definitions, ClipPath, Circle};

macro_rules! to_px {
    ($num:expr) => { format!("{}px", $num) }
}
macro_rules! s {
    ($x:expr) => { $x.to_string() }
}

type FlagColors = Vec<(String, f32)>;

fn create_flag_graphic(width:f32, height:f32, raw_colours:FlagColors) -> Group {
    // Remove all the non-positive height bands(just in case).
    let mut colours:FlagColors = raw_colours.clone();
    colours.retain(|&(_, band_height)| band_height > 0.);

    let mut total_band_height = 0.;
    for &(_, band_height) in colours.iter() {
        total_band_height += band_height;
    }

    let band_height_ratio = height / total_band_height;

    let mut layer = Group::new();
    let mut curr_y = 0.;
    for &(ref colour, band_heigh) in colours.iter() {
        let disp_rect_height = band_heigh * band_height_ratio;
        let mut rect_height = disp_rect_height + 1.;
        if rect_height + curr_y > height {
            rect_height = disp_rect_height;
        }
        let flag_colour = colour.to_string();
        let mut colour_rect = Rectangle::new();
        colour_rect.assign("fill", flag_colour);
        colour_rect.assign("width", to_px!(width));
        colour_rect.assign("height", to_px!(rect_height));
        colour_rect.assign("x", to_px!(0));
        colour_rect.assign("y", to_px!(curr_y));
        layer.append(colour_rect);
        curr_y += disp_rect_height;
    }
    layer
}


fn main() {
    const WIDTH:usize = 100;

    let flag_colours = vec![
        (s!("#750787"), 1.),
        (s!("#004DFF"), 1.),
        (s!("#008026"), 1.),
        (s!("#FFED00"), 1.),
        (s!("#FF8C00"), 1.),
        (s!("#E40303"), 1.)
    ];

    let mut clip = ClipPath::new();
    clip.assign("id", "clip");
    let mut circle = Rectangle::new();
    circle.assign("rx", to_px!(WIDTH as f32 / 3.5));
    circle.assign("ry", to_px!(WIDTH as f32 / 3.5));
    circle.assign("width", to_px!(WIDTH));
    circle.assign("height", to_px!(WIDTH));
    circle.assign("x", to_px!(0));
    circle.assign("y", to_px!(0));
    clip.append(circle);

    let mut pride_flag = create_flag_graphic(WIDTH as f32, WIDTH as f32, flag_colours);
    pride_flag.assign("clip-path", "url(#clip)");

    let mut document = Document::new()
                                .set("viewBox", (0, 0, WIDTH, WIDTH));
    let mut defs = Definitions::new();
    defs.append(clip);
    document = document.add(defs).add(pride_flag);

    svg::save("image.svg", &document).unwrap();
}
