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

## Variables

These are the variables that are exposed to the templates

- `overview` - A list of [`LicenseSet`](#licenseset)
- `licenses` - A list of [`License`](#license)

## Example

```hbs
<ul class="licenses-overview">
    {{#each overview}}
    <li><a href="#{{id}}">{{name}}</a> ({{count}})</li>
    {{/each}}
</ul>
```

## Preview of the default `about.hbs`

![license](https://i.imgur.com/pvOjj06.png)

You can view the full license [here](default-example.html).
