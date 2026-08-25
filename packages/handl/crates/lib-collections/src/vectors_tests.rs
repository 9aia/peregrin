use super::vectors::Vector;

#[test]
fn new_vector_is_empty() {
    let v = Vector::<i32>::new();
    assert!(v.is_empty());
    assert_eq!(v.len(), 0);
}

#[test]
fn push_pop() {
    let mut v = Vector::new();
    v.push(1);
    v.push(2);
    assert_eq!(v.pop(), Some(2));
    assert_eq!(v.pop(), Some(1));
    assert_eq!(v.pop(), None);
}

#[test]
fn get_and_set() {
    let mut v = Vector::new();
    v.push(10);
    v.push(20);
    assert_eq!(v.get(0), Some(&10));
    assert_eq!(v.get_mut(1), Some(&mut 20));
    assert_eq!(v.set(1, 25), Some(20));
    assert_eq!(v[1], 25);
    assert_eq!(v.set(5, 0), None);
}

#[test]
fn insert_remove() {
    let mut v = Vector::new();
    v.push(1);
    v.push(3);
    v.insert(1, 2);
    assert_eq!(v.as_slice(), &[1, 2, 3]);
    assert_eq!(v.remove(1), 2);
    assert_eq!(v.as_slice(), &[1, 3]);
}

#[test]
fn first_last() {
    let mut v = Vector::new();
    assert_eq!(v.first(), None);
    assert_eq!(v.last(), None);
    v.push(1);
    v.push(2);
    assert_eq!(v.first(), Some(&1));
    assert_eq!(v.last(), Some(&2));
}

#[test]
fn reverse_and_sort() {
    let mut v = Vector::from_iter(vec![3, 1, 2]);
    v.reverse();
    assert_eq!(v.as_slice(), &[2, 1, 3]);
    v.sort();
    assert_eq!(v.as_slice(), &[1, 2, 3]);
}

#[test]
fn sort_by_key() {
    let mut v = Vector::from_iter(vec![(2, "b"), (1, "a"), (3, "c")]);
    v.sort_by_key(|x| x.0);
    assert_eq!(v[0].0, 1);
    assert_eq!(v[1].0, 2);
    assert_eq!(v[2].0, 3);
}

#[test]
fn contains_and_slice() {
    let v = Vector::from_iter(vec![1, 2, 3, 4]);
    assert!(v.contains(&2));
    assert!(!v.contains(&5));
    let s = v.slice(1..3);
    assert_eq!(s.as_slice(), &[2, 3]);
}

#[test]
fn append_extend_and_index() {
    let mut a = Vector::from_iter(vec![1, 2]);
    let mut b = Vector::from_iter(vec![3, 4]);
    a.append(&mut b);
    a.extend(vec![5]);
    assert_eq!(a[4], 5);
    assert_eq!(a.as_slice(), &[1, 2, 3, 4, 5]);
    assert!(b.is_empty());
}

#[test]
fn reserve_shrink() {
    let mut v = Vector::with_capacity(10);
    v.reserve(5);
    v.push(1);
    v.shrink_to_fit();
    assert_eq!(v.len(), 1);
}
