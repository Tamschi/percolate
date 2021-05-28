# percolate

[![Lib.rs](https://img.shields.io/badge/Lib.rs-*-84f)](https://lib.rs/crates/percolate)
[![Crates.io](https://img.shields.io/crates/v/percolate)](https://crates.io/crates/percolate)
[![Docs.rs](https://docs.rs/percolate/badge.svg)](https://docs.rs/percolate)

![Rust 1.51](https://img.shields.io/static/v1?logo=Rust&label=&message=1.51&color=grey)
[![CI](https://github.com/Tamschi/percolate/workflows/CI/badge.svg?branch=unstable)](https://github.com/Tamschi/percolate/actions?query=workflow%3ACI+branch%3Aunstable)
![Crates.io - License](https://img.shields.io/crates/l/percolate/0.0.1)

[![GitHub](https://img.shields.io/static/v1?logo=GitHub&label=&message=%20&color=grey)](https://github.com/Tamschi/percolate)
[![open issues](https://img.shields.io/github/issues-raw/Tamschi/percolate)](https://github.com/Tamschi/percolate/issues)
[![open pull requests](https://img.shields.io/github/issues-pr-raw/Tamschi/percolate)](https://github.com/Tamschi/percolate/pulls)
[![crev reviews](https://web.crev.dev/rust-reviews/badge/crev_count/percolate.svg)](https://web.crev.dev/rust-reviews/crate/percolate/)

Yet another async utility library.

> Note: The API will likely contain incomplete permutations during these first `0.0.x` versions.
>
> I mainly work on this crate as I need it for other projects of mine, but feel free to [file an issue](https://github.com/Tamschi/percolate/issues/new/choose) if you'd like to see a particular feature.

## Installation

Please use [cargo-edit](https://crates.io/crates/cargo-edit) to always add the latest version of this library:

```cmd
cargo add percolate
```

## Example

```rust
//! use ergo_pin::ergo_pin;
//! use percolate::projection::{AsyncMut, IntoProjectionMut, ProjectionMut};
//! use pollster::block_on;
//! use tap::Conv;
//!
//! #[ergo_pin]
//! async fn project<A, B, X>(value: A, projection: impl IntoProjectionMut<A, B, X>) -> B {
//!     pin!(
//!         projection.into_projection_mut() // impl ProjectionMut<A, B>
//!     )                                    // Pin<&mut impl ProjectionMut<A, B>>
//!         .project(value)                  // PinHandleMut<dyn Future<B>>
//!         .await                           // B
//! }
//!
//! assert_eq!(block_on(project(1, |x: u8| x + 1)), 2);
//! assert_eq!(
//!     block_on(project(
//!         1,
//!         // Type inference doesn't understand this on its own (yet), unfortunately.
//!         // We can instead pass the projection pre-converted.
//!         (|x| async move { x + 1 }).conv::<AsyncMut<_, _, _, _>>()),
//!     ),
//!     2,
//! );
```

## License

Licensed under either of

* Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
* MIT license
   ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

## [Code of Conduct](CODE_OF_CONDUCT.md)

## [Changelog](CHANGELOG.md)

## Versioning

`percolate` strictly follows [Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html) with the following exceptions:

* The minor version will not reset to 0 on major version changes (except for v1).  
Consider it the global feature level.
* The patch version will not reset to 0 on major or minor version changes (except for v0.1 and v1).  
Consider it the global patch level.

This includes the Rust version requirement specified above.  
Earlier Rust versions may be compatible, but this can change with minor or patch releases.

Which versions are affected by features and patches can be determined from the respective headings in [CHANGELOG.md](CHANGELOG.md).
