use std::process::Child;
use std::sync::Mutex;

#[derive(Default)]
pub struct OsSystem {
    children: Mutex<Vec<Child>>,
}

impl OsSystem {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn keep_child(&self, child: Child) {
        self.children.lock().unwrap().push(child);
    }

    pub fn reap_children(&self) {
        for mut child in self.children.lock().unwrap().drain(..) {
            let _ = child.try_wait();
        }
    }
}
