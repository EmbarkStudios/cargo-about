<!-- markdownlint-disable blanks-around-headings blanks-around-lists no-duplicate-heading -->

# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!-- next-header -->
## [Unreleased] - ReleaseDate
### Changed
- [PR#275](https://github.com/EmbarkStudios/cargo-about/pull/275) updated crates.
- [PR#275](https://github.com/EmbarkStudios/cargo-about/pull/275) moved to edition 2024 and rust 1.85.0 as the MSRV.

## [0.6.6] - 2024-11-19
### Added
- [PR#268](https://github.com/EmbarkStudios/cargo-about/pull/268) resolved [#267](https://github.com/EmbarkStudios/cargo-about/issues/267) by adding a [`unicode-ident`](https://github.com/dtolnay/unicode-ident) workaround to compensate for the `LICENSE-UNICODE` file more closely matching the [`Unicode-3.0`](https://spdx.org/licenses/Unicode-3.0.html) SPDX identifier rather than the expected [`Unicode-DFS-2016`](https://spdx.org/licenses/Unicode-DFS-2016.html) one.

## [0.6.5] - 2024-11-18
### Added
- [PR#261](https://github.com/EmbarkStudios/cargo-about/pull/261) resolved [#246](https://github.com/EmbarkStudios/cargo-about/issues/246) by adding an `--offline` (as well as `--locked` and `--frozen`) option to the `generate` command.
- [PR#266](https://github.com/EmbarkStudios/cargo-about/pull/266) resolved [#230](https://github.com/EmbarkStudios/cargo-about/issues/230) by adding a `--target` option to `generate`, allowing one to specify one or more targets to filter the dependency graph by, overriding the `targets` configuration option.

### Changed
- [PR#262](https://github.com/EmbarkStudios/cargo-about/pull/262) resolved [#258](https://github.com/EmbarkStudios/cargo-about/issues/258) by using LTO for release builds, slightly decreasing binary sizes.

### Fixed
- [PR#263](https://github.com/EmbarkStudios/cargo-about/pull/263) resolved [#238](https://github.com/EmbarkStudios/cargo-about/issues/238) by adding the `native-certs` feature to use the native certificate store. This feature is not enabled by default as it is only required for corporate environments that man in the middle network traffic.
- [PR#265](https://github.com/EmbarkStudios/cargo-about/pull/265) resolved [#198](https://github.com/EmbarkStudios/cargo-about/issues/198) by detecting if the parent process is powershell and exiting with an error if cargo-about's output is being redirected instead of using the `-o` option, as powershell is terrible and doesn't use utf-8 encoding by default.
- [PR#266](https://github.com/EmbarkStudios/cargo-about/pull/266) resolved [#222](https://github.com/EmbarkStudios/cargo-about/issues/222) by adding some additional documentation on <https://clearlydefined.io>.

## [0.6.4] - 2024-08-12
### Fixed
- [PR#254](https://github.com/EmbarkStudios/cargo-about/pull/254) reverted unintended `id` -> `short_id` field rename.

## [0.6.3] **yanked** - 2024-08-12
### Changed
- [PR#251](https://github.com/EmbarkStudios/cargo-about/pull/251) updated crates and directly depend on `semver`.

### Fixed
- [PR#253](https://github.com/EmbarkStudios/cargo-about/pull/253) resolved [#250](https://github.com/EmbarkStudios/cargo-about/issues/250) by changing the example template to emit unique anchors.
- [PR#253](https://github.com/EmbarkStudios/cargo-about/pull/253) resolved [#252](https://github.com/EmbarkStudios/cargo-about/issues/252) by ignoring `SIGPIPE`.

## [0.6.2] - 2024-05-31
### Changed
- [PR#248](https://github.com/EmbarkStudios/cargo-about/pull/248) updated crates.

## [0.6.1] - 2024-01-23
### Changed
- [PR#244](https://github.com/EmbarkStudios/cargo-about/pull/244) updated `krates` => 0.16. Thanks [@kpreid](https://github.com/kpreid)!
- [PR#245](https://github.com/EmbarkStudios/cargo-about/pull/245) updated crates, notably `handlebars`.

## [0.6.0] - 2023-12-13
### Fixed
- [PR#234](https://github.com/EmbarkStudios/cargo-about/pull/234) relaxed the version restriction on the `ring` workaround to account for the 0.17.* versions.
- [PR#236](https://github.com/EmbarkStudios/cargo-about/pull/236) fixed an issue where the `count` field for each license was the number of unique licenses, rather than the number of unique crates using that license, as intended.
- [PR#240](https://github.com/EmbarkStudios/cargo-about/pull/240) resolved [#233](https://github.com/EmbarkStudios/cargo-about/issues/233) by publishing a binary for `aarch64-pc-windows-msvc`.
- [PR#240](https://github.com/EmbarkStudios/cargo-about/pull/240) resolved [#239](https://github.com/EmbarkStudios/cargo-about/issues/239) by correcting the name of the clarification field from `override_git_commit` -> `override-git-commit`.

### Changed
- [PR#235](https://github.com/EmbarkStudios/cargo-about/pull/235) and [PR#240](https://github.com/EmbarkStudios/cargo-about/pull/240) updated dependencies.

## [0.5.7] - 2023-09-02
### Changed
- [PR#231](https://github.com/EmbarkStudios/cargo-about/pull/231) updated dependencies, which included fixing [#225](https://github.com/EmbarkStudios/cargo-about/issues/225) by removing yanked crate versions, as well as getting rid of an [advisory](https://rustsec.org/advisories/RUSTSEC-2023-0052).
- [PR#231](https://github.com/EmbarkStudios/cargo-about/pull/231) updated MSRV to 1.70.0 because a dependency required it, but it also allowed use of the `IsTerminal` trait, meaning we could get rid of `atty` and the associated [advisory](https://rustsec.org/advisories/RUSTSEC-2021-0145).

## [0.5.6] - 2023-04-26
### Added
- [PR#224](https://github.com/EmbarkStudios/cargo-about/pull/224) added the `--format` option, allowing users to specify `json` to output the raw JSON used by the (previously) required handlebars templates, closing [#196](https://github.com/EmbarkStudios/cargo-about/issues/196).

## [0.5.5] - 2023-03-20
### Added
- [PR#219](https://github.com/EmbarkStudios/cargo-about/pull/219) added the `clearly-defined-timeout-secs` config option to specify a different timeout when attempting to gather license information from clearly defined. The default is 30 seconds.
- [PR#219](https://github.com/EmbarkStudios/cargo-about/pull/219) added the `max-depth` config option to specify the maximum depth from a crate's root that are searched for licenses. Most license files will be located at or near the root, so this option allows reducing the time, CPU, and memory costs associated with in-depth file scanning while still retaining the benefits of local file scanning.

### Changed
- [PR#219](https://github.com/EmbarkStudios/cargo-about/pull/219) updated dependencies.

## [0.5.4] - 2023-02-01
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
[Unreleased]: https://github.com/EmbarkStudios/cargo-about/compare/0.6.6...HEAD
[0.6.6]: https://github.com/EmbarkStudios/cargo-about/compare/0.6.5...0.6.6
[0.6.5]: https://github.com/EmbarkStudios/cargo-about/compare/0.6.4...0.6.5
[0.6.4]: https://github.com/EmbarkStudios/cargo-about/compare/0.6.3...0.6.4
[0.6.3]: https://github.com/EmbarkStudios/cargo-about/compare/0.6.2...0.6.3
[0.6.2]: https://github.com/EmbarkStudios/cargo-about/compare/0.6.1...0.6.2
[0.6.1]: https://github.com/EmbarkStudios/cargo-about/compare/0.6.0...0.6.1
[0.6.0]: https://github.com/EmbarkStudios/cargo-about/compare/0.5.7...0.6.0
[0.5.7]: https://github.com/EmbarkStudios/cargo-about/compare/0.5.6...0.5.7
[0.5.6]: https://github.com/EmbarkStudios/cargo-about/compare/0.5.5...0.5.6
[0.5.5]: https://github.com/EmbarkStudios/cargo-about/compare/0.5.4...0.5.5
[0.5.4]: https://github.com/EmbarkStudios/cargo-about/compare/0.5.3...0.5.4
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
