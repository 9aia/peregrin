use std::borrow::Borrow;
use std::collections::HashSet;
use std::hash::Hash;

/// An unordered set of unique values.
#[derive(Clone, Debug)]
pub struct Set<T>(HashSet<T>);

impl<T: Eq + Hash> Set<T> {
    /// Creates an empty set.
    pub fn new() -> Self {
        Self(HashSet::new())
    }

    /// Creates an empty set with the given capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self(HashSet::with_capacity(capacity))
    }

    /// Adds a value, returning `true` if it was not already present.
    pub fn insert(&mut self, value: T) -> bool {
        self.0.insert(value)
    }

    /// Removes a value, returning `true` if it was present.
    pub fn remove<Q>(&mut self, value: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.0.remove(value)
    }

    /// Returns `true` if the set contains the value.
    pub fn contains<Q>(&self, value: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.0.contains(value)
    }

    /// Returns the number of values in the set.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the set is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Clears the set.
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Returns `true` if the set has no elements in common with `other`.
    pub fn is_disjoint(&self, other: &Set<T>) -> bool {
        self.0.is_disjoint(&other.0)
    }

    /// Returns `true` if every element of this set is in `other`.
    pub fn is_subset(&self, other: &Set<T>) -> bool {
        self.0.is_subset(&other.0)
    }

    /// Returns `true` if this set contains every element of `other`.
    pub fn is_superset(&self, other: &Set<T>) -> bool {
        self.0.is_superset(&other.0)
    }

    /// Returns an iterator over the set.
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
}

impl<T: Eq + Hash + Clone> Set<T> {
    /// Returns a new set with elements that are in either set.
    pub fn union(&self, other: &Set<T>) -> Set<T> {
        Self(self.0.union(&other.0).cloned().collect())
    }

    /// Returns a new set with elements that are in both sets.
    pub fn intersection(&self, other: &Set<T>) -> Set<T> {
        Self(self.0.intersection(&other.0).cloned().collect())
    }

    /// Returns a new set with elements in this set but not in `other`.
    pub fn difference(&self, other: &Set<T>) -> Set<T> {
        Self(self.0.difference(&other.0).cloned().collect())
    }

    /// Returns a new set with elements in exactly one of the sets.
    pub fn symmetric_difference(&self, other: &Set<T>) -> Set<T> {
        Self(self.0.symmetric_difference(&other.0).cloned().collect())
    }
}

impl<T: Eq + Hash> FromIterator<T> for Set<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self(HashSet::from_iter(iter))
    }
}

impl<T: Eq + Hash> From<HashSet<T>> for Set<T> {
    fn from(set: HashSet<T>) -> Self {
        Self(set)
    }
}

impl<T: Eq + Hash> From<Set<T>> for HashSet<T> {
    fn from(set: Set<T>) -> Self {
        set.0
    }
}
