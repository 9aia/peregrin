use super::maps::Map;

#[test]
fn new_map_is_empty() {
    let map = Map::<&str, i32>::new();
    assert!(map.is_empty());
    assert_eq!(map.len(), 0);
}

#[test]
fn insert_and_get() {
    let mut map = Map::new();
    map.insert("a", 1);
    assert_eq!(map.get("a"), Some(&1));
    map.insert("a", 2);
    assert_eq!(map.get("a"), Some(&2));
}

#[test]
fn insert_returns_previous() {
    let mut map = Map::new();
    assert_eq!(map.insert("a", 1), None);
    assert_eq!(map.insert("a", 2), Some(1));
}

#[test]
fn get_mut() {
    let mut map = Map::new();
    map.insert("a", 1);
    *map.get_mut("a").unwrap() += 10;
    assert_eq!(map.get("a"), Some(&11));
}

#[test]
fn contains_key() {
    let mut map = Map::new();
    map.insert("a", 1);
    assert!(map.contains_key("a"));
    assert!(!map.contains_key("b"));
}

#[test]
fn remove() {
    let mut map = Map::new();
    map.insert("a", 1);
    assert_eq!(map.remove("a"), Some(1));
    assert_eq!(map.remove("a"), None);
    assert!(map.is_empty());
}

#[test]
fn clear_and_len() {
    let mut map = Map::new();
    map.insert("a", 1);
    map.insert("b", 2);
    assert_eq!(map.len(), 2);
    map.clear();
    assert_eq!(map.len(), 0);
    assert!(map.is_empty());
}

#[test]
fn iterators_and_to_vec() {
    let mut map = Map::new();
    map.insert("a", 1);
    map.insert("b", 2);
    let mut pairs = map.to_vec();
    pairs.sort();
    assert_eq!(pairs, vec![("a", 1), ("b", 2)]);
    assert_eq!(map.keys().count(), 2);
    assert_eq!(map.values().count(), 2);
    assert_eq!(map.entries().count(), 2);
}

#[test]
fn from_iterator_and_merge() {
    let mut map1 = Map::from_iter(vec![("a", 1), ("b", 2)]);
    let map2 = Map::from_iter(vec![("b", 20), ("c", 3)]);
    map1.merge(map2);
    assert_eq!(map1.get("a"), Some(&1));
    assert_eq!(map1.get("b"), Some(&20));
    assert_eq!(map1.get("c"), Some(&3));
    assert_eq!(map1.len(), 3);
}

#[test]
fn with_capacity_and_reserve() {
    let mut map = Map::<&str, i32>::with_capacity(10);
    map.reserve(5);
    map.insert("a", 1);
    assert_eq!(map.len(), 1);
}

#[test]
fn from_hash_map() {
    let mut std_map = std::collections::HashMap::new();
    std_map.insert("x", 10);
    let map = Map::from(std_map);
    assert_eq!(map.get("x"), Some(&10));
}
