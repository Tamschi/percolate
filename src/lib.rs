//! Yet another async utility library.
//!
//! # About the Documentation
//!
//! ## RFC 2119 Blurb (modified stylization)
//!
//! The key words **must**, **must not**, **required**, **shall**, **shall
//! not**, **should**, **should not**, **recommended**,  **may**, and
//! **OPTIONAL** in this document are to be interpreted as described in
//! [RFC 2119](https://datatracker.ietf.org/doc/html/rfc2119).
//!
//! ## Sets of Items by Partial Names
//!
//! Since this crate contains many traits which often follow naming patterns,
//! sets of these items are sometimes referred to by parts of their name.
//! In particular, in order of descending precedence:
//!
//! - `Part` traits -> traits with "Part" in their name
//! - <code><s>Part</s></code> traits -> traits that do *not* have "Part" in their name
//! - `Part1`/`Part2` -> traits with "Part1" OR "Part2" in their name *in an alternatives slot that can have either*
//! - `Part1` `Part2` -> traits with "Part1" AND "Part2" in their name
//!
//! When these phrases appear in a module of this crate, they only refer to types exported there unless otherwise specified.
//!
//! <!--
//! NOTE: When documenting, use inline HTML to describe strikethrough code!
//! Rustdoc does not render strikethrough correctly in the first item documentation paragraph shown in a module overview,
//! and even outside of that, most instances should use `<code><s>…</s></code>` rather than `<del><code>…</code></del>`,
//! the latter of which is what ~~`…`~~ renders as.
//!
//! This way, code can also be partially strikethrough, which avoids unwanted spaces.
//! -->
//!
//! ### Example
//!
//! "<code><s>Into</s></code> `Ref`/`Mut` traits"
//! includes [`RefProjection`](`projection::RefProjection`) and [`RefProjectionMut`](`projection::RefProjectionMut`),
//! but neither [`IntoProjection`](`projection::IntoProjection`) nor [`ProjectionMut`](`projection::ProjectionMut`)  (as `Ref` can't be trailing).

#![doc(html_root_url = "https://docs.rs/percolate/0.0.2")]
#![no_std]
#![warn(clippy::pedantic)]
#![allow(
	clippy::if_not_else,
	clippy::module_name_repetitions,
	clippy::redundant_else,
	clippy::single_match_else
)]

#[cfg(doctest)]
pub mod readme {
	doc_comment::doctest!("../README.md");
}

pub mod handles;
pub mod predicate;
pub mod projection;
pub mod stream;
