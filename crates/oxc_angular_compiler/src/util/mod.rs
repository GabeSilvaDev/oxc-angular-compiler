//! Shared utilities for the Angular compiler.

pub mod chars;
mod deferred_time;
pub(crate) mod lone_surrogate;
mod parse_util;
mod type_extract;

pub use deferred_time::*;
pub use parse_util::*;
pub use type_extract::*;

/// Whether a property of a decorator's options object is read as metadata.
/// Like ngtsc's `reflectObjectLiteral`, methods and accessors are not.
pub fn is_metadata_property(prop: &oxc_ast::ast::ObjectProperty<'_>) -> bool {
    !prop.method && prop.kind == oxc_ast::ast::PropertyKind::Init
}

/// The class's constructor implementation: the one with a body. Overload
/// signatures (no body) are skipped, since the implementation is what runs and
/// what carries the parameter decorators.
///
/// See: packages/compiler-cli/src/ngtsc/reflection/src/typescript.ts
/// (`TypeScriptReflectionHost.getConstructorParameters`)
pub fn find_constructor<'c, 'a>(
    class: &'c oxc_ast::ast::Class<'a>,
) -> Option<&'c oxc_ast::ast::MethodDefinition<'a>> {
    class.body.body.iter().find_map(|element| match element {
        oxc_ast::ast::ClassElement::MethodDefinition(method)
            if method.kind.is_constructor() && method.value.body.is_some() =>
        {
            Some(&**method)
        }
        _ => None,
    })
}
