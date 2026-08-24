use ac_runtime::Function;
use ac_runtime_modules::Module;
use ac_runtime_values::NonFunctionValue;

use super::ops::{End, From, IsEmpty, Len, New, Peek, Pop, Push, ToArray};

static NEW: New = New;
static FROM: From = From;
static TO_ARRAY: ToArray = ToArray;
static LEN: Len = Len;
static IS_EMPTY: IsEmpty = IsEmpty;

static VEC_PUSH: Push = Push { end: End::Back };
static VEC_POP: Pop = Pop { end: End::Back };
static VEC_PEEK: Peek = Peek { end: End::Back };

static LIST_PUSH: Push = Push { end: End::Front };
static LIST_POP: Pop = Pop { end: End::Front };
static LIST_PEEK: Peek = Peek { end: End::Front };

static QUEUE_ENQUEUE: Push = Push { end: End::Back };
static QUEUE_DEQUEUE: Pop = Pop { end: End::Front };
static QUEUE_PEEK: Peek = Peek { end: End::Front };

static STACK_PUSH: Push = Push { end: End::Back };
static STACK_POP: Pop = Pop { end: End::Back };
static STACK_PEEK: Peek = Peek { end: End::Back };

static DEQUE_PUSH_BACK: Push = Push { end: End::Back };
static DEQUE_PUSH_FRONT: Push = Push { end: End::Front };
static DEQUE_POP_BACK: Pop = Pop { end: End::Back };
static DEQUE_POP_FRONT: Pop = Pop { end: End::Front };
static DEQUE_PEEK_BACK: Peek = Peek { end: End::Back };
static DEQUE_PEEK_FRONT: Peek = Peek { end: End::Front };

/// The `collections` module — collection types for the AC runtime.
///
/// Currently implemented: `vec`, `list`, `queue`, `stack`, `deque`.
#[derive(Clone, Copy)]
pub struct CollectionsModule;

impl Module for CollectionsModule {
    fn name(&self) -> &str {
        "collections"
    }

    fn function(&self, name: &str) -> Option<&dyn Function<NonFunctionValue>> {
        match name {
            // vec
            "vec.new" => Some(&NEW),
            "vec.from" => Some(&FROM),
            "vec.to_array" => Some(&TO_ARRAY),
            "vec.len" => Some(&LEN),
            "vec.is_empty" => Some(&IS_EMPTY),
            "vec.push" => Some(&VEC_PUSH),
            "vec.pop" => Some(&VEC_POP),
            "vec.peek" => Some(&VEC_PEEK),
            // list
            "list.new" => Some(&NEW),
            "list.from" => Some(&FROM),
            "list.to_array" => Some(&TO_ARRAY),
            "list.len" => Some(&LEN),
            "list.is_empty" => Some(&IS_EMPTY),
            "list.push" => Some(&LIST_PUSH),
            "list.pop" => Some(&LIST_POP),
            "list.peek" => Some(&LIST_PEEK),
            // queue
            "queue.new" => Some(&NEW),
            "queue.from" => Some(&FROM),
            "queue.to_array" => Some(&TO_ARRAY),
            "queue.len" => Some(&LEN),
            "queue.is_empty" => Some(&IS_EMPTY),
            "queue.enqueue" => Some(&QUEUE_ENQUEUE),
            "queue.dequeue" => Some(&QUEUE_DEQUEUE),
            "queue.peek" => Some(&QUEUE_PEEK),
            // stack
            "stack.new" => Some(&NEW),
            "stack.from" => Some(&FROM),
            "stack.to_array" => Some(&TO_ARRAY),
            "stack.len" => Some(&LEN),
            "stack.is_empty" => Some(&IS_EMPTY),
            "stack.push" => Some(&STACK_PUSH),
            "stack.pop" => Some(&STACK_POP),
            "stack.peek" => Some(&STACK_PEEK),
            // deque
            "deque.new" => Some(&NEW),
            "deque.from" => Some(&FROM),
            "deque.to_array" => Some(&TO_ARRAY),
            "deque.len" => Some(&LEN),
            "deque.is_empty" => Some(&IS_EMPTY),
            "deque.push_back" => Some(&DEQUE_PUSH_BACK),
            "deque.push_front" => Some(&DEQUE_PUSH_FRONT),
            "deque.pop_back" => Some(&DEQUE_POP_BACK),
            "deque.pop_front" => Some(&DEQUE_POP_FRONT),
            "deque.peek_back" => Some(&DEQUE_PEEK_BACK),
            "deque.peek_front" => Some(&DEQUE_PEEK_FRONT),
            _ => None,
        }
    }
}
