<!-- markdownlint-disable blanks-around-headings blanks-around-lists no-duplicate-heading -->

# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!-- next-header -->
## [Unreleased] - ReleaseDate
### Added
- [PR#216](https://github.com/EmbarkStudios/cargo-about/pull/216) add the `filter-noassertion` configuration, allowing users to use local scanning for files that <clearlydefined.io> adds `NOASSERTION` license ids to so that they are properly attributed or ignored.

## [0.5.3] - 2023-01-27
### Fixed
- [PR#213](https://github.com/EmbarkStudios/cargo-about/pull/213) resolved [#203](https://github.com/EmbarkStudios/cargo-about/issues/203) by adding the `--fail` option to the generate subcommand. Thanks [@mikayla-maki](https://github.com/mikayla-maki)!

## [0.5.2] - 2022-11-25
### Changed
- [PR#205](https://github.com/EmbarkStudios/cargo-about/pull/205) updated to spdx 0.9 and therefore the SPDX license from version 3.14 -> 3.18. Thanks [@o0Ignition0o](https://github.com/o0Ignition0o)!

## [0.5.1] - 2022-04-05
### Added
- [PR#188](https://github.com/EmbarkStudios/cargo-about/pull/188) added the ability to ignore transitive dependencies via the `ignore-transitive-dependencies` config flag. Thanks [@haraldreingruber](https://github.com/haraldreingruber)!
- [PR#188](https://github.com/EmbarkStudios/cargo-about/pull/188) added a `crates` property to the handlebars context, see the [about_list_by_crate_example](about_list_by_crate_example.hbs) for how it can be used. Thanks [@haraldreingruber](https://github.com/haraldreingruber)!

### Changed
- [PR#189](https://github.com/EmbarkStudios/cargo-about/pull/189) updated dependencies, notably `regex` to fix an [advisory](https://rustsec.org/advisories/RUSTSEC-2022-0013).

## [0.5.0] - 2022-03-04
### Changed
- [PR#187](https://github.com/EmbarkStudios/cargo-about/pull/187) closed [#185](https://github.com/EmbarkStudios/cargo-about/issues/185) by making it so that **all** crates marked as `publish = false` will be ignored, rather than the previous behavior of only ignore workspace members. Please file an issue if this behavior is not acceptable. Thanks [@danielnelson](https://github.com/danielnelson)!

## [0.4.8] - 2022-03-02
### Fixed
- [PR#184](https://github.com/EmbarkStudios/cargo-about/pull/184) fixed [#183](https://github.com/EmbarkStudios/cargo-about/issues/183) to correct an issue where licenses were misattributed to crates if 1 or more crates was marked as `publish = false` and private crates were ignored in the config. Thanks [@danielnelson](https://github.com/danielnelson)!

## [0.4.7] - 2022-02-09
### Fixed
- [PR#182](https://github.com/EmbarkStudios/cargo-about/pull/182) fixed [#181](https://github.com/EmbarkStudios/cargo-about/issues/181) by adding version, author, and about metadata to the CLI output, as `structopt` by default added that, but `clap` v3 does not.

## [0.4.6] - 2022-02-07
### Fixed
- [PR#180](https://github.com/EmbarkStudios/cargo-about/pull/180) fixed [#179](https://github.com/EmbarkStudios/cargo-about/issues/179) by setting the MSRV to 1.56.1 and adding a CI check for it.

## [0.4.5] - 2022-02-04
### Changed
- [PR#178](https://github.com/EmbarkStudios/cargo-about/pull/178) updated dependencies.

## [0.4.4] - 2021-12-23
### Fixed
- [PR#177](https://github.com/EmbarkStudios/cargo-about/pull/177) updated the structure for the `.cargo_vcs_info.json` file since it now contains the path in the repo of the crate.

### Changed
- [PR#177](https://github.com/EmbarkStudios/cargo-about/pull/177) updated dependencies

## [0.4.3] - 2021-11-22
### Fixed
- [PR#176](https://github.com/EmbarkStudios/cargo-about/pull/176) fixed [#175](https://github.com/EmbarkStudios/cargo-about/issues/175) by updating `askalono` which was causing `cargo install` failures due to `cargo install`'s default behavior of not using the `Cargo.lock` file. This got rid of the `failure` dependency as well, which was pulling in a lot of additional crates that are now gone.

## [0.4.2] - 2021-11-21
### Changed
- [PR#174](https://github.com/EmbarkStudios/cargo-about/pull/174) updated dependencies, including `tokio` to fix an [advisory](https://rustsec.org/advisories/RUSTSEC-2021-0124).

## [0.4.1] - 2021-11-01
### Added
- [PR#172](https://github.com/EmbarkStudios/cargo-about/pull/172) resolved [#171](https://github.com/EmbarkStudios/cargo-about/issues/171) by adding support for ignoring private workspace crates.

## [0.4.0] - 2021-10-28
### Added
- [PR#168](https://github.com/EmbarkStudios/cargo-about/pull/168) added the ability to retrieve harvested license data from [clearlydefined.io](https://clearlydefined.io/about), which generally has superior machine harvested data to the old of approach of relying completely on askalono and local file scanning. This gathering is enabled by default, but can be turned off with the `no-clearly-defined` option in the config.
- [PR#168](https://github.com/EmbarkStudios/cargo-about/pull/168) added the concept of clarifications, which are essentially user specified overrides for the license for a crate, using 1 or more sources of truth to ensure there is no drift between the clarification and the crate license over time.
- [PR#168](https://github.com/EmbarkStudios/cargo-about/pull/168) added built-in `workarounds`, which are just opt-in clarifications that are built-in to `cargo-about` itself so that users of `cargo-about` don't have to repeat the same clarification process for various popular crates in the ecosystem.
- [PR#168](https://github.com/EmbarkStudios/cargo-about/pull/168) added the `clarify` subcommand, which can be used to help you clarify particular crates.
- [PR#168](https://github.com/EmbarkStudios/cargo-about/pull/168) added support for `accepted` licenses on a per-crate basis in addition to the global `accepted` licenses.
- [PR#169](https://github.com/EmbarkStudios/cargo-about/pull/169) added an mdbook at <https://embarkstudios.github.io/cargo-about/> to give improved documentation over the previous README.md only approach.

### Changed
- [PR#168](https://github.com/EmbarkStudios/cargo-about/pull/168) moved to [Rust 1.56.0 as well as the 2021 edition](https://blog.rust-lang.org/2021/10/21/Rust-1.56.0.html).

### Removed
- [PR#169](https://github.com/EmbarkStudios/cargo-about/pull/169) removed the `additional` and `ignore` crate configuration in favor of clarifications and/or the better harvested content from clearlydefined.io.

## [0.3.0] - 2021-03-17
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
[Unreleased]: https://github.com/EmbarkStudios/cargo-about/compare/0.5.3...HEAD
[0.5.3]: https://github.com/EmbarkStudios/cargo-about/compare/0.5.2...0.5.3
[0.5.2]: https://github.com/EmbarkStudios/cargo-about/compare/0.5.1...0.5.2
[0.5.1]: https://github.com/EmbarkStudios/cargo-about/compare/0.5.0...0.5.1
[0.5.0]: https://github.com/EmbarkStudios/cargo-about/compare/0.4.8...0.5.0
[0.4.8]: https://github.com/EmbarkStudios/cargo-about/compare/0.4.7...0.4.8
[0.4.7]: https://github.com/EmbarkStudios/cargo-about/compare/0.4.6...0.4.7
[0.4.6]: https://github.com/EmbarkStudios/cargo-about/compare/0.4.5...0.4.6
[0.4.5]: https://github.com/EmbarkStudios/cargo-about/compare/0.4.4...0.4.5
[0.4.4]: https://github.com/EmbarkStudios/cargo-about/compare/0.4.3...0.4.4
[0.4.3]: https://github.com/EmbarkStudios/cargo-about/compare/0.4.2...0.4.3
[0.4.2]: https://github.com/EmbarkStudios/cargo-about/compare/0.4.1...0.4.2
[0.4.1]: https://github.com/EmbarkStudios/cargo-about/compare/0.4.0...0.4.1
[0.4.0]: https://github.com/EmbarkStudios/cargo-about/compare/0.3.0...0.4.0
[0.3.0]: https://github.com/EmbarkStudios/cargo-about/compare/0.2.3...0.3.0
[0.2.3]: https://github.com/EmbarkStudios/cargo-about/compare/0.2.2...0.2.3
[0.2.2]: https://github.com/EmbarkStudios/cargo-about/compare/0.2.1...0.2.2
[0.2.1]: https://github.com/EmbarkStudios/cargo-about/compare/0.2.0...0.2.1
[0.2.0]: https://github.com/EmbarkStudios/cargo-about/compare/0.1.1...0.2.0
[0.1.1]: https://github.com/EmbarkStudios/cargo-about/compare/0.1.0...0.1.1
[0.1.0]: https://github.com/EmbarkStudios/cargo-about/compare/0.0.1...0.1.0
[0.0.1]: https://github.com/EmbarkStudios/cargo-about/releases/tag/0.0.1
