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

// ---------------------------------------------------------------------------
// Selector matching (`pipeline/selector.rs`, `component/defer_resolver.rs`):
// elements are matched the way ngc's `createCssSelectorFromNode` and
// `SelectorMatcher` match them.
// ---------------------------------------------------------------------------

/// Writes `app/dep.ts`, a standalone directive `Dep` with `selector`.
fn selector_matching_dep(dir: &TempDir, selector: &str) {
    create_test_file(
        dir.path(),
        "app/dep.ts",
        &format!(
            r#"
import {{ Directive }} from '@angular/core';

@Directive({{ selector: '{selector}' }})
export class Dep {{}}
"#
        ),
    );
}

/// Compiles a parent component that imports `Dep` and uses `template`.
fn selector_matching_transform(dir: &TempDir, template: &str) -> TransformResult {
    transform(
        dir,
        &format!(
            r#"
import {{ Component }} from '@angular/core';
import {{ Dep }} from './dep';

@Component({{
    selector: 'app-parent',
    imports: [Dep],
    template: `{template}`,
}})
export class Parent {{ x = ''; }}
"#
        ),
    )
}

#[test]
fn selector_matching_svg_element_used_eagerly_stays_eager() {
    let dir = TempDir::new().unwrap();
    selector_matching_dep(&dir, "svg[appIcon], i[appIcon]");
    let result =
        selector_matching_transform(&dir, "<svg appIcon></svg> @defer { <i appIcon></i> }");
    assert_eager(&result.code, "./dep");
}

#[test]
fn selector_matching_element_inside_svg_used_eagerly_stays_eager() {
    let dir = TempDir::new().unwrap();
    selector_matching_dep(&dir, "a[appLink]");
    let result =
        selector_matching_transform(&dir, "<svg><a appLink></a></svg> @defer { <a appLink></a> }");
    assert_eager(&result.code, "./dep");
}

#[test]
fn selector_matching_element_inside_svg_used_only_inside_defer_is_lazy_loaded() {
    let dir = TempDir::new().unwrap();
    selector_matching_dep(&dir, "g[appDep]");
    let result = selector_matching_transform(&dir, "@defer { <svg><g appDep></g></svg> }");
    assert_lazy(&result.code, "./dep", "Dep");
}

#[test]
fn selector_matching_namespaced_attribute_used_eagerly_stays_eager() {
    let dir = TempDir::new().unwrap();
    selector_matching_dep(&dir, "[href]");
    let result = selector_matching_transform(
        &dir,
        r#"<svg><a xlink:href="x"></a></svg> @defer { <a href="x"></a> }"#,
    );
    assert_eager(&result.code, "./dep");
}

#[test]
fn selector_matching_only_i18n_attributes_are_skipped() {
    let dir = TempDir::new().unwrap();
    selector_matching_dep(&dir, "[i18nTip]");
    let result =
        selector_matching_transform(&dir, r#"<b i18nTip></b> @defer { <b [i18nTip]="x"></b> }"#);
    assert_eager(&result.code, "./dep");
}

#[test]
fn selector_matching_class_attribute_name_is_case_insensitive() {
    let dir = TempDir::new().unwrap();
    selector_matching_dep(&dir, ".foo");
    let result =
        selector_matching_transform(&dir, r#"<b CLASS="foo"></b> @defer { <b class="foo"></b> }"#);
    assert_eager(&result.code, "./dep");
}

#[test]
fn selector_matching_not_only_selector_matches_any_element() {
    let dir = TempDir::new().unwrap();
    selector_matching_dep(&dir, ":not(.plain)");
    let result = selector_matching_transform(&dir, "@defer { <span></span> }");
    assert_lazy(&result.code, "./dep", "Dep");

    let dir = TempDir::new().unwrap();
    selector_matching_dep(&dir, "[appDep], :not([appDep]):not(span)");
    let result = selector_matching_transform(&dir, "<div></div> @defer { <span appDep></span> }");
    assert_eager(&result.code, "./dep");
}

#[test]
fn selector_matching_empty_selector_in_a_list_matches_nothing() {
    let dir = TempDir::new().unwrap();
    selector_matching_dep(&dir, "[appDep],");
    let result = selector_matching_transform(&dir, "<div></div> @defer { <span appDep></span> }");
    assert_lazy(&result.code, "./dep", "Dep");
}

#[test]
fn selector_matching_binding_overrides_static_attribute_value() {
    let dir = TempDir::new().unwrap();
    selector_matching_dep(&dir, "[type=a]");
    let result = selector_matching_transform(
        &dir,
        r#"<input type="a" [type]="x"/> @defer { <input type="a"/> }"#,
    );
    assert_lazy(&result.code, "./dep", "Dep");

    let dir = TempDir::new().unwrap();
    selector_matching_dep(&dir, ".foo");
    let result = selector_matching_transform(
        &dir,
        r#"<div class="foo" [class]="x"></div> @defer { <div class="foo"></div> }"#,
    );
    assert_lazy(&result.code, "./dep", "Dep");
}

#[test]
fn selector_matching_placeholder_of_a_nested_defer_is_lazy_loaded() {
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
    template: '@defer { @defer { <p>x</p> } @placeholder { <app-lazy/> } }',
})
export class Parent {}
"#,
    );
    assert_lazy(&result.code, "./lazy", "LazyCmp");
}
