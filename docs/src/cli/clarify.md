<!-- markdownlint-disable no-duplicate-heading -->

# clarify

Computes a clarification for a file

## Options

## `-s, --subsections`

One or more subsections in the file which is itself its own license. Uses `!!` as the separator between the start and end of the subsection.

## `--threshold` (default: 0.8)

The minimum confidence score a license must have

## Args

### `<path>`

The relative path from the root of the source of the file to clarify.

## Subcommands

### `crate`

Retrieves the file from the git repository and commit associated with the specified crate and version.

#### Args

##### `<spec>`

The crate's `<name>-<version>` spec to retrieve. The crate source must already be downloaded.

### `repo`

Pulls the file from a git repository rather than the file system.

#### Args

##### `<rev>`

The git revision to retrieve. Can either be a commit hash or a tag.

##### `<repo>`

The full URL to the git repo. Only `github.com`, `gitlab.com`, and `bitbucket.org` are currently supported.

### `path`

Reads the license information from a path on disk.

#### Args

##### `<root>`

The path root.
