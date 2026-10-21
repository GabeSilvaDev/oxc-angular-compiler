//! Implicit `@defer` dependencies (#313): a standalone component or directive
//! listed in `@Component.imports` and matched only inside `@defer` blocks is
//! lazy-loaded through a dynamic `import()`, the way ngc's full compilation
//! does, when cross-file analysis can read its selector
//! (`TransformOptions::resolve_imported_values`).

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

fn options() -> TransformOptions {
    TransformOptions { resolve_imported_values: true, ..Default::default() }
}

fn transform_with(dir: &TempDir, source: &str, options: &TransformOptions) -> TransformResult {
    let path = create_test_file(dir.path(), "app/parent.component.ts", source);
    let allocator = Allocator::default();
    let result =
        transform_angular_file(&allocator, path.to_str().unwrap(), source, Some(options), None);
    assert!(!result.has_errors(), "Should not have errors: {:?}", result.diagnostics);
    result
}

fn transform(dir: &TempDir, source: &str) -> TransformResult {
    transform_with(dir, source, &options())
}

fn compact(code: &str) -> String {
    code.chars().filter(|c| !c.is_whitespace()).collect()
}

fn lazy_cmp(dir: &TempDir) {
    create_test_file(
        dir.path(),
        "app/lazy.ts",
        r#"
import { Component } from '@angular/core';

@Component({ selector: 'app-lazy', template: 'lazy' })
export class LazyCmp {}
"#,
    );
}

fn assert_lazy(code: &str, module: &str, member: &str) {
    let compact = compact(code);
    assert!(
        compact.contains(&format!("import(\"{module}\").then((m)=>m.{member})"))
            || compact.contains(&format!("import(\"{module}\").then(m=>m.{member})")),
        "Expected `import('{module}').then(m => m.{member})`. Output:\n{code}"
    );
}

fn assert_eager(code: &str, module: &str) {
    assert!(
        !code.contains(&format!("import(\"{module}\")")),
        "`{module}` must stay an eager dependency. Output:\n{code}"
    );
}

#[test]
fn component_used_only_inside_defer_is_lazy_loaded() {
    let dir = TempDir::new().unwrap();
    lazy_cmp(&dir);
    let result = transform(
        &dir,
        r#"
import { Component } from '@angular/core';
import { LazyCmp } from './lazy';

@Component({
    selector: 'app-parent',
    imports: [LazyCmp],
    template: '@defer { <app-lazy/> }',
})
export class Parent {}
"#,
    );
    let code = &result.code;
    let compact = compact(code);

    assert_lazy(code, "./lazy", "LazyCmp");
    assert!(
        compact.contains("ɵɵdefer(1,0,()=>[import(\"./lazy\")"),
        "The resolver must be wired into `ɵɵdefer`. Output:\n{code}"
    );
    assert!(
        compact.contains("ɵɵgetComponentDepsFactory(Parent,[])"),
        "The lazy component must leave the eager dependencies. Output:\n{code}"
    );
    assert!(
        !code.contains("import { LazyCmp }"),
        "The static import of the lazy component must be removed. Output:\n{code}"
    );
    assert!(code.contains("setClassMetadataAsync"), "Output:\n{code}");
}

#[test]
fn component_also_used_outside_defer_stays_eager() {
    let dir = TempDir::new().unwrap();
    lazy_cmp(&dir);
    let result = transform(
        &dir,
        r#"
import { Component } from '@angular/core';
import { LazyCmp } from './lazy';

@Component({
    selector: 'app-parent',
    imports: [LazyCmp],
    template: '<app-lazy/> @defer { <app-lazy/> }',
})
export class Parent {}
"#,
    );
    let code = &result.code;
    assert_eager(code, "./lazy");
    assert!(compact(code).contains("ɵɵgetComponentDepsFactory(Parent,[LazyCmp])"), "{code}");
    assert!(code.contains("import { LazyCmp }"), "{code}");
}

#[test]
fn component_used_in_a_placeholder_stays_eager() {
    let dir = TempDir::new().unwrap();
    lazy_cmp(&dir);
    let result = transform(
        &dir,
        r#"
import { Component } from '@angular/core';
import { LazyCmp } from './lazy';

@Component({
    selector: 'app-parent',
    imports: [LazyCmp],
    template: '@defer { <app-lazy/> } @placeholder { <app-lazy/> }',
})
export class Parent {}
"#,
    );
    assert_eager(&result.code, "./lazy");
}

#[test]
fn component_used_in_nested_control_flow_inside_defer_is_lazy_loaded() {
    let dir = TempDir::new().unwrap();
    lazy_cmp(&dir);
    let result = transform(
        &dir,
        r#"
import { Component } from '@angular/core';
import { LazyCmp } from './lazy';

@Component({
    selector: 'app-parent',
    imports: [LazyCmp],
    template: '@defer { @switch (mode) { @case (1) { <div><app-lazy/></div> } } }',
})
export class Parent { mode = 1; }
"#,
    );
    assert_lazy(&result.code, "./lazy", "LazyCmp");
}

#[test]
fn aliased_import_reads_the_exported_name() {
    let dir = TempDir::new().unwrap();
    lazy_cmp(&dir);
    let result = transform(
        &dir,
        r#"
import { Component } from '@angular/core';
import { LazyCmp as Lazy } from './lazy';

@Component({
    selector: 'app-parent',
    imports: [Lazy],
    template: '@defer { <app-lazy/> }',
})
export class Parent {}
"#,
    );
    assert_lazy(&result.code, "./lazy", "LazyCmp");
}

#[test]
fn default_import_reads_default() {
    let dir = TempDir::new().unwrap();
    create_test_file(
        dir.path(),
        "app/lazy.ts",
        r#"
import { Component } from '@angular/core';

@Component({ selector: 'app-lazy', template: 'lazy' })
export default class LazyCmp {}
"#,
    );
    let result = transform(
        &dir,
        r#"
import { Component } from '@angular/core';
import LazyCmp from './lazy';

@Component({
    selector: 'app-parent',
    imports: [LazyCmp],
    template: '@defer { <app-lazy/> }',
})
export class Parent {}
"#,
    );
    assert_lazy(&result.code, "./lazy", "default");
}

#[test]
fn import_referenced_elsewhere_in_the_file_stays_eager() {
    let dir = TempDir::new().unwrap();
    lazy_cmp(&dir);
    let result = transform(
        &dir,
        r#"
import { Component } from '@angular/core';
import { LazyCmp } from './lazy';

export const EXPOSED = LazyCmp;

@Component({
    selector: 'app-parent',
    imports: [LazyCmp],
    template: '@defer { <app-lazy/> }',
})
export class Parent {}
"#,
    );
    assert_eager(&result.code, "./lazy");
}

#[test]
fn attribute_directive_used_only_inside_defer_is_lazy_loaded() {
    let dir = TempDir::new().unwrap();
    create_test_file(
        dir.path(),
        "app/tooltip.ts",
        r#"
import { Directive } from '@angular/core';

@Directive({ selector: '[appTooltip]' })
export class Tooltip {}
"#,
    );
    let result = transform(
        &dir,
        r#"
import { Component } from '@angular/core';
import { Tooltip } from './tooltip';

@Component({
    selector: 'app-parent',
    imports: [Tooltip],
    template: '<button>eager</button> @defer { <button appTooltip>lazy</button> }',
})
export class Parent {}
"#,
    );
    assert_lazy(&result.code, "./tooltip", "Tooltip");
}

#[test]
fn bound_input_matches_an_attribute_selector() {
    let dir = TempDir::new().unwrap();
    create_test_file(
        dir.path(),
        "app/tooltip.ts",
        r#"
import { Directive, input } from '@angular/core';

@Directive({ selector: '[appTooltip]' })
export class Tooltip { appTooltip = input(''); }
"#,
    );
    let result = transform(
        &dir,
        r#"
import { Component } from '@angular/core';
import { Tooltip } from './tooltip';

@Component({
    selector: 'app-parent',
    imports: [Tooltip],
    template: '<button [appTooltip]="text">eager</button> @defer { <span appTooltip></span> }',
})
export class Parent { text = ''; }
"#,
    );
    assert_eager(&result.code, "./tooltip");
}

#[test]
fn not_standalone_component_stays_eager() {
    let dir = TempDir::new().unwrap();
    create_test_file(
        dir.path(),
        "app/lazy.ts",
        r#"
import { Component } from '@angular/core';

@Component({ selector: 'app-lazy', template: 'lazy', standalone: false })
export class LazyCmp {}
"#,
    );
    let result = transform(
        &dir,
        r#"
import { Component } from '@angular/core';
import { LazyCmp } from './lazy';

@Component({
    selector: 'app-parent',
    imports: [LazyCmp],
    template: '@defer { <app-lazy/> }',
})
export class Parent {}
"#,
    );
    assert_eager(&result.code, "./lazy");
}

#[test]
fn pipe_stays_eager() {
    let dir = TempDir::new().unwrap();
    create_test_file(
        dir.path(),
        "app/shout.ts",
        r#"
import { Pipe } from '@angular/core';

@Pipe({ name: 'shout' })
export class ShoutPipe { transform(v: string) { return v; } }
"#,
    );
    let result = transform(
        &dir,
        r#"
import { Component } from '@angular/core';
import { ShoutPipe } from './shout';

@Component({
    selector: 'app-parent',
    imports: [ShoutPipe],
    template: '@defer { {{ name | shout }} }',
})
export class Parent { name = ''; }
"#,
    );
    assert_eager(&result.code, "./shout");
}

#[test]
fn implicit_and_explicit_deferred_imports_share_the_resolver() {
    let dir = TempDir::new().unwrap();
    lazy_cmp(&dir);
    let result = transform(
        &dir,
        r#"
import { Component } from '@angular/core';
import { LazyCmp } from './lazy';
import { Other } from './other';

@Component({
    selector: 'app-parent',
    imports: [LazyCmp],
    deferredImports: [Other],
    template: '@defer { <app-lazy/> <app-other/> }',
})
export class Parent {}
"#,
    );
    assert_lazy(&result.code, "./lazy", "LazyCmp");
    assert_lazy(&result.code, "./other", "Other");
}

#[test]
fn hmr_keeps_imports_eager() {
    let dir = TempDir::new().unwrap();
    lazy_cmp(&dir);
    let result = transform_with(
        &dir,
        r#"
import { Component } from '@angular/core';
import { LazyCmp } from './lazy';

@Component({
    selector: 'app-parent',
    imports: [LazyCmp],
    template: '@defer { <app-lazy/> }',
})
export class Parent {}
"#,
        &TransformOptions { hmr: true, ..options() },
    );
    assert_eager(&result.code, "./lazy");
}
