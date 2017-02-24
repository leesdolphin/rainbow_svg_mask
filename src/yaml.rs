
use error::Result;

use std::{f64, str};
use std::collections::HashMap;
use std::str::FromStr;

use yaml_rust::Yaml;
use yaml_rust::yaml::Hash;
use svgdom::FromStream;
use svgdom;

type ResultOptional<T> = Result<Option<T>>;

#[derive(Debug, Copy, Clone)]
pub struct BorderInformation {
    color: Option<svgdom::types::Color>,
    linecap: Option<svgdom::ValueId>,
    linejoin: Option<svgdom::ValueId>,
    width: Option<f64>,
}

impl BorderInformation {
    pub fn color(&self) -> svgdom::types::Color {
        self.color.unwrap_or(svgdom::types::Color::new(0xee, 0xee, 0xee))
    }
    pub fn linecap(&self) -> svgdom::ValueId{
        self.linecap.unwrap_or(svgdom::ValueId::Round)
    }
    pub fn linejoin(&self) -> svgdom::ValueId{
        self.linejoin.unwrap_or(svgdom::ValueId::Stroke)
    }
    pub fn width(&self) -> f64{
        self.width.unwrap_or(5.)
    }
}

#[derive(Debug, Clone)]
pub struct ClipInformation {
    pub name: String,
    pub filename: String,
    pub border: BorderInformation,
}

pub type ClipMap = HashMap<String, ClipInformation>;
pub type FlagMap = HashMap<String, FlagColors>;
pub type FlagColors = Vec<(svgdom::types::Color, f64)>;

macro_rules! get_yaml {
    ($hash:expr, $key:expr) => {
        $hash.get(&Yaml::String($key.to_string()))
    };
    // ($hash:expr, $key:expr => $t:ident) => {
    //     match get_yaml!($hash, $key) {
    //         Some(value) => match value.$t() {
    //             Some(value) => Some(value),
    //             None => bail!("The value did not convert correctly $type")
    //         },
    //         None => None,
    //     }
    // };
}

fn get_yaml_str<'a>(hash:&'a Hash, key:&str) -> ResultOptional<&'a str> {
    match get_yaml!(hash, key) {
        Some(yaml_value) => match yaml_value.as_str() {
            Some(str_value) => Ok(Some(str_value)),
            None => bail!("The value did not convert correctly $type")
        },
        None => Ok(None),
    }
}

macro_rules! yaml_to_float {
    ($yaml:expr) => {{
        match $yaml {
            &Yaml::String(ref float) | &Yaml::Real(ref float) => match f64::from_str(&float) {
                Ok(f) => f,
                Err(_) => bail!("Invalid floating point number for height definition.")
            },
            &Yaml::Integer(i) => i.clone() as f64,
            &Yaml::Boolean(b) => if b { 1. } else { 0. },
            _ => bail!("Invalid value for height definition.")
        }
    }};
}

macro_rules! parse_svg_value {
    ($hash:expr, $key:expr => $($valid_val:ident),+) => {
        parse_svg_value_from_hash($hash, $key, &vec!($(svgdom::ValueId::$valid_val),+))
    };
}
fn parse_svg_value_from_hash(hash:&Hash, key:&str, valid:&Vec<svgdom::ValueId>) -> ResultOptional<svgdom::ValueId> {
    match try!(get_yaml_str(hash, key)) {
        Some(value_str) => match svgdom::ValueId::from_name(value_str) {
            Some(vid) => {
                if valid.contains(&vid) {
                    Ok(Some(vid))
                } else {
                    bail!("Value is invalid for this attribute.")
                }
            },
            None => bail!("Value is not a valid SVG attribute's value.")
        },
        None => Ok(None)
    }
}

fn parse_clippings_yaml(hash:&Hash) -> Result<ClipMap> {
    let mut clips:ClipMap = HashMap::new();
    for (clip_name, clip_dfn) in hash.iter() {
        let name = try!(clip_name.as_str().ok_or("Clip name must be a string"));
        let mut color = None;
        let mut linecap = None;
        let mut linejoin = None;
        let mut width = None;
        let filename:Option<&str>;
        match clip_dfn {
            &Yaml::String(ref fname) => { filename = Some(fname) },
            &Yaml::Hash(ref clip_hash) => {
                filename = try!(get_yaml_str(&clip_hash, "src"));
                color = match try!(get_yaml_str(&clip_hash, "color")) {
                    Some(color_str) => Some(try!(yaml_to_color(color_str))),
                    None => None
                };
                // println!("{:?}", match get_yaml!(clip_hash, "linecap" => into_string) {
                //     Some(value_str) => Some(0),
                //     None => None
                // });
                linecap = try!(parse_svg_value!(clip_hash, "linecap" => Butt, Round, Square, Inherit));
                linejoin = try!(parse_svg_value!(clip_hash, "linecap" => Miter, Round, Bevel, Inherit));
                width = match get_yaml!(clip_hash, "width") {
                    Some(val) => Some(yaml_to_float!(val)),
                    None => None,
                };
                println!("{:?}, {:?}, {:?}, {:?}, {:?}", filename, color, linecap, linejoin, width);
            },
            _ => bail!("Invalid Clip Definition {}", name)
        }
        if let Some(fname) = filename {
            let border_info = BorderInformation {
                color: color,
                linecap: linecap,
                linejoin: linejoin,
                width: width,
            };
            let clip_info = ClipInformation {
                name: name.to_string(),
                filename: fname.to_string(),
                border: border_info
            };
            clips.insert(name.to_string(), clip_info);
        } else {
            bail!("Filename not given for clip definition {}", name);
        }
    }
    Ok(clips)
}

fn yaml_to_color(color_name:&str) -> Result<svgdom::types::Color> {
    match svgdom::types::Color::from_data(&color_name.as_bytes()[..]) {
        Ok(color) => Ok(color),
        Err(_) => {
            // Assume it was an RBG color, and it is simply missing the '#'.
            let color_rgb = format!("#{}", color_name);
            match svgdom::types::Color::from_data(&color_rgb.into_bytes()[..]) {
                Ok(color) => Ok(color),
                Err(_) => bail!(format!("Invalid color definition."))
            }
        }
    }
}

fn parse_flag_colors_from_yaml<'b>(yaml:&'b Yaml) -> Result<FlagColors> {
    let mut colors:FlagColors = FlagColors::new();
    match yaml {
        &Yaml::Array(ref arr) => {
            for c_item in arr {
                match c_item {
                    &Yaml::String(ref color_name) => colors.push((try!(yaml_to_color(color_name)), 1.)),
                    _ => bail!("Invalid item in flag definition array.")
                }
            }
        },
        &Yaml::Hash(ref color_hash) => {
            for (color_name, band) in color_hash.iter() {
                let color = match *color_name {
                    Yaml::String(ref color_name) => try!(yaml_to_color(&color_name)),
                    _ => bail!("Invalid item in flag definition array.")
                };
                let band:f64 = yaml_to_float!(band);
                colors.push((color, band));
            }
        },
        _ => bail!("Invalid type for flag definition.")
    }
    Ok(colors)
}

fn parse_flags_from_yaml<'a, 'b>(hash:&'b Hash) -> Result<FlagMap> {
    let mut flags = FlagMap::new();
    for (flag_name, color_dfn) in hash.iter() {
        if let &Yaml::String(ref yaml_name) = flag_name {
            if yaml_name.starts_with('_') {
                continue;
            }
            let flag_colors = try!(parse_flag_colors_from_yaml(&color_dfn));
            flags.insert(yaml_name.clone(), flag_colors);
        };
    }
    Ok(flags)
}

pub fn parse_flag_yaml(doc:&Yaml) -> Result<(FlagMap, ClipMap)> {
    match doc {
        &Yaml::Hash(ref hash) => {
            let clip_map = match hash.get(&Yaml::String("_clips".to_string())) {
                Some(&Yaml::Hash(ref clippings)) => try!(parse_clippings_yaml(clippings)),
                Some(_) => bail!("Key `_clips` is defined incorrectly, should be a hash."),
                _ => bail!("Key `_clips` is not defined.")
            };
            let flags_map = try!(parse_flags_from_yaml(hash));
            Ok((flags_map, clip_map))
        },
        _ => return try!(None.ok_or("Document is not a Hash(mapping)"))
    }
}
