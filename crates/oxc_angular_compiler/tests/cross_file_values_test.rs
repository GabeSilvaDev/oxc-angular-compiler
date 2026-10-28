//! End-to-end tests for `TransformOptions::resolve_imported_values`
//! (#518): decorator metadata that references values exported from other
//! files (`inputs: INPUTS`, `@Input(OPTS)`) must evaluate like ngtsc's
//! program-wide checker instead of erroring with "imported from another
//! module". Unresolvable imports keep that diagnostic.

#![cfg(feature = "cross_file_elision")]

use oxc_allocator::Allocator;
use oxc_angular_compiler::{TransformOptions, TransformResult, transform_angular_file};
use tempfile::TempDir;

fn create_test_file(dir: &std::path::Path, name: &str, content: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&path, content).unwrap();
    path
}

/// The whitespace-free `ɵɵdefineDirective` call for `class_name`.
fn define_call(code: &str, class_name: &str) -> String {
    let compact: String = code.chars().filter(|c| !c.is_whitespace()).collect();
    let start = compact.find(&format!("({{type:{class_name},")).expect("define call");
    let end = compact[start..].find("});").map_or(compact.len(), |i| start + i);
    compact[start..end].to_string()
}

fn transform(dir: &TempDir, source: &str, options: &TransformOptions) -> TransformResult {
    let path = create_test_file(dir.path(), "app/test.ts", source);
    let allocator = Allocator::default();
    transform_angular_file(&allocator, path.to_str().unwrap(), source, Some(options), None)
}

fn resolve_options() -> TransformOptions {
    TransformOptions { resolve_imported_values: true, ..Default::default() }
}

fn error_messages(result: &TransformResult) -> String {
    result.diagnostics.iter().map(|d| d.to_string()).collect::<Vec<_>>().join("\n")
}

#[test]
fn imported_inputs_array_evaluates() {
    let dir = TempDir::new().unwrap();
    create_test_file(dir.path(), "app/meta.ts", "export const INPUTS = ['x', 'y: why'];");
    let result = transform(
        &dir,
        r#"import { Directive } from '@angular/core';
import { INPUTS } from './meta';
@Directive({ selector: 'a', inputs: INPUTS })
export class A {}
"#,
        &resolve_options(),
    );
    assert!(!result.has_errors(), "{}", error_messages(&result));
    assert!(
        define_call(&result.code, "A").contains(r#"inputs:{x:"x",y:[0,"why","y"]}"#),
        "{}",
        define_call(&result.code, "A")
    );
}

#[test]
fn input_member_options_object_evaluates() {
    let dir = TempDir::new().unwrap();
    create_test_file(
        dir.path(),
        "app/meta.ts",
        "export const OPTS = { alias: 'r', required: true };",
    );
    let result = transform(
        &dir,
        r#"import { Directive, Input } from '@angular/core';
import { OPTS } from './meta';
@Directive({ selector: 'a' })
export class A { @Input(OPTS) prop?: string; }
"#,
        &resolve_options(),
    );
    assert!(!result.has_errors(), "{}", error_messages(&result));
    assert!(
        define_call(&result.code, "A").contains(r#"inputs:{prop:[0,"r","prop"]}"#),
        "{}",
        define_call(&result.code, "A")
    );
}

#[test]
fn namespace_import_member_evaluates() {
    let dir = TempDir::new().unwrap();
    create_test_file(dir.path(), "app/meta.ts", "export const X = { alias: 'x-x' };");
    let result = transform(
        &dir,
        r#"import { Directive, Input } from '@angular/core';
import * as meta from './meta';
@Directive({ selector: 'a' })
export class A { @Input(meta.X) prop?: string; }
"#,
        &resolve_options(),
    );
    assert!(!result.has_errors(), "{}", error_messages(&result));
    assert!(
        define_call(&result.code, "A").contains(r#"inputs:{prop:[0,"x-x","prop"]}"#),
        "{}",
        define_call(&result.code, "A")
    );
}

#[test]
fn imported_values_follow_re_export_chain() {
    let dir = TempDir::new().unwrap();
    create_test_file(dir.path(), "app/impl.ts", "export const INPUTS = ['x'];");
    create_test_file(dir.path(), "app/inner.ts", "export { INPUTS as X } from './impl';");
    create_test_file(dir.path(), "app/outer.ts", "export * from './inner';");
    let result = transform(
        &dir,
        r#"import { Directive } from '@angular/core';
import { X } from './outer';
@Directive({ selector: 'a', inputs: X })
export class A {}
"#,
        &resolve_options(),
    );
    assert!(!result.has_errors(), "{}", error_messages(&result));
    assert!(
        define_call(&result.code, "A").contains(r#"inputs:{x:"x"}"#),
        "{}",
        define_call(&result.code, "A")
    );
}

#[test]
fn read_files_are_reported_as_dependencies() {
    let dir = TempDir::new().unwrap();
    let meta = create_test_file(dir.path(), "app/meta.ts", "export const INPUTS = ['x'];");
    let result = transform(
        &dir,
        r#"import { Directive } from '@angular/core';
import { INPUTS } from './meta';
@Directive({ selector: 'a', inputs: INPUTS })
export class A {}
"#,
        &resolve_options(),
    );
    assert!(!result.has_errors(), "{}", error_messages(&result));
    // Compare canonicalized paths: the resolver's `full_path()` can differ
    // from `tempdir`'s form (macOS /var -> /private/var, Windows `\\?\`
    // verbatim prefixes and 8.3 short names like RUNNER~1).
    let meta = std::fs::canonicalize(&meta).unwrap();
    assert!(
        result.dependencies.iter().any(|d| std::fs::canonicalize(d).is_ok_and(|c| c == meta)),
        "dependencies {:?} should contain {}",
        result.dependencies,
        meta.display()
    );
}

#[test]
fn unresolvable_import_keeps_diagnostic() {
    let dir = TempDir::new().unwrap();
    // `require()` isn't statically evaluable; the import stays opaque.
    create_test_file(dir.path(), "app/meta.ts", "export const INPUTS = require('./other');");
    let result = transform(
        &dir,
        r#"import { Directive } from '@angular/core';
import { INPUTS } from './meta';
@Directive({ selector: 'a', inputs: INPUTS })
export class A {}
"#,
        &resolve_options(),
    );
    assert!(result.has_errors());
    let errors = error_messages(&result);
    assert!(errors.contains("'INPUTS', which is imported from another module"), "{errors}");
}

#[test]
fn missing_export_keeps_diagnostic() {
    let dir = TempDir::new().unwrap();
    create_test_file(dir.path(), "app/meta.ts", "export const OTHER = ['x'];");
    let result = transform(
        &dir,
        r#"import { Directive } from '@angular/core';
import { INPUTS } from './meta';
@Directive({ selector: 'a', inputs: INPUTS })
export class A {}
"#,
        &resolve_options(),
    );
    assert!(result.has_errors());
    assert!(
        error_messages(&result).contains("'INPUTS', which is imported from another module"),
        "{}",
        error_messages(&result)
    );
}

#[test]
fn feature_off_keeps_diagnostic() {
    let dir = TempDir::new().unwrap();
    create_test_file(dir.path(), "app/meta.ts", "export const INPUTS = ['x'];");
    let result = transform(
        &dir,
        r#"import { Directive } from '@angular/core';
import { INPUTS } from './meta';
@Directive({ selector: 'a', inputs: INPUTS })
export class A {}
"#,
        &TransformOptions::default(),
    );
    assert!(result.has_errors());
    assert!(
        error_messages(&result).contains("'INPUTS', which is imported from another module"),
        "{}",
        error_messages(&result)
    );
}

#[test]
fn package_import_is_not_resolved() {
    let dir = TempDir::new().unwrap();
    let result = transform(
        &dir,
        r#"import { Directive } from '@angular/core';
import { INPUTS } from 'some-pkg';
@Directive({ selector: 'a', inputs: INPUTS })
export class A {}
"#,
        &resolve_options(),
    );
    assert!(result.has_errors());
    assert!(
        error_messages(&result).contains("'INPUTS', which is imported from another module"),
        "{}",
        error_messages(&result)
    );
}

// `styles` in `@Component` resolves through the same imported bindings.

#[test]
fn imported_styles_array_evaluates() {
    let dir = TempDir::new().unwrap();
    create_test_file(dir.path(), "app/meta.ts", "export const STYLES = ['.a { color: red }'];");
    let result = transform(
        &dir,
        r#"import { Component } from '@angular/core';
import { STYLES } from './meta';
@Component({ selector: 'x', template: '', styles: STYLES })
export class X {}
"#,
        &resolve_options(),
    );
    assert!(!result.has_errors(), "{}", error_messages(&result));
    assert!(
        define_call(&result.code, "X").contains(r#"styles:[".a[_ngcontent-%COMP%]{color:red}"]"#),
        "{}",
        define_call(&result.code, "X")
    );
}

/// ngtsc keeps `EnumValue` identity across module resolution: an imported
/// enum member in `styles` is the same `Value is of type 'E'` error a
/// same-file one reports, not a string silently unwrapped to its value.
#[test]
fn imported_enum_member_is_not_a_string() {
    let dir = TempDir::new().unwrap();
    create_test_file(dir.path(), "app/meta.ts", "export enum E { A = '.a { color: red }' }");
    let result = transform(
        &dir,
        r#"import { Component } from '@angular/core';
import { E } from './meta';
@Component({ selector: 'x', template: '', styles: [E.A] })
export class X {}
"#,
        &resolve_options(),
    );
    assert!(result.has_errors());
    assert!(
        error_messages(&result)
            .contains("Failed to resolve styles at position 0 to a string Value is of type 'E'."),
        "{}",
        error_messages(&result)
    );
}

/// `host` goes through the same evaluator, so an imported object or string resolves
/// in a spread, as the whole host and as an entry's value.
#[test]
fn imported_host_metadata_evaluates() {
    let dir = TempDir::new().unwrap();
    create_test_file(
        dir.path(),
        "app/meta.ts",
        "export const SHARED = { '(press)': 'go()' };\nexport const NAME = 'button';",
    );
    let listener =
        r#"i0.ɵɵlistener("press",functionA_press_HostBindingHandler(){returnctx.go();})"#;
    for (host, attrs) in
        [("{ ...SHARED, role: NAME }", Some(r#"hostAttrs:["role","button"]"#)), ("SHARED", None)]
    {
        let source = format!(
            "import {{ Directive }} from '@angular/core';
import {{ SHARED, NAME }} from './meta';
@Directive({{ selector: '[a]', host: {host} }})
export class A {{ go() {{}} }}
"
        );
        let result = transform(&dir, &source, &resolve_options());
        assert!(!result.has_errors(), "{}", error_messages(&result));
        let compact: String = result.code.chars().filter(|c| !c.is_whitespace()).collect();
        assert!(compact.contains(listener), "`host: {host}`:\n{compact}");
        if let Some(attrs) = attrs {
            assert!(compact.contains(attrs), "`host: {host}`:\n{compact}");
        }
    }
}

/// Without the option the import is not read, and the host is reported, not dropped.
#[test]
fn imported_host_metadata_without_the_option_is_reported() {
    let dir = TempDir::new().unwrap();
    create_test_file(dir.path(), "app/meta.ts", "export const SHARED = { '(press)': 'go()' };");
    let result = transform(
        &dir,
        r#"import { Directive } from '@angular/core';
import { SHARED } from './meta';
@Directive({ selector: '[a]', host: { ...SHARED } })
export class A { go() {} }
"#,
        &TransformOptions::default(),
    );
    assert!(
        error_messages(&result).contains(
            "@Directive.host depends on 'SHARED', which is imported from another module."
        ),
        "{}",
        error_messages(&result)
    );

    // A namespace spread can't even be named: `import * as ns` has no one
    // binding to point at, so it reports like any unresolvable host.
    let result = transform(
        &dir,
        r#"import { Directive } from '@angular/core';
import * as meta from './meta';
@Directive({ selector: '[a]', host: { ...meta } })
export class A { go() {} }
"#,
        &TransformOptions::default(),
    );
    assert!(
        error_messages(&result)
            .contains("Decorator host metadata must be an object Value could not be determined"),
        "{}",
        error_messages(&result)
    );
}

/// `import * as ns` spreads all of the module's exports into the host map,
/// like ngtsc's `ResolvedModule.getExports()` — direct exports and ones a
/// star export forwards. An export whose value can't be read is an error on
/// its entry, not a dropped key.
#[test]
fn namespace_import_spreads_into_host() {
    let dir = TempDir::new().unwrap();
    create_test_file(dir.path(), "app/impl.ts", "export const VIA = 'v';");
    create_test_file(dir.path(), "app/barrel.ts", "export * from './impl';");
    create_test_file(
        dir.path(),
        "app/meta.ts",
        "export const ROLE = 'button';\nexport const TITLE = 't';",
    );
    for (source, want) in [
        (
            r#"import { Directive } from '@angular/core';
import * as meta from './meta';
@Directive({ selector: '[a]', host: { ...meta } })
export class A { go() {} }
"#,
            [r#""ROLE","button""#, r#""TITLE","t""#].as_slice(),
        ),
        (
            r#"import { Directive } from '@angular/core';
import * as barrel from './barrel';
@Directive({ selector: '[a]', host: { ...barrel } })
export class A { go() {} }
"#,
            [r#""VIA","v""#].as_slice(),
        ),
    ] {
        let result = transform(&dir, source, &resolve_options());
        assert!(!result.has_errors(), "{}", error_messages(&result));
        let compact: String = result.code.chars().filter(|c| !c.is_whitespace()).collect();
        for needle in want {
            assert!(compact.contains(needle), "missing `{needle}` in:\n{compact}");
        }
    }

    // An export that can't be read statically reports the entry: upstream
    // emits its expression for a plain attribute, which is written in the
    // other file and can't be emitted in this one (a declared divergence).
    create_test_file(
        dir.path(),
        "app/bad.ts",
        "declare function make(): string;\nexport const F = make();",
    );
    let result = transform(
        &dir,
        r#"import { Directive } from '@angular/core';
import * as bad from './bad';
@Directive({ selector: '[a]', host: { ...bad } })
export class A { go() {} }
"#,
        &resolve_options(),
    );
    assert!(
        error_messages(&result).contains("value of 'F' could not be determined statically"),
        "{}",
        error_messages(&result)
    );
}

/// Where a same-file enum member acts as its value (a string operand of
/// `+`), an imported one does too.
#[test]
fn imported_enum_member_in_concatenation() {
    let dir = TempDir::new().unwrap();
    create_test_file(dir.path(), "app/meta.ts", "export enum E { A = '.a' }");
    let result = transform(
        &dir,
        r#"import { Component } from '@angular/core';
import { E } from './meta';
@Component({ selector: 'x', template: '', styles: [E.A + ' { color: red }'] })
export class X {}
"#,
        &resolve_options(),
    );
    assert!(!result.has_errors(), "{}", error_messages(&result));
    assert!(
        define_call(&result.code, "X").contains(r#"styles:[".a[_ngcontent-%COMP%]{color:red}"]"#),
        "{}",
        define_call(&result.code, "X")
    );
}

/// A host attribute computed from an import ngtsc can't evaluate either is
/// emitted as written (ngtsc_spec.ts: "should accept dynamic host attribute
/// bindings").
#[test]
fn dynamic_host_attribute_from_an_unreadable_import() {
    let dir = TempDir::new().unwrap();
    create_test_file(dir.path(), "app/other.d.ts", "export declare const foo: any;");
    let result = transform(
        &dir,
        r#"import { Component } from '@angular/core';
import { foo } from './other';
const test = foo.bar();
@Component({ selector: 'test', template: '', host: { 'test': test } })
export class TestCmp {}
"#,
        &resolve_options(),
    );
    assert!(!result.has_errors(), "{}", error_messages(&result));
    let definition = define_call(&result.code, "TestCmp");
    assert!(definition.contains(r#"hostAttrs:["test",test]"#), "{definition}");
}
