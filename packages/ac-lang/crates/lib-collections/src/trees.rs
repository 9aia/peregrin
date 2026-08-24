/// A node in the binary search tree.
#[derive(Clone, Debug)]
struct Node<T> {
    value: T,
    left: Option<Box<Node<T>>>,
    right: Option<Box<Node<T>>>,
}

/// A binary search tree.
#[derive(Clone, Debug)]
pub struct Tree<T>(Option<Box<Node<T>>>);

impl<T> Tree<T> {
    /// Creates an empty tree.
    pub fn new() -> Self {
        Self(None)
    }

    /// Returns `true` if the tree is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_none()
    }

    /// Returns the number of values in the tree.
    pub fn len(&self) -> usize {
        self.0.as_deref().map_or(0, count)
    }

    /// Clears the tree.
    pub fn clear(&mut self) {
        self.0 = None;
    }

    /// Returns values in ascending order.
    pub fn in_order(&self) -> Vec<&T> {
        let mut out = Vec::new();
        if let Some(node) = self.0.as_deref() {
            in_order(node, &mut out);
        }
        out
    }

    /// Returns values in pre-order.
    pub fn pre_order(&self) -> Vec<&T> {
        let mut out = Vec::new();
        if let Some(node) = self.0.as_deref() {
            pre_order(node, &mut out);
        }
        out
    }

    /// Returns values in post-order.
    pub fn post_order(&self) -> Vec<&T> {
        let mut out = Vec::new();
        if let Some(node) = self.0.as_deref() {
            post_order(node, &mut out);
        }
        out
    }

    /// Returns the smallest value.
    pub fn min(&self) -> Option<&T> {
        self.0.as_deref().map(|mut n| {
            while let Some(left) = n.left.as_deref() {
                n = left;
            }
            &n.value
        })
    }

    /// Returns the largest value.
    pub fn max(&self) -> Option<&T> {
        self.0.as_deref().map(|mut n| {
            while let Some(right) = n.right.as_deref() {
                n = right;
            }
            &n.value
        })
    }
}

impl<T> Default for Tree<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Ord> Tree<T> {
    /// Inserts a value.
    pub fn insert(&mut self, value: T) {
        match &mut self.0 {
            None => {
                self.0 = Some(Box::new(Node {
                    value,
                    left: None,
                    right: None,
                }));
            }
            Some(node) => insert_node(node, value),
        }
    }

    /// Returns `true` if the tree contains the value.
    pub fn contains(&self, value: &T) -> bool {
        let mut current = self.0.as_deref();
        while let Some(node) = current {
            match value.cmp(&node.value) {
                std::cmp::Ordering::Equal => return true,
                std::cmp::Ordering::Less => current = node.left.as_deref(),
                std::cmp::Ordering::Greater => current = node.right.as_deref(),
            }
        }
        false
    }

    /// Removes a value, returning `true` if it was present.
    pub fn remove(&mut self, value: &T) -> bool {
        self.0.take().map_or(false, |node| {
            let (new_root, removed) = remove_node(node, value);
            self.0 = new_root;
            removed
        })
    }
}

fn insert_node<T: Ord>(node: &mut Node<T>, value: T) {
    let target = if value < node.value {
        &mut node.left
    } else if value > node.value {
        &mut node.right
    } else {
        node.value = value;
        return;
    };

    match target {
        None => {
            *target = Some(Box::new(Node {
                value,
                left: None,
                right: None,
            }));
        }
        Some(child) => insert_node(child, value),
    }
}

fn count<T>(node: &Node<T>) -> usize {
    let mut n = 1;
    if let Some(left) = &node.left {
        n += count(left);
    }
    if let Some(right) = &node.right {
        n += count(right);
    }
    n
}

fn in_order<'a, T>(node: &'a Node<T>, out: &mut Vec<&'a T>) {
    if let Some(left) = node.left.as_deref() {
        in_order(left, out);
    }
    out.push(&node.value);
    if let Some(right) = node.right.as_deref() {
        in_order(right, out);
    }
}

fn pre_order<'a, T>(node: &'a Node<T>, out: &mut Vec<&'a T>) {
    out.push(&node.value);
    if let Some(left) = node.left.as_deref() {
        pre_order(left, out);
    }
    if let Some(right) = node.right.as_deref() {
        pre_order(right, out);
    }
}

fn post_order<'a, T>(node: &'a Node<T>, out: &mut Vec<&'a T>) {
    if let Some(left) = node.left.as_deref() {
        post_order(left, out);
    }
    if let Some(right) = node.right.as_deref() {
        post_order(right, out);
    }
    out.push(&node.value);
}

fn remove_node<T: Ord>(mut node: Box<Node<T>>, value: &T) -> (Option<Box<Node<T>>>, bool) {
    match value.cmp(&node.value) {
        std::cmp::Ordering::Less => {
            if let Some(left) = node.left.take() {
                let (new_left, removed) = remove_node(left, value);
                node.left = new_left;
                (Some(node), removed)
            } else {
                (Some(node), false)
            }
        }
        std::cmp::Ordering::Greater => {
            if let Some(right) = node.right.take() {
                let (new_right, removed) = remove_node(right, value);
                node.right = new_right;
                (Some(node), removed)
            } else {
                (Some(node), false)
            }
        }
        std::cmp::Ordering::Equal => {
            let new_root = match (node.left.take(), node.right.take()) {
                (None, None) => None,
                (Some(left), None) => Some(left),
                (None, Some(right)) => Some(right),
                (Some(left), Some(right)) => Some(combine(left, right)),
            };
            (new_root, true)
        }
    }
}

fn combine<T: Ord>(left: Box<Node<T>>, right: Box<Node<T>>) -> Box<Node<T>> {
    let (min_value, new_right) = take_min(right);
    Box::new(Node {
        value: min_value,
        left: Some(left),
        right: new_right,
    })
}

fn take_min<T: Ord>(mut node: Box<Node<T>>) -> (T, Option<Box<Node<T>>>) {
    if let Some(left) = node.left.take() {
        let (min, new_left) = take_min(left);
        node.left = new_left;
        (min, Some(node))
    } else {
        let right = node.right.take();
        (node.value, right)
    }
}
