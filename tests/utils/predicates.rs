use crate::utils::Package;

use predicates::prelude::*;
use predicates::str::contains;
use predicates::Predicate;

pub fn mit_license_text(year: &str, copyright_holder: &str) -> String {
    format! ("\
            Copyright (c) {year} {copyright_holder}\n\
            \n\
            Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the \"Software\"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:\n\
            \n\
            The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.\n\
            \n\
            THE SOFTWARE IS PROVIDED \"AS IS\", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.\n\
    "
    )
}

pub fn contains_default_mit_license_content() -> impl Predicate<str> {
    contains_mit_license_content("<year>", "<copyright holders>")
}

pub fn contains_default_apache2_license_content() -> impl Predicate<str> {
    contains_apache2_license_content("[yyyy]", "[name of copyright owner]")
}

pub fn contains_mit_license_content(year: &str, copyright_holder: &str) -> impl Predicate<str> {
    contains(mit_license_text(year, copyright_holder))
}

pub fn contains_apache2_license_content(year: &str, copyright_holder: &str) -> impl Predicate<str> {
    let header = "\
        Apache License\n\
        Version 2.0, January 2004\n\
    ";
    let copyright = format!("Copyright {year} {copyright_holder}");

    contains(header).and(contains(copyright))
}

pub fn overview_count(count: usize) -> impl Predicate<str> {
    contains(format!("#o:[{}]", "o".repeat(count)))
}

pub fn licenses_count(count: usize) -> impl Predicate<str> {
    contains(format!("#l:[{}]", "l".repeat(count)))
}

pub fn contains_missing_license_field_warning(package: &Package) -> impl Predicate<str> {
    contains(format!(
        "crate '{} {}' doesn't have a license field",
        package.name, package.version
    ))
}

pub fn unable_to_synthesize_license_expr_warning(package: &Package) -> impl Predicate<str> {
    contains(format!(
        "unable to synthesize license expression for '{} {}': \
            no `license` specified, and no license files were found",
        package.name, package.version
    ))
}
