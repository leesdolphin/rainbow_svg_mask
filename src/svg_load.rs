use std::fs::File;
use std::io::Read;

use error::{Error, SvgDomError};


use svgdom::{Document, ParseOptions};
use svgcleaner::cleaner::clean_doc;

// const DOM_FILTERS:Vec<fn(&Document)> = vec!();


pub fn load_svg_into_document(filename: &str, opt: Option<ParseOptions>) -> Result<Document, Error> {
    let mut file = try!(File::open(filename));
    let length = try!(file.metadata()).len() as usize;

    let mut data = Vec::with_capacity(length + 1);
    try!(file.read_to_end(&mut data));

    let document = if let Some(parse_opts) = opt {
        svg_try!(Document::from_data_with_opt(&data[..], &parse_opts))
    } else {
        svg_try!(Document::from_data(&data[..]))
    };

    // TODO: Clean the SVG.

    Ok(document)
}
