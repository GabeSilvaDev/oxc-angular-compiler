//! Type extraction helpers shared between DI token resolution and class metadata.
//!
//! Mirrors Angular's `typeReferenceToExpression` in
//! `packages/compiler-cli/src/ngtsc/reflection/src/typescript.ts`, which filters
//! `null` literal type nodes out of a union before resolving the remaining type
//! as an injection token.

use oxc_allocator::{Allocator, Box};
use oxc_ast::ast::{TSType, TSTypeName};
use oxc_str::Ident;

use crate::output::ast::{OutputExpression, ReadPropExpr, ReadVarExpr};

/// Resolves a TypeScript type annotation to the concrete type usable as a DI
/// injection token.
///
/// When the annotation is a union, `null` keyword variants are filtered out;
/// if exactly one non-null type remains, that type is returned. Otherwise
/// (no remaining types, or two or more remaining types) the result is `None`,
/// matching the reference compiler's "unresolvable token" behavior.
///
/// Parenthesized type nodes (`(T)`) are transparently unwrapped before union
/// handling — oxc preserves them in the AST and the reference compiler treats
/// them as the inner type for token resolution.
///
/// `undefined` is intentionally **not** stripped — this matches Angular's
/// reference compiler, where `T | undefined` remains ambiguous for token
/// resolution while `T | null` is the canonical optional-DI pattern.
///
/// Non-union, non-parenthesized types are returned as-is.
pub fn resolve_di_token_type<'a, 'b>(ts_type: &'b TSType<'a>) -> Option<&'b TSType<'a>> {
    match ts_type {
        TSType::TSParenthesizedType(paren) => resolve_di_token_type(&paren.type_annotation),
        TSType::TSUnionType(union) => {
            let mut non_null = union.types.iter().filter(|t| {
                // Unwrap parens inside the union so `MyService | (null)` and
                // `(MyService) | null` both reduce to `MyService`.
                !matches!(unwrap_parens(t), TSType::TSNullKeyword(_))
            });
            let first = non_null.next()?;
            if non_null.next().is_some() {
                // More than one non-null type — ambiguous for token resolution.
                return None;
            }
            // Recurse so nested cases like `(MyService | null) | null` collapse.
            resolve_di_token_type(first)
        }
        _ => Some(ts_type),
    }
}

/// Strip nested `TSParenthesizedType` wrappers, returning the underlying type.
fn unwrap_parens<'a, 'b>(ts_type: &'b TSType<'a>) -> &'b TSType<'a> {
    let mut current = ts_type;
    while let TSType::TSParenthesizedType(paren) = current {
        current = &paren.type_annotation;
    }
    current
}

/// The value a DI token's type name refers to, as written: `Foo` → `Foo`,
/// `ns.one.One` → `ns.one.One`. `this` has no value: `None`.
///
/// Mirrors `entityNameToValue` in
/// `packages/compiler-cli/src/ngtsc/reflection/src/type_to_value.ts`.
pub fn type_name_to_value_expr<'a>(
    allocator: &'a Allocator,
    name: &TSTypeName<'a>,
) -> Option<OutputExpression<'a>> {
    match name {
        TSTypeName::IdentifierReference(id) => Some(OutputExpression::ReadVar(Box::new_in(
            ReadVarExpr { name: id.name, source_span: None },
            &allocator,
        ))),
        TSTypeName::QualifiedName(qualified) => {
            let receiver = type_name_to_value_expr(allocator, &qualified.left)?;
            Some(OutputExpression::ReadProp(Box::new_in(
                ReadPropExpr {
                    receiver: Box::new_in(receiver, &allocator),
                    name: qualified.right.name,
                    optional: false,
                    source_span: None,
                },
                &allocator,
            )))
        }
        TSTypeName::ThisExpression(_) => None,
    }
}

/// Fold `members` onto `base` as property reads: `base.a.b`.
pub fn read_member_path<'a>(
    allocator: &'a Allocator,
    base: OutputExpression<'a>,
    members: &[Ident<'a>],
) -> OutputExpression<'a> {
    members.iter().fold(base, |receiver, name| {
        OutputExpression::ReadProp(Box::new_in(
            ReadPropExpr {
                receiver: Box::new_in(receiver, &allocator),
                name: *name,
                optional: false,
                source_span: None,
            },
            &allocator,
        ))
    })
}
