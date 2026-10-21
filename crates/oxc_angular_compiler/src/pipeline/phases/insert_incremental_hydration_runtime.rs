//! Incremental hydration runtime phase.
//!
//! Inserts an `EnableIncrementalHydrationRuntime` op before the first `@defer`
//! block with hydrate triggers in each view, so the view emits
//! `ɵɵenableIncrementalHydrationRuntime()` once. Without that call the runtime
//! never hydrates `hydrate` blocks.
//!
//! Ported from Angular's `template/pipeline/src/phases/insert_incremental_hydration_runtime.ts`.

use crate::ir::enums::TDeferDetailsFlags;
use crate::ir::ops::{CreateOp, CreateOpBase, EnableIncrementalHydrationRuntimeOp};
use crate::pipeline::compilation::ComponentCompilationJob;

/// Inserts `EnableIncrementalHydrationRuntime` before the first hydrating `Defer` op of each view.
pub fn insert_incremental_hydration_runtime(job: &mut ComponentCompilationJob<'_>) {
    if !job.supports_incremental_hydration_runtime() {
        return;
    }

    for view in job.all_views_mut() {
        let mut cursor = view.create.cursor_front();
        loop {
            // The new op takes the defer block's source span, as in Angular.
            let hydrating_defer_span = match cursor.current() {
                Some(CreateOp::Defer(defer))
                    if defer.flags.is_some_and(|flags| {
                        flags & TDeferDetailsFlags::HAS_HYDRATE_TRIGGERS.bits() != 0
                    }) =>
                {
                    Some(defer.base.source_span)
                }
                _ => None,
            };
            if let Some(source_span) = hydrating_defer_span {
                cursor.insert_before(CreateOp::EnableIncrementalHydrationRuntime(
                    EnableIncrementalHydrationRuntimeOp {
                        base: CreateOpBase { source_span, ..Default::default() },
                    },
                ));
                break;
            }
            if !cursor.move_next() {
                break;
            }
        }
    }
}
