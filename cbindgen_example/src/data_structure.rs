use std::collections::HashSet;

#[derive(Default)]
pub struct IntHashSet {
    set: HashSet<usize>,
}


impl IntHashSet {
    pub fn new() -> Self {
        Default::default()
    }

    fn insert(&mut self, value: usize) {
        self.set.insert(value);
    }

    fn contains(&self, value: usize) -> bool {
        self.set.contains(&value)
    }

    fn remove(&mut self, value: usize) {
        self.set.remove(&value);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn int_set_new() -> *mut IntHashSet {
    Box::into_raw(Box::new(IntHashSet::new()))
}

#[unsafe(no_mangle)]
pub extern "C" fn int_set_insert(set: *mut IntHashSet, value: usize) {
    unsafe{
        if !set.is_null() {
            (*set).insert(value);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn int_set_remove(set: *mut IntHashSet, value: usize) {
    unsafe { 
        if !set.is_null() {
            (*set).remove(value);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn int_set_contain(set: *mut IntHashSet, value: usize) -> bool {
    unsafe { 
        if !set.is_null() {
            (*set).contains(value)
        } else {
            println!("IntHashSet is NULL pointer");
            false
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn int_set_free(set: *mut IntHashSet) {
    unsafe{ 
        if !set.is_null() {
            let _ = Box::from_raw(set);
        }
    }
}
