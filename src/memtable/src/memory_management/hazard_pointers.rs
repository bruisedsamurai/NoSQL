// Michael, Maged M. "Hazard pointers: Safe memory reclamation for lock-free objects."
// IEEE Transactions on Parallel and Distributed Systems 15, no. 6 (2004): 491-504.

//Reference implementation
//https://github.com/pramalhe/ConcurrencyFreaks/blob/master/CPP/papers/hazarderas

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU32, Ordering};
use std::sync::Arc;

pub struct HazarPointerRecord<T> {
    pub hazard_pointers: Vec<AtomicPtr<T>>,
    pub next: AtomicPtr<HazarPointerRecord<T>>,
    pub active: AtomicBool,
    pub r_list: HashSet<*mut T>,
    pub r_count: usize,
    pub head: Arc<AtomicPtr<HazarPointerRecord<T>>>,
    pub max_hp_count: Arc<AtomicU32>,
}

impl<T> HazarPointerRecord<T> {
    pub fn allocate_hp_record(
        head: Arc<AtomicPtr<HazarPointerRecord<T>>>,
        max_hp_count: Arc<AtomicU32>,
        per_record_hp_count: u32,
    ) -> *mut HazarPointerRecord<T> {
        let mut hp_record: *mut HazarPointerRecord<T>;
        hp_record = head.load(Ordering::SeqCst);
        while hp_record != std::ptr::null_mut() {
            unsafe {
                if (*hp_record).active.load(Ordering::SeqCst) {
                    hp_record = (*hp_record).next.load(Ordering::SeqCst);
                    continue;
                }
                if (*hp_record)
                    .active
                    .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                    .is_err()
                {
                    hp_record = (*hp_record).next.load(Ordering::SeqCst);
                    continue;
                }
            }
            return hp_record;
        }

        loop {
            unsafe {
                let old_count = max_hp_count.load(Ordering::SeqCst);
                if max_hp_count
                    .compare_exchange(
                        old_count,
                        old_count + per_record_hp_count,
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
        let mut hprec = Box::into_raw(Box::new(HazarPointerRecord {
            hazard_pointers: std::iter::repeat_with(|| AtomicPtr::new(std::ptr::null_mut()))
                .take(per_record_hp_count as usize)
                .collect(),
            next: AtomicPtr::new(std::ptr::null_mut()),
            active: AtomicBool::new(true),
            r_list: HashSet::new(),
            r_count: 0,
            head:head.clone(),
            max_hp_count: max_hp_count.clone(),
        }));

        loop {
            unsafe {
                let old_head = head.load(Ordering::SeqCst);
                (*hprec).next = AtomicPtr::new(old_head);
                if let Ok(_) =
                    head.compare_exchange(old_head, hprec, Ordering::SeqCst, Ordering::SeqCst)
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
        let hp_list: &Vec<AtomicPtr<T>>;
        unsafe {
            hp_length = (*rec_node).hazard_pointers.len();
            hp_list = (*rec_node).hazard_pointers.as_ref();
        }
        // HazarPointerRecord::scan(self.head.load(Ordering::SeqCst),rec_node);
        for i in 0..hp_length {
            hp_list[i].store(std::ptr::null_mut(), Ordering::SeqCst);
        }
        unsafe {
            (*rec_node).active.store(false, Ordering::SeqCst);
        }
    }

    //An object can only be retired if it is no longer accessible to any thread that comes after
    // OR
    // call to retire_node implies that `node` is a reference to an object that is no
    // longer reachable from any other object or global reference.
    pub fn retire_node(
        head: *mut HazarPointerRecord<T>,
        hp_record_ptr: *mut HazarPointerRecord<T>,
        node: *mut T,
        max_r_count: usize,
    ) {
        let self_r_count;
        unsafe {
            debug_assert!((*hp_record_ptr).active.load(Ordering::SeqCst) == true);
            if (*hp_record_ptr).r_list.insert(node) {
                (*hp_record_ptr).r_count += 1;
            }
            self_r_count = (*hp_record_ptr).r_count;
        }
        if self_r_count >= max_r_count {
            unsafe {
                HazarPointerRecord::scan(head, hp_record_ptr);
                HazarPointerRecord::help_scan(head, hp_record_ptr, max_r_count);
            }
        }
    }

    /// Removes hazard pointers from inactive hazard pointer records
    pub fn help_scan(
        head: *mut HazarPointerRecord<T>,
        self_ptr: *mut HazarPointerRecord<T>,
        max_hptr_count: usize,
    ) {
        let mut hp_record = head;
        while hp_record != std::ptr::null_mut() {
            unsafe {
                if (*hp_record).active.load(Ordering::SeqCst) {
                    hp_record = (*hp_record).next.load(Ordering::SeqCst);
                    continue;
                }
                if (*hp_record)
                    .active
                    .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                    .is_err()
                {
                    hp_record = (*hp_record).next.load(Ordering::SeqCst);
                    continue;
                }
                let set =  (*hp_record).r_list.drain().collect::<HashSet<*mut T>>();
                (*hp_record).r_count = 0;
                for node in set {
                    if (*self_ptr).r_list.insert(node) {
                        (*self_ptr).r_count += 1;
                    }
                    if (*self_ptr).r_count >= max_hptr_count {
                        HazarPointerRecord::scan(head,self_ptr);
                    }
                }
                (*hp_record).active.store(false, Ordering::SeqCst);
                hp_record = (*hp_record).next.load(Ordering::SeqCst);
            }
        }
    }

    /// Collect and release nodes if no hazar pointers from other hazard pointer records points to it
    fn scan(head: *mut HazarPointerRecord<T>, self_ptr: *mut HazarPointerRecord<T>) {
        let mut hazard_ptr_collection: HashSet<*mut T> = HashSet::new();
        let mut hp_record = head;
        while hp_record != std::ptr::null_mut() {
            let hp_iter = unsafe {
                (*hp_record).hazard_pointers.iter()
            };
            for h_pointer in hp_iter {
                if !h_pointer.load(Ordering::SeqCst).is_null() {
                    hazard_ptr_collection.insert(h_pointer.load(Ordering::SeqCst));
                }
            }
            hp_record = unsafe {
                (*hp_record).next.load(Ordering::SeqCst)
            };
        }

        let vec;
        unsafe {
            vec = (*self_ptr).r_list.drain().collect::<Vec<*mut T>>();
            (*self_ptr).r_count = 0;
        }
        for node in vec {
            if hazard_ptr_collection.contains(&node) {
                unsafe {
                    if (*self_ptr).r_list.insert(node){
                        (*self_ptr).r_count += 1;
                    }
                }
            } else {
                unsafe {
                    let _ = Box::from_raw(node);
                }
            }
        }
    }
}

// Todo: Implement drop trait for HPR
//
// impl<T> Drop for HazarPointerRecord<T> {
//     fn drop(&mut self) {
//         HazarPointerRecord::retire_hp_record(self as *mut HazarPointerRecord<T>);
//     }
// }
