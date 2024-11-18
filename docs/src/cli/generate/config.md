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

## The `ignore-transitive-dependencies` field (optional)

If true, only direct dependencies of crates in the workspace will be included in the graph, transitive dependencies (dependencies of dependencies) will be ignored.

```ini
ignore-transitive-dependencies = true
```

## The `no-clearly-defined` field (optional)

If true, will not attempt to lookup licensing information for any crate from [clearlydefined.io], only user clarifications, workarounds, and local file scanning will be used to determine licensing information.

By default, `cargo-about` will use [clearlydefined.io] to augment the license information that can be gathered by scanning local files, as it has more advanced license detection (eg. it can detect multiple license in the same file unlike askalono), and can have [curations](https://docs.clearlydefined.io/docs/get-involved/data-curation) applied that benefit all users of a crate, rather than the project-specific clarifications supported by `cargo-about`.

[clearlydefined.io] does have some downsides however, in that it is an external source of information that can be missing or updated, which can result in different output given the same dependency graph input.

It will also show warnings for when the license information for a crate cannot be retrieved, the most common of which is

> the definition for <crate> has not been harvested

which indicates that the particular crate version has not been scanned and indexed by [clearlydefined.io] yet. Simply by making a request for a crate version from cargo-about, [clearlydefined.io] will automatically queue it to be harvested, but depending on load may take several hours or more before it is available.

## The `filter-noassertion` field (optional)

If using [clearlydefined.io] to gather license information, that service will conservatively add [`NOASSERTION`](https://docs.clearlydefined.io/curation-guidelines) to the expression for files that contain license like data, but an SPDX license ID could not be confidently ascribed to it. This can result in the license expression for the crate to contain 1 or more `NOASSERTION` identifiers, which would require the user to accept that (not really valid) ID to pass the license check. By setting this field to `true`, files that have a `NOASSERTION` id will instead be scanned locally, which will generally either figure out the license, or else skip that file.

For a real world example of what this looks like, [`webpki:0.22.0`](https://crates.io/crates/webpki/0.22.0)'s [LICENSE](https://clearlydefined.io/file/5b698ca13897be3afdb7174256fa1574f8c6892b8bea1a66dd6469d3fe27885a) file is an ISC license, however it has a preamble that is not part of the ISC license that trips up clearly defined's inspection, causing it to be attributed with `ISC AND NOASSERTION`. Locally scanning the file will be more tolerant and just attribute it with `ISC`.

## The `workarounds` field (optional)

Unfortunately, not all crates properly package their licenses, or if they do, sometimes in a non-machine readable format, or in a few cases, are slightly wrong. These can be clarified manually via configuration, but some crates that are widely used in the Rust ecosystem have these issues, and rather than require that every cargo-about user who happens to have a dependency on one or more of these crates specify the same config to get it working, cargo-about instead includes a few built-in clarifications that can be opted into with a single config entry rather than redoing work.

See [Workarounds](./workarounds.md) for a list of the workarounds currently built-in to cargo-about

```ini
workarounds = [
    "ring",
    "rustls",
]
```

## The `private` field (optional)

It's often not useful or wanted to check for licenses in your own private workspace crates. So the private field allows you to do so.

### The `ignore` field

If `true`, workspace members will not have their license expression checked, _if_ they are not published.

```ini
# Cargo.toml
[package]
name = "sekret"
license = "¯\_(ツ)_/¯"
publish = false # "private"!
```

```ini
# about.toml

# The sekret package would be ignored now
private = { ignore = true }
```

### The `registries` field

A list of private registries you may publish your workspace crates to. If a workspace member **only** publishes to private registries, it will also be ignored if `private.ignore = true`

```ini
# Cargo.toml
[package]
name = "sekret"
license = "¯\_(ツ)_/¯"
publish = ["sauce"]
```

```ini
# about.toml

# Still ignored!
private = { ignore = true, registries = ["sauce"] }
```

## Crate configuration

Along with the global options, crates can be individually configured as well, using the name of the crate as the key. Crate specific configuration _must_ come last in the config file.

### The `accepted` field (optional)

Just as with the global [`accepted`](#the-accepted-field) field, this accepts specific licenses for the crate. These licenses are appended to the global list, and are again in priority order. So for example, if the global accept was like this:

```ini
accepted = ["MIT", "ISC"]
```

And we are using `ring`, which also is licensed under the [`OpenSSL`](https://spdx.org/licenses/OpenSSL.html) license, we could use the following configuration to satisfy the license requirements of `ring`.

```ini
accepted = ["MIT", "ISC"]

[ring]
accepted = ["OpenSSL"]
```

### The `clarify` field (optional)

As noted in the [`workarounds`](#the-workarounds-field-optional), some crates have complicated or incomplete licensing that messes up the harvesting of the license info in an automated fashion. While the `workarounds` exists for popular crates (and can always be expanded with PRs!) there are often going to be crates that you will need to clarify yourself until a new release of the crate, etc, which is the purpose of the `clarify` field, to specify exactly what the license information is, and how to verify that the license terms are still the same as when they were clarified, using hashes of the input files.

Note that since clarifications are human supplied in your project's own configuration, they take precedence over all other methods. If a crate is clarified, it will not be retrieved from clearlydefined.io nor via local file harvesting.

#### The `license` field

This is the top level SPDX expression for the crate as a whole. It should be noted that this actually overrides the `license` expression of the crate itself if it exists, though in most cases this will be the same as the stated `license`, it is simply required so that you can't accidentally forget it in the cases where it _does_ differ.

```ini
[ring.clarify]
license = "ISC AND MIT AND OpenSSL"
```

#### The `override-git-commit` field (optional)

When clarifying a crate with files pulled from its source git repository, cargo-about will normally read the contents of the `.cargo_vcs_info.json` file that is usually part of a published crate's contents, which includes the full commit hash at the time the crate was published. However, this file is not guaranteed to be present (eg, if the crate is published with `cargo publish --allow-dirty`), so in that case the git ref to pull the license file contents must be supplied. This can either be a full git revision, or a git tag.

```ini
[core-graphics-types.clarify]
override-git-commit = "3841d2bb3aa76dec2ea6319e757603fb923b5a50"
```

#### The `files` and/or `git` field

When clarifying the license of a crate, it is **required** to give a source of truth for the licenses in the expression, to prevent drift between the clarification and the actual licensing of the crate in question. For example, if a crate uses the `Zlib` license, then changes between releases to use the `MIT` license instead, the source of truth (eg. the `LICENSE` file) would also (hopefully...) change resulting in a hash mismatch that means the clarification would not be used.

We'll be using this example for the `ring` crate

```ini
[ring.clarify]
license = "ISC AND MIT AND OpenSSL"

[[ring.clarify.files]]
path = 'LICENSE'
license = 'OpenSSL'
checksum = '53552a9b197cd0db29bd085d81253e67097eedd713706e8cd2a3cc6c29850ceb'
start = '/* ===================================================================='
end = '\n * Hudson (tjh@cryptsoft.com).\n *\n */'
```

##### The `path` field

This is the relative path to the file from the root. For `files` this is the root of the crate, but for `git` this is the repo root.

##### The `license` field (optional)

In a multiple license situation it can be useful to supply the exact license for the file. If this is not supplied the parent `clarify.license` expression is used instead.

##### The `checksum` field

This is the full sha-256 checksum of the contents. If this doesn't match the computed checksum, the clarification will not be used.

##### The `start` field (optional)

In some cases, crates concatenate multiple licenses together into a single file, which confuses machine readers, and makes splatting the license text into the final generated template a pain, so in those cases you need to supply a place in the text that a license starts and/or ends from. This is just a simple substring find.

##### The `end` field (optional)

Just as with start, this is just a simple substring find, however, it will only match text that comes _after_ the position the start text (or beginning of the file) was found.

[clearlydefined.io]: https://clearlydefined.io
