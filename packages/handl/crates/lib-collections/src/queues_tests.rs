use super::queues::{Deque, Queue};

#[test]
fn new_queue_is_empty() {
    let q = Queue::<i32>::new();
    assert!(q.is_empty());
    assert_eq!(q.len(), 0);
}

#[test]
fn enqueue_dequeue() {
    let mut q = Queue::new();
    q.enqueue(1);
    q.enqueue(2);
    assert_eq!(q.peek(), Some(&1));
    assert_eq!(q.peek_back(), Some(&2));
    assert_eq!(q.dequeue(), Some(1));
    assert_eq!(q.dequeue(), Some(2));
    assert_eq!(q.dequeue(), None);
}

#[test]
fn push_front_pop_back() {
    let mut q = Queue::new();
    q.push_front(1);
    q.push_front(2);
    assert_eq!(q.pop_back(), Some(1));
    assert_eq!(q.dequeue(), Some(2));
}

#[test]
fn clear_and_to_vec() {
    let mut q = Queue::from_iter(vec![1, 2, 3]);
    assert_eq!(q.len(), 3);
    q.clear();
    assert!(q.is_empty());
    q.enqueue(4);
    assert_eq!(q.to_vec(), vec![4]);
}

#[test]
fn contains_and_iter() {
    let q = Queue::from_iter(vec![1, 2, 3]);
    assert!(q.contains(&2));
    assert!(!q.contains(&4));
    assert_eq!(q.iter().count(), 3);
}

#[test]
fn deque_alias() {
    let mut d = Deque::new();
    d.enqueue(1);
    d.push_front(0);
    assert_eq!(d.to_vec(), vec![0, 1]);
}
