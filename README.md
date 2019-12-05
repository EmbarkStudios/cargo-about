# 📜 cargo-about

[![Build Status](https://github.com/EmbarkStudios/cargo-about/workflows/CI/badge.svg)](https://github.com/EmbarkStudios/cargo-about/actions?workflow=CI)
[![Crates.io](https://img.shields.io/crates/v/cargo-about.svg)](https://crates.io/crates/cargo-about)
[![Docs](https://docs.rs/cargo-about/badge.svg)](https://docs.rs/cargo-about)
[![Contributor Covenant](https://img.shields.io/badge/contributor%20covenant-v1.4%20adopted-ff69b4.svg)](CODE_OF_CONDUCT.md)
[![Embark](https://img.shields.io/badge/embark-open%20source-blueviolet.svg)](https://embark.dev)

Cargo plugin for generating a listing of all of the crates used by a root crate, and the terms under which they are licensed.

## Getting started:

### Installing

```
cargo install cargo-about
```

### Generate license information for your own project

```bash
# Generates `about.toml` and `about.hbs` in your cargo project
cargo about init
# Generate the license information with
cargo about generate about.hbs > license.html
```

## `about.toml`

### `[accepted]`

Priority list of all the accepted licenses for a project. `cargo-about` will try to satisfy the licenses in the order that they are declared in this list.

```toml
accepted = [
    "Apache-2.0",
    "MIT",
]
```
### `[[DEPDENDENCY.additional]]`
* `root` Name of the root folder
* `license` Name of the license. Has to be parsable from SPDX, see https://spdx.org/licenses/
* `license-file` The path to the license file where the license is specified
* `license-start` The starting line number of the license in the specified license file
* `license-end` The ending line number of the license in the specified license file


```toml
# Example
[[physx-sys.additional]]
root = "PhysX"
license = "BSD-3-Clause"
license-file = "PhysX/README.md"
license-start = 3
license-end = 28
```

### `[[DEPDENDENCY.ignore]]`
Sometimes libraries include licenses for example code that you don't want to use.

* `license` Name of the license that you want to ingore. Has to be parsable from SPDX, see https://spdx.org/licenses/
* `license-file` The path to the license file where the license is specified

```toml
# Example
[[imgui-sys.ignore]]
license = "Zlib"
license-file = "third-party/cimgui/imgui/examples/libs/glfw/COPYING.txt"
```

## `about.hbs`
See [handlebars](https://handlebarsjs.com)

### Variables

* `overview` A list of `LicenseSet`
* `licenses` A list of `License`

### Types

#### `LicenseSet`
* `count` The number of times the license is used
* `name` The name of the license
* `id` The `id` of the license

#### `License`
* `name` The full name of the license
* `id` The SPDX identifier
* `text` The license text
* `source_path` The path of the license
* `used_by` A list of `UsedBy`

#### `UsedBy`
* `crate` Metadata for a cargo [package](https://docs.rs/cargo_metadata/newest/cargo_metadata/struct.Package.html)
* `path` Optional path of the depndency that is being used by the license

#### Example

```hbs
<ul class="licenses-overview">
    {{#each overview}}
    <li><a href="#{{id}}">{{name}}</a> ({{count}})</li>
    {{/each}}
</ul>
```

#### Preview of the default `about.hbs`
![license](https://i.imgur.com/pvOjj06.png)
You can view the full license [here](media/license.html)

## FAQ

### Unable to satisfy the following licenses

```bash
[ERROR] Crate 'aho-corasick': Unable to satisfy [Unlicense OR MIT], with the following accepted licenses [Apache-2.0]
```

In this case you are missing either `MIT` or `Unlicense` as an `accepted` license in your `about.toml`

## Contributing

We welcome community contributions to this project.

Please read our [Contributor Guide](CONTRIBUTING.md) for more information on how to get started.

## License

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
