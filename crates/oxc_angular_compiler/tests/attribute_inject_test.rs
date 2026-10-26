//! `@Attribute('name')` on a constructor parameter compiles to
//! `ɵɵinjectAttribute('name')` whatever the class kind, as ngtsc does
//! (`compileInjectDependency` in `r3_factory.ts` checks `attributeNameType`
//! before the target). A plain token inject makes Angular look the attribute
//! name up as a DI token at runtime.

use oxc_allocator::Allocator;
use oxc_angular_compiler::{TransformOptions, transform_angular_file};

/// The compiled code for `source`, without whitespace and with `'` as `"`.
fn compile(source: &str) -> String {
    let allocator = Allocator::default();
    let result = transform_angular_file(
        &allocator,
        "test.ts",
        source,
        Some(&TransformOptions::default()),
        None,
    );
    assert!(!result.has_errors(), "Should not have errors: {:?}", result.diagnostics);
    result
        .code
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| if c == '\'' { '"' } else { c })
        .collect()
}

fn assert_injects_attribute(code: &str) {
    assert!(code.contains(r#"(i0.ɵɵinjectAttribute("title"))"#), "Got:\n{code}");
}

#[test]
fn directive_factory_injects_the_attribute() {
    let code = compile(
        r"
import { Attribute, Directive } from '@angular/core';
@Directive({ selector: '[d]' })
export class D {
  constructor(@Attribute('title') title: string) {}
}
",
    );
    assert_injects_attribute(&code);
}

#[test]
fn pipe_factory_injects_the_attribute() {
    let code = compile(
        r"
import { Attribute, Pipe } from '@angular/core';
@Pipe({ name: 'p' })
export class P {
  constructor(@Attribute('title') title: string) {}
  transform(v: unknown) { return v; }
}
",
    );
    assert_injects_attribute(&code);
}

#[test]
fn injectable_factory_injects_the_attribute() {
    let code = compile(
        r"
import { Attribute, Injectable } from '@angular/core';
@Injectable()
export class S {
  constructor(@Attribute('title') title: string) {}
}
",
    );
    assert_injects_attribute(&code);
}

#[test]
fn ng_module_factory_injects_the_attribute() {
    let code = compile(
        r"
import { Attribute, NgModule } from '@angular/core';
@NgModule({})
export class M {
  constructor(@Attribute('title') title: string) {}
}
",
    );
    assert_injects_attribute(&code);
}

/// The `.d.ts` factory type names the attribute, as ngtsc's `createCtorDepType`
/// writes `attributeNameType`: `[{ attribute: "title"; optional: true; }]`.
#[test]
fn factory_declaration_names_the_attribute() {
    let source = r#"
import { Attribute, Directive, Injectable, NgModule, Optional, Pipe } from '@angular/core';
@Directive({ selector: '[d]' })
export class D { constructor(@Optional() @Attribute('title') t: string) {} }
@Pipe({ name: 'p' })
export class P { constructor(@Attribute('say "hi"') t: string) {} transform(v: unknown) { return v; } }
@Injectable()
export class S { constructor(@Attribute('title') t: string) {} }
@NgModule({})
export class M { constructor(@Attribute('title') t: string) {} }
"#;
    let allocator = Allocator::default();
    let result = transform_angular_file(
        &allocator,
        "test.ts",
        source,
        Some(&TransformOptions::default()),
        None,
    );
    let factory_type = |class: &str| {
        let decl = result.dts_declarations.iter().find(|d| d.class_name == class).unwrap();
        let line = decl.members.lines().find(|l| l.contains("static ɵfac")).unwrap();
        line.trim().to_string()
    };
    assert!(
        factory_type("D").contains(r#"[{ attribute: "title", optional: true }]"#),
        "{}",
        factory_type("D")
    );
    assert!(
        factory_type("P").contains(r#"[{ attribute: "say \"hi\"" }]"#),
        "{}",
        factory_type("P")
    );
    for class in ["S", "M"] {
        assert!(
            factory_type(class).contains(r#"[{ attribute: "title" }]"#),
            "{}",
            factory_type(class)
        );
    }
}
