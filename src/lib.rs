#![doc(html_root_url = "https://docs.rs/percolate/0.0.1")]
#![no_std]
#![warn(clippy::pedantic)]
#![allow(clippy::single_match_else)]

#[cfg(doctest)]
pub mod readme {
	doc_comment::doctest!("../README.md");
}

pub mod handles;
pub mod peek_stream;
pub mod predicate;
pub mod projection;
