//! The project-wide `preserveWhitespaces` option (`angularCompilerOptions`) is
//! the default for components that do not set their own, as in ngtsc's
//! `parseTemplateDeclaration` (annotations/component/src/resources.ts).
//!
//! Ported from Angular's `ngtsc_spec.ts` `preserveWhitespaces` cases.

use oxc_allocator::Allocator;
use oxc_angular_compiler::{CompilationMode, TransformOptions, transform_angular_file};

fn compile(decorator_option: &str, options: &TransformOptions) -> String {
    let source = format!(
        "import {{Component}} from '@angular/core';
@Component({{
  selector: 'test',{decorator_option}
  template: `
    <div>
      Template with whitespaces
    </div>
  `
}})
export class FooCmp {{}}
"
    );
    let allocator = Allocator::default();
    let result = transform_angular_file(&allocator, "test.ts", &source, Some(options), None);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    result.code
}

const COLLAPSED: &str = r#"ɵɵtext(1," Template with whitespaces ")"#;
const PRESERVED: &str = r#"ɵɵtext(2,"\n      Template with whitespaces\n    ")"#;

fn squash(code: &str) -> String {
    code.replace(", ", ",")
}

#[test]
fn defaults_to_collapsing_whitespace() {
    let code = compile("", &TransformOptions::default());
    assert!(squash(&code).contains(COLLAPSED), "{code}");
}

#[test]
fn component_false_collapses_whitespace() {
    let code = compile("\n  preserveWhitespaces: false,", &TransformOptions::default());
    assert!(squash(&code).contains(COLLAPSED), "{code}");
}

#[test]
fn takes_the_preserve_whitespaces_option_into_account() {
    let options = TransformOptions { preserve_whitespaces: Some(true), ..Default::default() };
    let code = compile("", &options);
    assert!(squash(&code).contains(PRESERVED), "{code}");
}

#[test]
fn component_preserve_whitespaces_overrides_the_option() {
    let options = TransformOptions { preserve_whitespaces: Some(true), ..Default::default() };
    let code = compile("\n  preserveWhitespaces: false,", &options);
    assert!(squash(&code).contains(COLLAPSED), "{code}");

    let options = TransformOptions { preserve_whitespaces: Some(false), ..Default::default() };
    let code = compile("\n  preserveWhitespaces: true,", &options);
    assert!(squash(&code).contains(PRESERVED), "{code}");
}

#[test]
fn partial_declaration_carries_the_resolved_option() {
    let options = TransformOptions {
        preserve_whitespaces: Some(true),
        compilation_mode: CompilationMode::Partial,
        ..Default::default()
    };
    let code = compile("", &options);
    assert!(code.contains("preserveWhitespaces:true"), "{code}");

    let code = compile("\n  preserveWhitespaces: false,", &options);
    assert!(!code.contains("preserveWhitespaces:true"), "{code}");
}
