use super::trees::Tree;

#[test]
fn new_tree_is_empty() {
    let t = Tree::<i32>::new();
    assert!(t.is_empty());
    assert_eq!(t.len(), 0);
}

#[test]
fn insert_and_contains() {
    let mut t = Tree::new();
    t.insert(5);
    t.insert(3);
    t.insert(7);
    assert!(t.contains(&5));
    assert!(t.contains(&3));
    assert!(t.contains(&7));
    assert!(!t.contains(&10));
    assert_eq!(t.len(), 3);
}

#[test]
fn in_order() {
    let mut t = Tree::new();
    t.insert(5);
    t.insert(3);
    t.insert(7);
    t.insert(1);
    t.insert(9);
    assert_eq!(t.in_order(), vec![&1, &3, &5, &7, &9]);
}

#[test]
fn pre_and_post_order() {
    let mut t = Tree::new();
    t.insert(2);
    t.insert(1);
    t.insert(3);
    assert_eq!(t.pre_order(), vec![&2, &1, &3]);
    assert_eq!(t.post_order(), vec![&1, &3, &2]);
}

#[test]
fn min_and_max() {
    let mut t = Tree::new();
    t.insert(3);
    t.insert(1);
    t.insert(5);
    assert_eq!(t.min(), Some(&1));
    assert_eq!(t.max(), Some(&5));
}

#[test]
fn remove_leaf() {
    let mut t = Tree::new();
    t.insert(2);
    t.insert(1);
    t.insert(3);
    assert!(t.remove(&1));
    assert!(!t.contains(&1));
    assert!(t.contains(&2));
    assert!(t.contains(&3));
    assert_eq!(t.len(), 2);
}

#[test]
fn remove_with_one_child() {
    let mut t = Tree::new();
    t.insert(3);
    t.insert(1);
    t.insert(2);
    assert!(t.remove(&1));
    assert_eq!(t.in_order(), vec![&2, &3]);
}

#[test]
fn remove_root_with_two_children() {
    let mut t = Tree::new();
    t.insert(5);
    t.insert(3);
    t.insert(7);
    t.insert(2);
    t.insert(4);
    t.insert(6);
    t.insert(8);
    assert!(t.remove(&5));
    assert_eq!(t.len(), 6);
    assert!(t.in_order().iter().all(|v| **v != 5));
    assert!(t.contains(&3));
    assert!(t.contains(&7));
}

#[test]
fn remove_nonexistent() {
    let mut t = Tree::new();
    t.insert(1);
    assert!(!t.remove(&2));
    assert_eq!(t.len(), 1);
}

#[test]
fn clear() {
    let mut t = Tree::new();
    t.insert(1);
    t.clear();
    assert!(t.is_empty());
}
