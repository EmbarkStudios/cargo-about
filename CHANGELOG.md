# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!-- next-header -->
## [Unreleased] - ReleaseDate
### Added
- [PR#148](https://github.com/EmbarkStudios/cargo-about/pull/148) added the `-o, --output-file` argument to specify a file to write to. Thanks [@MaulingMonkey](https://github.com/MaulingMonkey)!
- [PR#153](https://github.com/EmbarkStudios/cargo-about/pull/153) added the `--workspace` flag, closing [#151](https://github.com/EmbarkStudios/cargo-about/issues/151). Thanks [@MaulingMonkey](https://github.com/MaulingMonkey)!

### Changed
- [PR#157](https://github.com/EmbarkStudios/cargo-about/pull/157) returned to [`mimalloc`](https://github.com/purpleprotocol/mimalloc_rust) from `rpmalloc` to address [#137](https://github.com/EmbarkStudios/cargo-about/issues/137). The original issue with `mimalloc` relying on cmake was fixed. Thanks [@badboy](https://github.com/badboy)!
- Crates which use the same license are also now sorted lexicographically.
- Updated dependencies, namely `krates`.

## [0.2.3] - 2020-11-11
### Changed
- Updated dependencies.

## [0.2.2] - 2020-05-07
### Changed
- [PR#84](https://github.com/EmbarkStudios/cargo-about/pull/84) switched from mimalloc to rpmalloc to avoid usage of cmake which broke musl builds.

## [0.2.1] - 2020-05-06 **YANKED**
### Changed
- [PR#83](https://github.com/EmbarkStudios/cargo-about/pull/83) changed the default allocator from the system allocator to [mimalloc](https://github.com/purpleprotocol/mimalloc_rust), which should give some performance improvements, particular when building for musl.

## [0.2.0] - 2020-01-24
### Added
- `cfg()` dependendent crates can now be ignored by specifying only the `targets = []` you actually build for
- `build` and `dev` dependencies can now be optionally ignored

### Fixed
- The `used_by` list of crates that use a particular license are now always sorted lexicographically

## [0.1.1] - 2019-12-12
### Fixed
- [#20](https://github.com/EmbarkStudios/cargo-about/pull/20) Fewer files are now scanned for license information
- [#21](https://github.com/EmbarkStudios/cargo-about/pull/21) Pipes in the file system are now ignored on unix systems
- [#23](https://github.com/EmbarkStudios/cargo-about/pull/23) Fixes searching for the `about.toml` configuration file

## [0.1.0] - 2019-12-06

## [0.0.1] - 2019-11-07
### Added
- Initial add of the thing

<!-- next-url -->
[Unreleased]: https://github.com/EmbarkStudios/cargo-about/compare/0.2.3...HEAD
[0.2.3]: https://github.com/EmbarkStudios/cargo-about/compare/0.2.2...0.2.3
[0.2.2]: https://github.com/EmbarkStudios/cargo-about/compare/0.2.1...0.2.2
[0.2.1]: https://github.com/EmbarkStudios/cargo-about/compare/0.2.0...0.2.1
[0.2.0]: https://github.com/EmbarkStudios/cargo-about/compare/0.1.1...0.2.0
[0.1.1]: https://github.com/EmbarkStudios/cargo-about/compare/0.1.0...0.1.1
[0.1.0]: https://github.com/EmbarkStudios/cargo-about/compare/0.0.1...0.1.0
[0.0.1]: https://github.com/EmbarkStudios/cargo-about/releases/tag/0.0.1
