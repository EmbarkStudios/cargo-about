# common

## `-L, --log-level`

The log level for messages, only log messages at or above the level will be emitted.

Possible values:

* `off` - No output will be emitted
* `error`
* `warn` (default)
* `info`
* `debug`
* `trace`

## `--color`

Whether coloring is applied to human-formatted output, using it on JSON output has no effect.

Possible values:

* `auto` (default) - Coloring is applied if the output stream is a TTY
* `always` - Coloring is always applied
* `never` - No coloring is applied for any output
