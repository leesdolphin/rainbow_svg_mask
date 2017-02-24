use std::error;
use std::io;
use std::fmt;
use std::convert;
use std::num;

use yaml_rust;
use svgdom;


#[derive(Debug)]
pub struct SvgDomError {
    inner:svgdom::Error
}
impl SvgDomError {

    #[inline]
    pub fn new(error:svgdom::Error) -> Self {
        SvgDomError {
            inner: error
        }
    }
}
impl error::Error for SvgDomError {
    fn description(&self) -> &str {
        "SVGDom Error"
    }

    fn cause(&self) -> Option<&error::Error> { None }
}
impl fmt::Display for SvgDomError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl convert::From<svgdom::Error> for SvgDomError {
    fn from(err:svgdom::Error) -> Self {
        SvgDomError::new(err)
    }
}

#[macro_export]
macro_rules! svg_try {
    ($result:expr) => {
        try!(match $result {
            Ok(result) => Ok(result),
            Err(err) => {
                Err(SvgDomError::new(err))
            }
        })
    }
}

error_chain! {
    foreign_links {
        Io(io::Error);
        SvgDom(SvgDomError);
        ParseError(num::ParseFloatError);
        YamlError(yaml_rust::ScanError);
    }
}
