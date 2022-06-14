#![deny(unsafe_code)]

mod macros;

pub mod assets;
pub mod assets_metadata {
    pub use super::assets::assets_metadata::*;
}
pub mod config;
pub mod controller;
mod decorations;
mod diff;
pub mod error;
pub mod input;
mod less;
pub mod line_range;
mod output;
#[cfg(feature = "paging")]
mod pager;
#[cfg(feature = "paging")]
pub mod paging;
mod preprocessor;
pub(crate) mod printer;
pub mod style;
pub(crate) mod syntax_mapping;
mod terminal;
mod vscreen;
pub(crate) mod wrapping;

pub use syntax_mapping::{MappingTarget, SyntaxMapping};
pub use wrapping::WrappingMode;

#[cfg(feature = "paging")]
pub use paging::PagingMode;
