use ac_vm::Context;

/// Sets the maximum scope depth on a context.
pub fn with_max_depth<C: Context>(ctx: &mut C, max: usize) -> &mut C {
    ctx.set_max_depth(max);
    ctx
}

/// Sets the maximum call count on a context.
pub fn with_max_calls<C: Context>(ctx: &mut C, max: usize) -> &mut C {
    ctx.set_max_calls(max);
    ctx
}

/// Sets both resource limits on a context.
pub fn with_limits<C: Context>(
    ctx: &mut C,
    max_depth: Option<usize>,
    max_calls: Option<usize>,
) -> &mut C {
    if let Some(max) = max_depth {
        ctx.set_max_depth(max);
    }
    if let Some(max) = max_calls {
        ctx.set_max_calls(max);
    }
    ctx
}
