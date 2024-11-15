# generate

The generate subcommand is the primary subcommand of `cargo-about`. It attempts to find and satisfy all license requirements for a crate's or workspace's dependency graph and generate licensing output based on one or more handlebar templates.

## Flags

### `--all-features` (single crate or workspace)

Enables all features when determining which crates to consider. Works for both single crates and workspaces.

### `--no-default-features` (single crate only)

Disables the `default` feature for a crate when determining which crates to consider.

### `--workspace`

Scan licenses for the entire workspace, not just the active package.

### [`--locked`](https://doc.rust-lang.org/cargo/commands/cargo-fetch.html#option-cargo-fetch---locked)

Asserts that the exact same dependencies and versions are used as when the existing Cargo.lock file was originally generated. Cargo will exit with an error when either of the following scenarios arises:

* The lock file is missing.
* Cargo attempted to change the lock file due to a different dependency resolution.

### [`--offline`](https://doc.rust-lang.org/cargo/commands/cargo-fetch.html#option-cargo-fetch---offline)

Prevents Cargo and `cargo-about` from accessing the network for any reason. Without this flag, Cargo will stop with an error if it needs to access the network and the network is not available. With this flag, Cargo will attempt to proceed without the network if possible.

Beware that this may result in different dependency resolution than online mode. Cargo will restrict itself to crates that are downloaded locally, even if there might be a newer version as indicated in the local copy of the index. See the cargo-fetch(1) command to download dependencies before going offline.

`cargo-about` will also not query clearlydefined.io for license information, meaning that user provided clarifications won't be used, and some ambiguous/complicated license files might be missed by `cargo-about`. Additionally, clarifications that use license files from the crate's source repository will not be applied, meaning that `cargo-about` will fallback to using the default license text rather than the one in the source repository, losing eg. copyright or other unique information.

### [`--frozen`](https://doc.rust-lang.org/cargo/commands/cargo-fetch.html#option-cargo-fetch---frozen)

Equivalent to specifying both `--locked` and `--offline`.

### `--fail`

Exits with a non-zero exit code if any crate's license cannot be reasonably determined

## Options

### `-c, --config`

Path to the [config](config.md) to use. Will default to `<manifest_root/about.toml>` if not specified.

#### `--features` (single crate only)

Space-separated list of features to enable when determining which crates to consider.

#### `-i, --include-local`

Include local crates beneath one or more directories, local crates are disregarded by default.

#### `-m, --manifest-path`

The path of the Cargo.toml for the root crate, defaults to the current crate or workspace in the current working directory.

#### `-n, --name`

The name of the template to use when rendering. If only passing a single template file to [`templates`](#templates) this is not used.

#### `-o, --output-file`

A file to write the generated output to. Typically an `.html` file.

#### `--threshold` (default: 0.8)

The confidence threshold required for license files to be positively identified: `0.0 - 1.0`

#### `--format <json|handlebars>` (default: `handlebars`)

The format to output the license + crate data in.

## Args

### `<templates>`

The template(s) or template directory to use. Must either be a `.hbs` file, or have at least one `.hbs` file in it if it is a directory. Required if `--format = handlebars` (the default).
