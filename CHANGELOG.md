# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!-- next-header -->
## [Unreleased] - ReleaseDate
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
[Unreleased]: https://github.com/EmbarkStudios/cargo-about/compare/0.2.0...HEAD
[0.2.0]: https://github.com/EmbarkStudios/cargo-about/compare/0.1.1...0.2.0
[0.1.1]: https://github.com/EmbarkStudios/cargo-about/compare/0.1.0...0.1.1
[0.1.0]: https://github.com/EmbarkStudios/cargo-about/compare/0.0.1...0.1.0
[0.0.1]: https://github.com/EmbarkStudios/cargo-about/releases/tag/0.0.1
