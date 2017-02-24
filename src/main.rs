extern crate rainbow_svg_mask;
extern crate svgdom;
#[macro_use]
extern crate error_chain;
extern crate svgcleaner;
extern crate yaml_rust;
extern crate svgparser;

#[macro_use] pub mod error;
pub mod svg_load;
pub mod yaml;

use error::Result;
use yaml::{FlagColors, ClipInformation, parse_flag_yaml};

use std::{f64, str};
use std::fmt;
use std::fs::File;
use std::io::{Write, Read};
use std::ops::Deref;

use yaml_rust::YamlLoader;
use svgdom::{WriteBuffer, WriteOptions, AttributeType};

macro_rules! to_px {
    ($num:expr) => { format!("{}px", $num) }
}
macro_rules! s {
    ($x:expr) => { $x.to_string() }
}
macro_rules! set_attrs {
    ($node:expr, $($name:ident, $value:expr);+) => {
        $(
            $node.set_attribute(svgdom::AttributeId::$name, $value);
        )+
    }
}

#[derive(Debug, Copy, Clone)]
struct ViewBox {
    x:f64,
    y:f64,
    width:f64,
    height:f64
}

impl ViewBox {

    #[inline]
    pub fn new(x:f64, y:f64, width:f64, height:f64) -> Self {
        ViewBox {
            x: x,
            y: y,
            width: width,
            height: height,
        }
    }

    #[inline]
    pub fn from_vec(vec:&Vec<f64>) -> Option<Self> {
        if vec.len() != 4 {
            None
        } else {
            Some(ViewBox::new(
                vec[0].into(),
                vec[1].into(),
                vec[2].into(),
                vec[3].into(),
            ))
        }
    }

    #[inline]
    pub fn pad(&self, pad_amount:f64) -> Self {
        ViewBox {
            x: self.x - pad_amount,
            y: self.y - pad_amount,
            width: self.width + pad_amount * 2.,
            height: self.height + pad_amount * 2.,
        }
    }
}

impl fmt::Display for ViewBox {

    #[inline]
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "{} {} {} {}", self.x, self.y, self.width, self.height)
    }
}

fn clone_node(doc:&svgdom::Document, orig_node:&svgdom::Node) -> Option<svgdom::Node> {
    let node = match orig_node.node_type() {
        svgdom::NodeType::Element => match orig_node.tag_name() {
            Some(tag_name) => match tag_name.deref() {
                &svgdom::Name::Id(tag_id) => doc.create_element(tag_id),
                &svgdom::Name::Name(ref tag_str) => doc.create_element(tag_str.as_ref()),
            },
            None => return None
        },
        svgdom::NodeType::Root => unreachable!(),
        _ => match orig_node.text() {
            Some(text) => doc.create_node(orig_node.node_type(), text.deref()),
            None => doc.create_node(orig_node.node_type(), "")
        }
    };
    for orig_attr in orig_node.attributes().iter() {
        node.set_attribute_object(orig_attr.clone());
    }
    for orig_child in orig_node.children() {
        if let Some(new_child) = clone_node(doc, &orig_child){
            node.append(&new_child);
        }
    }
    Some(node)
}


fn create_flag_graphic(document:&svgdom::Document, view_box:&ViewBox, raw_colors:&FlagColors) -> svgdom::Node {
    // Remove all the non-positive height bands(just in case).
    let mut colors:FlagColors = raw_colors.clone();
    colors.retain(|&(_, band_height)| band_height > 0.);

    let mut total_band_height = 0.;
    for &(_, band_height) in colors.iter() {
        total_band_height += band_height;
    }

    let band_height_ratio = view_box.height / total_band_height;

    let layer =  document.create_element(svgdom::ElementId::G);
    let mut curr_y = 0.;
    for &(ref color, band_heigh) in colors.iter() {
        let disp_rect_height = band_heigh * band_height_ratio;
        let mut rect_height = disp_rect_height + 1.;
        if rect_height + curr_y > view_box.height {
            rect_height = disp_rect_height;
        }
        let flag_color = color.to_string();
        let color_rect =  document.create_element(svgdom::ElementId::Rect);
        color_rect.set_attribute(svgdom::AttributeId::Fill, flag_color);
        color_rect.set_attribute(svgdom::AttributeId::Width, to_px!(view_box.width));
        color_rect.set_attribute(svgdom::AttributeId::Height, to_px!(rect_height));
        color_rect.set_attribute(svgdom::AttributeId::X, to_px!(view_box.x));
        color_rect.set_attribute(svgdom::AttributeId::Y, to_px!(view_box.y + curr_y));
        layer.append(&color_rect);
        curr_y += disp_rect_height;
    }
    layer
}

fn create_rainbow_flag<U: Iterator<Item=svgdom::Node>>(
        document:&svgdom::Document, view_box:&ViewBox, flag_colors:&FlagColors,
        clip:U, clip_id:&str) -> Result<()> {
    let pride_flag = create_flag_graphic(document, view_box, &flag_colors);
    pride_flag.set_attribute(svgdom::AttributeId::ClipPath, format!("url(#{})", clip_id));

    let clip_path = document.create_element(svgdom::ElementId::ClipPath);
    clip_path.set_id(clip_id);
    for node in clip {
        clip_path.append(&node);
    }
    let defs = document.create_element(svgdom::ElementId::Defs);
    defs.append(&clip_path);
    // Use unwrap as it is a code invariant instead of a possible user error.
    let doc_root = document.svg_element().unwrap();
    doc_root.append(&defs);
    doc_root.append(&pride_flag);
    Ok(())
}

fn create_pride_flag<'a>(clip_info: &ClipInformation, colors: &FlagColors, clip_svg:&svgdom::Node, view_box:&ViewBox) -> Result<svgdom::Document> {
    let new_doc = svgdom::Document::new();
    let svg = new_doc.append(&new_doc.create_element(svgdom::ElementId::Svg));
    let clip_svg = match clone_node(&new_doc, &clip_svg) {
        Some(node) => node,
        None => bail!("Could not clone root node")
    };
    if clip_info.border.width() <= 0. {
        set_attrs!(svg,
            ViewBox, view_box.to_string()
        );
    } else {
        set_attrs!(svg,
            ViewBox, view_box.pad(clip_info.border.width()).to_string()
        );

        let border_g = new_doc.create_element(svgdom::ElementId::G);
        set_attrs!(border_g,
            Stroke, clip_info.border.color();
            StrokeLinecap, clip_info.border.linecap();
            StrokeLinejoin, clip_info.border.linejoin();
            StrokeWidth, clip_info.border.width() * 2.
        );
        for orig_node in svgdom::Children::new(clip_svg.first_child()) {
            if let Some(node) = clone_node(&new_doc, &orig_node) {
                node.attributes_mut().retain(
                    |attr| attr.is_svg() && !attr.is_fill() && !attr.is_animation_event()
                            && !attr.is_graphical_event() && !attr.is_document_event());
                border_g.append(&node);
            }
        }
        svg.append(&border_g);
    }
    try!(create_rainbow_flag(&new_doc, &view_box, &colors, clip_svg.children(), "queer_clip_001"));

    Ok(new_doc)
}

fn find_view_box(clip_svg:&svgdom::Node) -> Result<ViewBox> {
    match clip_svg.attributes().get(svgdom::AttributeId::ViewBox) {
        Some(view_box) => match view_box.value {
            svgdom::AttributeValue::NumberList(ref vb_vec) =>{
                match ViewBox::from_vec(&vb_vec) {
                    Some(view_box) => return Ok(view_box),
                    _ => bail!("SVG root has an unsuported view box format.")
                }
            },
            _ => bail!("SVG root has an unsuported view box format.")
        },
        None => bail!("SVG root must have a viewBox defined")
    };
}

fn create_flags(yaml_file:&str, output_folder:&str) -> Result<()>{
    let mut file = try!(File::open(yaml_file));
    let mut data = String::new();
    try!(file.read_to_string(&mut data));

    let docs = try!(YamlLoader::load_from_str(data.as_str()));
    let doc = docs.first().unwrap();
    let (flag_map, clip_map) = try!(parse_flag_yaml(doc));
    let mut buf:Vec<u8> = Vec::with_capacity(1024);

    for (clip_name, clip_info) in clip_map.iter() {
        let clip_doc = try!(svg_load::load_svg_into_document(clip_info.filename.as_ref(), None));
        let clip_svg = try!(clip_doc.svg_element().ok_or("Invalid SVG"));
        let view_box = try!(find_view_box(&clip_svg));
        for (flag_name, flag_colors) in flag_map.iter() {
            let flag = try!(create_pride_flag(&clip_info, &flag_colors, &clip_svg, &view_box));

            buf.resize(0, 0); // Nuke all old data; but keep capacity; we'll likely need it.
            let mut opts = WriteOptions::default();
            opts.trim_hex_colors = true;
            opts.indent = 2;
            opts.paths.coordinates_precision = 5;
            flag.write_buf_opt(&opts, &mut buf);
            let out_filename = format!("{}/{}_{}.svg", output_folder, flag_name, clip_name);
            println!("{:?}", out_filename);
            try!(try!(File::create(out_filename)).write_all(&buf[..]));
        }
    }

    Ok(())
}

fn main() {
    match create_flags("flags.yml", "target") {
        Err(err) => {
            println!("Error: {:?}", err);
            panic!(err);
        },
        Ok(_) => println!("Done.")
    };
}
