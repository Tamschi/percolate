# percolate Changelog

<!-- markdownlint-disable no-trailing-punctuation -->

## 0.0.2

2021-05-30

- **Breaking Changes:**
  - Fixed name of `IntoRefProjection::into_ref_projection` from `into_ref_projection_mut`
  - `RefProjection` traits require `MutProjection` traits
  - ~~`Mut`~~`Predicate` traits require `MutPredicate` traits

- New Features:
  - `MutProjection` traits
  - `MutPredicate` traits
  - `FusedMutBlockingMut` `Fn(&mut A) -> B` adapter
  - `PeekStream::{peek_1_mut, peek_n_mut}` methods
  - `PeekStream::next_if_mut` method

- Revisions:
  - Fixed README example

## 0.0.1

2021-05-29

Initial unstable release
