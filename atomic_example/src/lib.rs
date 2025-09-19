use crossbeam_epoch::{self as epoch, Atomic, Owned};
use std::sync::{atomic::Ordering, Mutex};

pub struct FreeList<T> {
    head: Atomic<Node<T>>,
}

struct Node<T> {
    value: T,
    next: Atomic<Node<T>>,
}

impl<T> FreeList<T> {
    pub fn new() -> Self {
        Self { head: Atomic::null() }
    }

    pub fn push(&self, value: T) {
        let mut node = Owned::new(Node {
            value,
            next: Atomic::null(),
        });

        let guard = &epoch::pin();
        loop {
            let head = self.head.load(Ordering::Acquire, guard);
            node.next.store(head, Ordering::Relaxed);

            match self.head.compare_exchange(
                head,
                node,
                Ordering::Release,
                Ordering::Relaxed,
                guard,
            ) {
                Ok(_) => return,
                Err(e) => {
                    node = e.new; // retry with the owned node
                }
            }
        }
    }



    pub fn pop(&self) -> Option<T> {
        let guard = &epoch::pin();
        loop {
            let head = self.head.load(Ordering::Acquire, guard);
            if head.is_null() {
                return None;
            }

            let next = unsafe { head.deref().next.load(Ordering::Acquire, guard) };

            match self.head.compare_exchange(
                head,
                next,
                Ordering::Release,
                Ordering::Relaxed,
                guard,
            ) {
                Ok(_) => {
                    // SAFETY: We successfully unlinked this node from the list, so no other thread can access it anymore. We need to read the value before deferring destruction.
                    let value = unsafe { std::ptr::read(&(*head.as_raw()).value) };

                    // Defer the destruction of the node until all currently pinned threads are unpinned. This ensures memory safety.
                    unsafe {
                        guard.defer_destroy(head);
                    }

                    return Some(value);
                }
                Err(_) => continue,
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        let guard = &epoch::pin();
        self.head.load(Ordering::Relaxed, guard).is_null()
    }
}

impl<T> Drop for FreeList<T> {
    fn drop(&mut self) {
        let guard = &epoch::pin();

        // Start from the head
        let mut current = self.head.load(Ordering::Relaxed, guard);

        // Iteratively process each node to avoid recursive drop and stack overflow
        while !current.is_null() {
            unsafe {
                // Get the raw pointer to the current node
                let node_ptr = current.as_raw() as *mut Node<T>;

                // Load the next node before we modify the current one
                let next = (*node_ptr).next.load(Ordering::Relaxed, guard);

                // Clear the next field to break the chain before dropping
                // This prevents recursive drop by ensuring each Node<T>
                // has an empty next field when it gets dropped
                (*node_ptr).next.store(epoch::Shared::null(), Ordering::Relaxed);

                // Now safely drop the current node
                // Since next is null, this won't cause recursive drop
                let _ = Owned::from_raw(node_ptr);

                // Move to the next node
                current = next;
            }
        }
    }
}


unsafe impl<T: Send> Send for FreeList<T> {}
unsafe impl<T: Send> Sync for FreeList<T> {}

pub struct MutexLinkedList<T> {
    head: Mutex<Option<Box<MutexNode<T>>>>,
}

struct MutexNode<T> {
    value: T,
    next: Option<Box<MutexNode<T>>>,
}

impl<T> MutexLinkedList<T> {
    pub fn new() -> Self {
        Self { head: Mutex::new(None) }
    }

    pub fn push(&self, value: T) {
        let mut guard = self.head.lock().unwrap();
        let new = Box::new(MutexNode {
            value,
            next: guard.take(),
        });
        *guard = Some(new);
    }

    pub fn pop(&self) -> Option<T> {
        let mut guard = self.head.lock().unwrap();
        guard.take().map(|boxed_node| {
            let MutexNode { value, next } = *boxed_node;
            *guard = next;
            value
        })
    }

    pub fn is_empty(&self) -> bool {
        let guard = self.head.lock().unwrap();
        guard.is_none()
    }
}

unsafe impl<T: Send> Send for MutexLinkedList<T> {}
unsafe impl<T: Send> Sync for MutexLinkedList<T> {}
