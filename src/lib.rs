//! `bat` is a library to print syntax highlighted content.
//!
//! The main struct of this crate is `PrettyPrinter` which can be used to
//! configure and run the syntax highlighting.
//!
//! If you need more control, you can instead use the
//! [`bat-impl`](https://docs.rs/bat-impl/latest/bat_impl/) crate (start with
//! [`controller::Controller`](https://docs.rs/bat-impl/latest/bat_impl/controller/struct.Controller.html)),
//! but note that the API of these internal modules is much more likely to
//! change. Some or all of these modules might be removed in the future.
//!
//! "Hello world" example:
//! ```
//! use bat::PrettyPrinter;
//!
//! PrettyPrinter::new()
//!     .input_from_bytes(b"<span style=\"color: #ff00cc\">Hello world!</span>\n")
//!     .language("html")
//!     .print()
//!     .unwrap();
//! ```

#![deny(unsafe_code)]

mod pretty_printer;
pub use pretty_printer::{Input, PrettyPrinter};

#[cfg(feature = "paging")]
pub use bat_impl::PagingMode;
