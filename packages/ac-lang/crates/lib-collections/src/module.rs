use ac_runtime::Function;
use ac_runtime_modules::Module;
use ac_runtime_values::NonFunctionValue;

use super::hash_map as hm;
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

static HASH_MAP_NEW: hm::New = hm::New;
static HASH_MAP_FROM: hm::From = hm::From;
static HASH_MAP_TO_OBJECT: hm::ToObject = hm::ToObject;
static HASH_MAP_LEN: hm::Len = hm::Len;
static HASH_MAP_IS_EMPTY: hm::IsEmpty = hm::IsEmpty;
static HASH_MAP_GET: hm::Get = hm::Get;
static HASH_MAP_SET: hm::Set = hm::Set;
static HASH_MAP_REMOVE: hm::Remove = hm::Remove;
static HASH_MAP_HAS: hm::Has = hm::Has;
static HASH_MAP_KEYS: hm::Keys = hm::Keys;
static HASH_MAP_VALUES: hm::Values = hm::Values;

/// The `collections` module — collection types for the AC runtime.
///
/// Currently implemented: `vec`, `list`, `queue`, `stack`, `deque`, `hash_map`.
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
            // hash_map
            "hash_map.new" => Some(&HASH_MAP_NEW),
            "hash_map.from" => Some(&HASH_MAP_FROM),
            "hash_map.to_object" => Some(&HASH_MAP_TO_OBJECT),
            "hash_map.len" => Some(&HASH_MAP_LEN),
            "hash_map.is_empty" => Some(&HASH_MAP_IS_EMPTY),
            "hash_map.get" => Some(&HASH_MAP_GET),
            "hash_map.set" => Some(&HASH_MAP_SET),
            "hash_map.remove" => Some(&HASH_MAP_REMOVE),
            "hash_map.has" => Some(&HASH_MAP_HAS),
            "hash_map.keys" => Some(&HASH_MAP_KEYS),
            "hash_map.values" => Some(&HASH_MAP_VALUES),
            _ => None,
        }
    }
}
