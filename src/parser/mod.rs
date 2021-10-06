mod csv_parser;
mod dre_parser;
mod mus_parser;
mod txt_parser;

pub use csv_parser::parse_csv_input;
pub use dre_parser::parse_dreadnaut_input;
pub use mus_parser::{parse_mus, BinInput, BinParseError, BinParseResult};
pub use txt_parser::parse_txt_input;

pub type Input<'a> = &'a str;
pub type ParseError<'a> = nom::error::VerboseError<Input<'a>>;
pub type ParseResult<'a, O> = nom::IResult<Input<'a>, O, ParseError<'a>>;
