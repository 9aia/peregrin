/// A growable array.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Vector<T>(Vec<T>);

impl<T> Vector<T> {
    /// Creates an empty vector.
    pub fn new() -> Self {
        Self(Vec::new())
    }

    /// Creates an empty vector with the given capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity))
    }

    /// Appends an element to the end.
    pub fn push(&mut self, value: T) {
        self.0.push(value);
    }

    /// Removes and returns the last element.
    pub fn pop(&mut self) -> Option<T> {
        self.0.pop()
    }

    /// Returns the number of elements.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the vector is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Clears the vector.
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Returns a reference to the element at the index.
    pub fn get(&self, index: usize) -> Option<&T> {
        self.0.get(index)
    }

    /// Returns a mutable reference to the element at the index.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.0.get_mut(index)
    }

    /// Sets the element at the index, returning the old value.
    pub fn set(&mut self, index: usize, value: T) -> Option<T> {
        if index < self.0.len() {
            Some(std::mem::replace(&mut self.0[index], value))
        } else {
            None
        }
    }

    /// Inserts an element at the index, shifting elements to the right.
    pub fn insert(&mut self, index: usize, value: T) {
        self.0.insert(index, value);
    }

    /// Removes and returns the element at the index.
    pub fn remove(&mut self, index: usize) -> T {
        self.0.remove(index)
    }

    /// Appends all elements from another vector.
    pub fn append(&mut self, other: &mut Vector<T>) {
        self.0.append(&mut other.0);
    }

    /// Returns a reference to the first element.
    pub fn first(&self) -> Option<&T> {
        self.0.first()
    }

    /// Returns a reference to the last element.
    pub fn last(&self) -> Option<&T> {
        self.0.last()
    }

    /// Returns a slice of the vector.
    pub fn as_slice(&self) -> &[T] {
        &self.0
    }

    /// Returns a mutable slice.
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.0
    }

    /// Reverses the vector in place.
    pub fn reverse(&mut self) {
        self.0.reverse();
    }

    /// Sorts the vector in place.
    pub fn sort(&mut self)
    where
        T: Ord,
    {
        self.0.sort();
    }

    /// Sorts the vector by a key function.
    pub fn sort_by_key<F, K>(&mut self, f: F)
    where
        F: FnMut(&T) -> K,
        K: Ord,
    {
        self.0.sort_by_key(f);
    }

    /// Returns `true` if the vector contains the value.
    pub fn contains(&self, value: &T) -> bool
    where
        T: PartialEq,
    {
        self.0.contains(value)
    }

    /// Returns an iterator over the elements.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.0.iter()
    }

    /// Returns an iterator that allows mutation.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.0.iter_mut()
    }

    /// Extends the vector from an iterator.
    pub fn extend(&mut self, iter: impl IntoIterator<Item = T>) {
        self.0.extend(iter);
    }

    /// Reserves capacity for at least `additional` more elements.
    pub fn reserve(&mut self, additional: usize) {
        self.0.reserve(additional);
    }

    /// Shrinks the capacity to fit the length.
    pub fn shrink_to_fit(&mut self) {
        self.0.shrink_to_fit();
    }
}

impl<T: Clone> Vector<T> {
    /// Returns a cloned slice over the given range.
    pub fn slice<R>(&self, range: R) -> Vector<T>
    where
        R: std::slice::SliceIndex<[T], Output = [T]>,
    {
        Self(self.0[range].to_vec())
    }
}

impl<T> FromIterator<T> for Vector<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl<T> From<Vec<T>> for Vector<T> {
    fn from(vec: Vec<T>) -> Self {
        Self(vec)
    }
}

impl<T> From<Vector<T>> for Vec<T> {
    fn from(vec: Vector<T>) -> Self {
        vec.0
    }
}

impl<T> std::ops::Index<usize> for Vector<T> {
    type Output = T;

    fn index(&self, index: usize) -> &T {
        &self.0[index]
    }
}

impl<T> std::ops::IndexMut<usize> for Vector<T> {
    fn index_mut(&mut self, index: usize) -> &mut T {
        &mut self.0[index]
    }
}
