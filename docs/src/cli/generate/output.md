# Output template

cargo-about uses handlebars templates to take the output of license gathering and transform it into your desired output. See [handlebars](https://handlebarsjs.com) for how handlebar templates work generally.

## Types

### `LicenseSet`

- `count` - The number of times the license was used to satisfy a license expression for a crate
- `name` - The name of the license
- `id` - The `id` of the license

### `License`

- `name` - The full name of the license
- `id` - The [SPDX](https://spdx.dev/ids/) identifier
- `text` - The full license text
- `source_path` - The path of the license if it was pulled from the source code of the crate
- `used_by` A list of [`UsedBy`](#usedby)

### `UsedBy`

- `crate` - Metadata for a cargo [package](https://docs.rs/cargo_metadata/newest/cargo_metadata/struct.Package.html)
- `path` - Optional path of the dependency that is being used by the license

### `Notice`

- `crate` - Metadata for the cargo package containing the NOTICE file
- `source_path` - The path of the NOTICE file in the crate's local source code
- `text` - The complete, unmodified notice text

## Variables

These are the variables that are exposed to the templates

- `overview` - A list of [`LicenseSet`](#licenseset)
- `licenses` - A list of [`License`](#license)
- `notices` - A list of [`Notice`](#notice), one per file, sorted by crate and path

## Example

```hbs
<ul class="licenses-overview">
    {{#each overview}}
    <li><a href="#{{id}}">{{name}}</a> ({{count}})</li>
    {{/each}}
</ul>
```

## Notices

Files named `NOTICE`, `NOTICE.*`, or `NOTICE-*` are collected from the local sources of all included crates, including crates with clarifications or workarounds. Collection includes subdirectories and respects the existing scan filters and `max-depth` setting. Notices are collected independently of the selected license and do not affect license resolution or grouping.

The default template displays notices in a separate section. Existing custom templates can display them by adding:

```hbs
{{#if notices}}
<h2>Notices</h2>
{{#each notices}}
    <h3>{{crate.name}} {{crate.version}}</h3>
    <pre class="license-text">{{text}}</pre>
{{/each}}
{{/if}}
```

Applications using `--format json` can render the same `notices` array in their license listing. The array is empty when no NOTICE files are found.

## Preview of the default `about.hbs`

![license](https://i.imgur.com/pvOjj06.png)

You can view the full license [here](default-example.html).
