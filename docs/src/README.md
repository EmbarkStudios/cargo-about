# [cargo-about](https://github.com/EmbarkStudios/cargo-about)

`cargo-about` is a cargo plugin that lets you generate license information for all crates in a dependency graph.

## Quickstart

Installs cargo-about, initializes your project with a default configuration, then generates a licenses.html file.

```bash
cargo install --locked cargo-about && cargo about init && cargo about generate -o licenses.html about.hbs
```

## Command Line Interface

cargo-about is intended to be used as a [Command Line Tool](cli/index.html), see the link for the available commands and options.

## API

cargo-about is primarily meant to be used as a cargo plugin, but a majority of its functionality is within a library whose docs you may view on [docs.rs](https://docs.rs/cargo-about)
