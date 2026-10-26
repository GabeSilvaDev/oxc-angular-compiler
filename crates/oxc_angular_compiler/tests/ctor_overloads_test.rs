//! With constructor overloads, the implementation (the declaration with a body)
//! is what runs and what carries the parameter decorators, so like ngtsc's
//! `TypeScriptReflectionHost.getConstructorParameters` the factory,
//! `setClassMetadata` and JIT `ctorParameters` read it, not the first overload.
//!
//! Ported from Angular's `ngtsc_spec.ts`: "should compile @Injectable with
//! constructor overloads".

use oxc_allocator::Allocator;
use oxc_angular_compiler::{TransformOptions, transform_angular_file};

fn transform(source: &str, options: &TransformOptions) -> String {
    let allocator = Allocator::default();
    let result = transform_angular_file(&allocator, "test.ts", source, Some(options), None);
    assert!(result.diagnostics.is_empty(), "unexpected diagnostics: {:?}", result.diagnostics);
    result.code.chars().filter(|c| !c.is_whitespace()).collect()
}

fn source(decorator: &str, extra_members: &str) -> String {
    format!(
        r"
import {{Component, Directive, Injectable, NgModule, Optional, Pipe}} from '@angular/core';

@Injectable()
class Dep {{}}

@Injectable()
class OptionalDep {{}}

{decorator}
class Service {{
  constructor(dep: Dep);

  constructor(dep: Dep, @Optional() optionalDep?: OptionalDep) {{}}
  {extra_members}
}}
"
    )
}

#[test]
fn injectable_factory_reads_the_implementation() {
    let code = transform(&source("@Injectable()", ""), &TransformOptions::default());
    assert!(
        code.contains(
            "returnnew(__ngFactoryType__||Service)(i0.ɵɵinject(Dep),i0.ɵɵinject(OptionalDep,8));"
        ),
        "{code}"
    );
}

#[test]
fn every_decorator_kind_reads_the_implementation() {
    let cases = [
        ("@Component({selector: 'c', template: ''})", "", "i0.ɵɵdirectiveInject(OptionalDep,8)"),
        ("@Directive({selector: '[d]'})", "", "i0.ɵɵdirectiveInject(OptionalDep,8)"),
        ("@Pipe({name: 'p'})", "transform() {}", "i0.ɵɵdirectiveInject(OptionalDep,24)"),
        ("@NgModule({})", "", "i0.ɵɵinject(OptionalDep,8)"),
        ("@Injectable()", "", "i0.ɵɵinject(OptionalDep,8)"),
    ];
    for (decorator, extra, optional_dep) in cases {
        let code = transform(&source(decorator, extra), &TransformOptions::default());
        assert!(code.contains(&format!(",{optional_dep});")), "{decorator}: {code}");
        // setClassMetadata's ctor params come from the implementation too.
        assert!(
            code.contains("()=>[{type:Dep},{type:OptionalDep,decorators:[{type:Optional}]}]"),
            "{decorator}: {code}"
        );
        // The implementation's parameter decorator is compiled, so it is removed.
        assert!(
            code.contains("constructor(dep:Dep,optionalDep?:OptionalDep){}"),
            "{decorator}: {code}"
        );
    }
}

#[test]
fn jit_ctor_parameters_read_the_implementation() {
    let options = TransformOptions { jit: true, ..TransformOptions::default() };
    let code = transform(&source("@Injectable()", ""), &options);
    assert!(
        code.contains(
            "staticctorParameters=()=>[{type:Dep},{type:OptionalDep,decorators:[{type:Optional}]}];"
        ),
        "{code}"
    );
    // ...and its Angular parameter decorator is removed, not lowered to `__param`.
    assert!(!code.contains("Optional()"), "{code}");
}
