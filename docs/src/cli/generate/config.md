# Config

Contains all of the configuration options used when running `generate`

## The `accepted` field

Priority list of all the accepted licenses for a project. `cargo-about` will try to satisfy the licenses in the order that they are declared in this list. So in the below example, if a crate is licensed with the typical `Apache-2.0 OR MIT` license expression, only the `Apache-2.0` license would be used as it has higher priority than `MIT` only one of them is required. This list applies globally to all crates. The licenses specified here are used to satisfy the license expressions for every crate, if they can't be satisfied then `cargo-about` will emit an error for why.

```ini
accepted = [
    "Apache-2.0",
    "MIT",
]
```

## The `targets` field (optional)

A list of targets that are actually building for. Crates which are only included via `cfg()` expressions that don't match one or more of the listed targets will be ignored. Note that currently the targets are evaluated all at once, so there might be cases where a crate is included that is actually impossible for any one target alone.

```ini
targets = [
    "x86_64-unknown-linux-gnu",
    "x86_64-unknown-linux-musl",
    "x86_64-pc-windows-msvc",
    "x86_64-apple-darwin",
]
```

## The `ignore-build-dependencies` field (optional)

If true, all crates that are only used as build dependencies will be ignored.

```ini
ignore-build-dependencies = true
```

## The `ignore-dev-dependencies` field (optional)

If true, all crates that are only used as dev dependencies will be ignored.

```ini
ignore-dev-dependencies = true
```

## The `no-clearly-defined` field (optional)

If true, will not attempt to lookup licensing information for any crate from <https://clearlydefined.io>, only user clarifications, workarounds, and local file scanning will be used to determine licensing information.

## The `workarounds` field (optional)

Unfortunately, not all crates properly package their licenses, or if they do, sometimes in a non-machine readable format, or in a few cases, are slightly wrong. These can be clarified manually via configuration, but some crates that are widely used in the Rust ecosystem have these issues, and rather than require that every cargo-about user who happens to have a dependency on one or more of these crates specify the same config to get it working, cargo-about instead includes a few built-in clarifications that can be opted into with a single config entry rather than redoing work.

See [Workarounds](./workarounds.md) for a list of the workarounds currently built-in to cargo-about

```ini
workarounds = [
    "ring",
    "rustls",
]
```

## Crate configuration

Along with the global options, crates can be individually configured as well, using the name of the crate as the key.



### `[[DEPENDENCY.additional]]`

* `root` Name of the root folder
* `license` Name of the license. Has to be parsable from SPDX, see <https://spdx.org/licenses/>
* `license-file` The path to the license file where the license is specified
* `license-start` The starting line number of the license in the specified license file
* `license-end` The ending line number of the license in the specified license file

```ini
# Example
[[physx-sys.additional]]
root = "PhysX"
license = "BSD-3-Clause"
license-file = "PhysX/README.md"
license-start = 3
license-end = 28
```

### `[[DEPENDENCY.ignore]]`

Sometimes libraries include licenses for example code that you don't want to use.

* `license` Name of the license that you want to ingore. Has to be parsable from SPDX, see <https://spdx.org/licenses/>
* `license-file` The path to the license file where the license is specified

```ini
[[imgui-sys.ignore]]
license = "Zlib"
license-file = "third-party/cimgui/imgui/examples/libs/glfw/COPYING.txt"
```
