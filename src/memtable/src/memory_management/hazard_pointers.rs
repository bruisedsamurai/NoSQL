// Michael, Maged M. "Hazard pointers: Safe memory reclamation for lock-free objects."
// IEEE Transactions on Parallel and Distributed Systems 15, no. 6 (2004): 491-504.
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU32, Ordering};
use std::sync::Arc;

pub struct HazarPointerRecord<T> {
    pub hazard_pointers: Vec<*mut T>,
    pub next: *mut HazarPointerRecord<T>,
    pub active: AtomicBool,
    pub r_list: HashSet<*mut T>,
    pub r_count: usize,
}

impl<T> HazarPointerRecord<T> {
    pub fn allocate_hp_record(
        head: Arc<AtomicPtr<HazarPointerRecord<T>>>,
        total_hp_count: Arc<AtomicU32>,
        per_record_hp_count: u32,
    ) -> *mut HazarPointerRecord<T> {
        let mut hp_record: *mut HazarPointerRecord<T>;
        unsafe {
            hp_record = (*head).load(Ordering::SeqCst);
        }
        while hp_record != std::ptr::null_mut() {
            unsafe {
                if (*hp_record).active.load(Ordering::SeqCst) {
                    hp_record = (*hp_record).next;
                    continue;
                }
                if !(*hp_record)
                    .active
                    .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok()
                {
                    hp_record = (*hp_record).next;
                    continue;
                }
            }
            return hp_record;
        }

        loop {
            unsafe {
                let oldcount = (*total_hp_count).load(Ordering::SeqCst);
                if (*total_hp_count)
                    .compare_exchange(
                        oldcount,
                        oldcount + per_record_hp_count,
                        Ordering::SeqCst,
                        Ordering::SeqCst,
                    )
                    .is_ok()
                {
                    break;
                }
            }
        }

        //cursed code
        let hprec = Box::into_raw(Box::new(HazarPointerRecord {
            hazard_pointers: vec![std::ptr::null_mut(); per_record_hp_count as usize],
            next: std::ptr::null_mut(),
            active: AtomicBool::new(true),
            r_list: HashSet::new(),
            r_count: 0,
        }));

        loop {
            unsafe {
                let old_head = (*head).load(Ordering::SeqCst);
                (*hprec).next = old_head;
                if let Ok(ptr) =
                    (*head).compare_exchange(old_head, hprec, Ordering::SeqCst, Ordering::SeqCst)
                {
                    // let _ = Box::from_raw(ptr);
                    break;
                }
            }
        }
        unsafe { hprec }
    }

    pub fn retire_hp_record(rec_node: *mut HazarPointerRecord<T>) {
        let hp_length;
        let hp_list: &mut Vec<*mut T>;
        unsafe {
            hp_length = (*rec_node).hazard_pointers.len();
            hp_list = (*rec_node).hazard_pointers.as_mut();
        }
        for i in 0..hp_length {
            hp_list[i] = std::ptr::null_mut();
        }
        unsafe {
            (*rec_node).active.store(false, Ordering::SeqCst);
        }
    }

    pub fn retire_node(
        &mut self,
        node: *mut T,
        head: *mut HazarPointerRecord<T>,
        max_hptr_count: usize,
    ) {
        self.r_list.insert(node);
        self.r_count += 1;
        if self.r_count >= max_hptr_count {
            self.scan(head);
            self.help_scan(head, max_hptr_count);
        }
    }

    /// Removes hazard pointers from inactive hazard pointer records
    fn help_scan(&mut self, head_hp_record: *mut HazarPointerRecord<T>, max_hptr_count: usize) {
        let mut hp_record = head_hp_record;
        while hp_record != std::ptr::null_mut() {
            unsafe {
                if (*hp_record).active.load(Ordering::SeqCst) {
                    hp_record = (*hp_record).next;
                    continue;
                }
                if !(*hp_record)
                    .active
                    .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok()
                {
                    hp_record = (*hp_record).next;
                    continue;
                }
                for node in (*hp_record).r_list.iter() {
                    (*hp_record).r_list.remove(node);
                    (*hp_record).r_count -= 1;
                    self.r_list.insert(*node);
                    self.r_count += 1;
                    let head = head_hp_record;
                    if self.r_count >= max_hptr_count {
                        self.scan(head);
                    }
                }
                (*hp_record).active.store(false, Ordering::SeqCst);
                hp_record = (*hp_record).next;
            }
        }
    }

    /// Collect and release nodes if no hazar pointers from other hazard pointer records points to it
    fn scan(&mut self, head: *mut HazarPointerRecord<T>) {
        let mut hazard_ptr_collection: HashSet<*mut T> = HashSet::new();
        let mut hp_record = head;
        while hp_record != std::ptr::null_mut() {
            unsafe {
                for &h_pointer in (*hp_record).hazard_pointers.iter() {
                    if !h_pointer.is_null() {
                        hazard_ptr_collection.insert(h_pointer);
                    }
                }
                hp_record = (*hp_record).next;
            }
        }

        let vec = self.r_list.drain().collect::<Vec<*mut T>>();
        self.r_count = 0;
        for node in vec {
            if hazard_ptr_collection.contains(&node) {
                self.r_list.insert(node);
                self.r_count += 1;
            } else {
                unsafe {
                    let _ = Box::from_raw(node);
                }
            }
        }
    }
}
