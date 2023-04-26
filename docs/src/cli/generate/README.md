# generate

The generate subcommand is the primary subcommand of `cargo-about`. It attempts to find and satisfy all license requirements for a crate's or workspace's dependency graph and generate licensing output based on one or more handlebar templates.

## Flags

### `--all-features` (single crate or workspace)

Enables all features when determining which crates to consider. Works for both single crates and workspaces.

### `--no-default-features` (single crate only)

Disables the `default` feature for a crate when determining which crates to consider.

### `--workspace`

Scan licenses for the entire workspace, not just the active package.

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

#### `-o, --output`

A file to write the generated output to. Typically an `.html` file.

#### `--threshold` (default: 0.8)

The confidence threshold required for license files to be positively identified: `0.0 - 1.0`

#### `--format <json|handlebars>` (default: `handlebars`)

The format to output the license + crate data in.

## Args

### `<templates>`

The template(s) or template directory to use. Must either be a `.hbs` file, or have at least one `.hbs` file in it if it is a directory. Required if `--format = handlebars` (the default).
