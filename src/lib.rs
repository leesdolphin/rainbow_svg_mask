#![cfg(doc=false)]

extern crate svgdom;
#[macro_use]
extern crate error_chain;
extern crate svgcleaner;
extern crate yaml_rust;
extern crate svgparser;

#[macro_use] pub mod error;
pub mod svg_load;
pub mod yaml;
