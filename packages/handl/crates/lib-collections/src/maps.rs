use std::borrow::Borrow;
use std::collections::HashMap;
use std::hash::Hash;

/// A key-value map backed by a hash table.
#[derive(Clone, Debug)]
pub struct Map<K, V>(HashMap<K, V>);

impl<K, V> Map<K, V>
where
    K: Eq + Hash,
{
    /// Creates an empty map.
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    /// Creates an empty map with the given capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self(HashMap::with_capacity(capacity))
    }

    /// Inserts a key-value pair, returning the previous value if present.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.0.insert(key, value)
    }

    /// Returns a reference to the value associated with the key.
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.0.get(key)
    }

    /// Returns a mutable reference to the value associated with the key.
    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.0.get_mut(key)
    }

    /// Removes a key and returns its value.
    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.0.remove(key)
    }

    /// Returns `true` if the map contains the key.
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.0.contains_key(key)
    }

    /// Returns the number of entries in the map.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the map contains no entries.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Clears all entries.
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Returns an iterator over the keys.
    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.0.keys()
    }

    /// Returns an iterator over the values.
    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.0.values()
    }

    /// Returns an iterator over the entries as `(key, value)` pairs.
    pub fn entries(&self) -> impl Iterator<Item = (&K, &V)> {
        self.0.iter()
    }

    /// Returns a vector of cloned `(key, value)` pairs.
    pub fn to_vec(&self) -> Vec<(K, V)>
    where
        K: Clone,
        V: Clone,
    {
        self.0.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }

    /// Extends the map with entries from an iterator.
    pub fn extend(&mut self, iter: impl IntoIterator<Item = (K, V)>) {
        self.0.extend(iter);
    }

    /// Merges another map into this one, overwriting existing entries.
    pub fn merge(&mut self, other: Map<K, V>) {
        self.0.extend(other.0);
    }

    /// Reserves capacity for at least `additional` more entries.
    pub fn reserve(&mut self, additional: usize) {
        self.0.reserve(additional);
    }

    /// Shrinks the capacity to fit the current length.
    pub fn shrink_to_fit(&mut self) {
        self.0.shrink_to_fit();
    }
}

impl<K, V> FromIterator<(K, V)> for Map<K, V>
where
    K: Eq + Hash,
{
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        Self(HashMap::from_iter(iter))
    }
}

impl<K, V> From<HashMap<K, V>> for Map<K, V> {
    fn from(map: HashMap<K, V>) -> Self {
        Self(map)
    }
}

impl<K, V> From<Map<K, V>> for HashMap<K, V> {
    fn from(map: Map<K, V>) -> Self {
        map.0
    }
}
