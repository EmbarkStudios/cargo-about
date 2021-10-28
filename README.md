<div align="center">

# `📜 cargo-about`

**Cargo plugin for generating a license listing for all dependencies of a crate**

[![Embark Opensource](https://img.shields.io/badge/embark-open%20source-blueviolet.svg)](https://embark.dev)
[![Embark Discord](https://img.shields.io/badge/discord-ark-%237289da.svg?logo=discord)](https://discord.gg/Fg4u4VX)
[![Crates.io](https://img.shields.io/crates/v/cargo-about.svg)](https://crates.io/crates/cargo-about)
[![API Docs](https://docs.rs/cargo-about/badge.svg)](https://docs.rs/cargo-about)
[![SPDX Version](https://img.shields.io/badge/SPDX%20Version-3.14-blue.svg)](https://spdx.org/licenses/)
[![dependency status](https://deps.rs/repo/github/EmbarkStudios/cargo-about/status.svg)](https://deps.rs/repo/github/EmbarkStudios/cargo-about)
[![Build Status](https://github.com/EmbarkStudios/cargo-about/workflows/CI/badge.svg)](https://github.com/EmbarkStudios/cargo-about/actions?workflow=CI)

</div>

See the [book 📕](https://embarkstudios.github.io/cargo-about/) for in-depth documentation.

_Please Note: This is a tool that we use (and like!) and it makes sense to us to release it as open source. However, we can’t take any responsibility for your use of the tool, if it will function correctly or fulfil your needs. No functionality in - or information provided by - cargo-about constitutes legal advice._

## Getting started

### Installing

#### From crates.io

```bash
cargo install cargo-about
```

#### From the AUR

Arch Linux users can install [cargo-about](https://aur.archlinux.org/packages/?O=0&K=cargo-about) from the AUR using an [AUR helper](https://wiki.archlinux.org/index.php/AUR_helpers). For example,

```bash
paru -S cargo-about
```

### Generate license information for your own project

```bash
# Generates `about.toml` and `about.hbs` in your cargo project
cargo about init
# Generate the license information with
cargo about generate about.hbs > license.html
```

## Contributing

[![Contributor Covenant](https://img.shields.io/badge/contributor%20covenant-v1.4-ff69b4.svg)](CODE_OF_CONDUCT.md)

We welcome community contributions to this project.

Please read our [Contributor Guide](CONTRIBUTING.md) for more information on how to get started.

## License

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
