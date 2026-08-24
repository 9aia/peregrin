use super::sets::Set;

#[test]
fn new_set_is_empty() {
    let set = Set::<i32>::new();
    assert!(set.is_empty());
    assert_eq!(set.len(), 0);
}

#[test]
fn insert_contains_remove() {
    let mut set = Set::new();
    assert!(set.insert(1));
    assert!(!set.insert(1));
    assert!(set.contains(&1));
    assert!(!set.contains(&2));
    assert!(set.remove(&1));
    assert!(!set.remove(&1));
    assert!(set.is_empty());
}

#[test]
fn clear_and_len() {
    let mut set = Set::new();
    set.insert(1);
    set.insert(2);
    assert_eq!(set.len(), 2);
    set.clear();
    assert!(set.is_empty());
}

#[test]
fn operations() {
    let a = Set::from_iter(vec![1, 2, 3]);
    let b = Set::from_iter(vec![2, 3, 4]);
    let union = a.union(&b);
    let intersection = a.intersection(&b);
    let difference = a.difference(&b);
    let sym_diff = a.symmetric_difference(&b);
    assert!(union.contains(&1));
    assert!(union.contains(&4));
    assert!(intersection.contains(&2));
    assert!(difference.contains(&1));
    assert!(!difference.contains(&4));
    assert!(sym_diff.contains(&1));
    assert!(sym_diff.contains(&4));
    assert!(!sym_diff.contains(&2));
}

#[test]
fn subset_disjoint_superset() {
    let a = Set::from_iter(vec![1, 2]);
    let b = Set::from_iter(vec![1, 2, 3]);
    assert!(a.is_subset(&b));
    assert!(b.is_superset(&a));
    let c = Set::from_iter(vec![4, 5]);
    assert!(a.is_disjoint(&c));
}

#[test]
fn to_vec_and_iter() {
    let set = Set::from_iter(vec![3, 1, 2]);
    let mut v = set.to_vec();
    v.sort();
    assert_eq!(v, vec![1, 2, 3]);
    assert_eq!(set.iter().count(), 3);
}
