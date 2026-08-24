use std::collections::VecDeque;

/// A first-in-first-out queue.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Queue<T>(VecDeque<T>);

impl<T> Queue<T> {
    /// Creates an empty queue.
    pub fn new() -> Self {
        Self(VecDeque::new())
    }

    /// Creates an empty queue with the given capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self(VecDeque::with_capacity(capacity))
    }

    /// Adds an element to the back of the queue.
    pub fn enqueue(&mut self, value: T) {
        self.0.push_back(value);
    }

    /// Removes and returns the front element.
    pub fn dequeue(&mut self) -> Option<T> {
        self.0.pop_front()
    }

    /// Returns a reference to the front element.
    pub fn peek(&self) -> Option<&T> {
        self.0.front()
    }

    /// Returns a reference to the back element.
    pub fn peek_back(&self) -> Option<&T> {
        self.0.back()
    }

    /// Returns the number of elements.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Clears the queue.
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Pushes to the front.
    pub fn push_front(&mut self, value: T) {
        self.0.push_front(value);
    }

    /// Pops from the back.
    pub fn pop_back(&mut self) -> Option<T> {
        self.0.pop_back()
    }

    /// Returns `true` if the queue contains the value.
    pub fn contains(&self, value: &T) -> bool
    where
        T: PartialEq,
    {
        self.0.contains(value)
    }

    /// Returns an iterator over the queue.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.0.iter()
    }

    /// Returns a vector of the values.
    pub fn to_vec(&self) -> Vec<T>
    where
        T: Clone,
    {
        self.0.iter().cloned().collect()
    }

    /// Reserves capacity for `additional` more elements.
    pub fn reserve(&mut self, additional: usize) {
        self.0.reserve(additional);
    }

    /// Shrinks the capacity to fit.
    pub fn shrink_to_fit(&mut self) {
        self.0.shrink_to_fit();
    }
}

impl<T> FromIterator<T> for Queue<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl<T> From<Vec<T>> for Queue<T> {
    fn from(vec: Vec<T>) -> Self {
        Self(vec.into())
    }
}

impl<T> From<Queue<T>> for Vec<T> {
    fn from(queue: Queue<T>) -> Self {
        queue.0.into()
    }
}

/// A double-ended queue.
pub type Deque<T> = Queue<T>;
