//! An `@Injectable({ useFactory | useClass, deps })` entry written as an array
//! (`[new Optional(), new Self(), Dep]`) is a token plus flags, as ngtsc's
//! `getDep` (`annotations/src/injectable.ts`) reads it: `Inject`, `Optional`,
//! `Self` and `SkipSelf` from `@angular/core`, `new X()` or bare `X`, set the
//! flags (or the token, for `new Inject(T)`); any other element is the token.

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

fn assert_contains(code: &str, expected: &str) {
    assert!(code.contains(expected), "Expected `{expected}` in:\n{code}");
}

/// ngtsc_spec.ts: "should compile Injectables with providedIn and factory with
/// deps with array literal tokens".
#[test]
fn use_factory_deps_with_array_literal_tokens() {
    let code = compile(
        r"
import {Injectable, Optional, Self} from '@angular/core';

@Injectable()
export class Dep {}

@Injectable({
  providedIn: 'root',
  useFactory: (dep: Dep) => new Service(dep),
  deps: [[new Optional(), new Self(), Dep]],
})
export class Service {
  constructor(dep: Dep) {}
}
",
    );
    assert_contains(&code, "returnnew(__ngFactoryType__||Service)(i0.ɵɵinject(Dep));");
    assert_contains(
        &code,
        "__ngConditionalFactory__=((dep)=>newService(dep))(i0.ɵɵinject(Dep,10))",
    );
}

#[test]
fn use_class_deps_with_bare_flag_identifiers() {
    let code = compile(
        r"
import {Injectable, Optional, SkipSelf} from '@angular/core';

@Injectable()
export class Dep {}

@Injectable()
export class Impl {
  constructor(dep: Dep) {}
}

@Injectable({
  providedIn: 'root',
  useClass: Impl,
  deps: [[Optional, SkipSelf, Dep]],
})
export class Service {}
",
    );
    assert_contains(&code, "__ngConditionalFactory__=newImpl(i0.ɵɵinject(Dep,12))");
}

#[test]
fn inject_entry_sets_the_token() {
    let code = compile(
        r"
import {Inject, Injectable, InjectionToken, Optional} from '@angular/core';

export const TOKEN = new InjectionToken<string>('TOKEN');

@Injectable({
  providedIn: 'root',
  useFactory: (a: string, b: string) => new Service(),
  deps: [[new Inject(TOKEN), new Optional()], TOKEN],
})
export class Service {}
",
    );
    assert_contains(&code, "(i0.ɵɵinject(TOKEN,8),i0.ɵɵinject(TOKEN))");
}

#[test]
fn flags_not_imported_from_angular_core_are_tokens() {
    let code = compile(
        r"
import {Injectable} from '@angular/core';
import {Optional} from './other';

@Injectable()
export class Dep {}

@Injectable({
  providedIn: 'root',
  useFactory: (dep: Dep) => new Service(dep),
  deps: [[Dep, new Optional()]],
})
export class Service {
  constructor(dep: Dep) {}
}
",
    );
    assert_contains(&code, "(i0.ɵɵinject(newOptional()))");
}
