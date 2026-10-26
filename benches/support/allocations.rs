//! Allocation observations for the single-threaded benchmark executable only.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

pub struct CountingAllocator;

static ENABLED: AtomicBool = AtomicBool::new(false);
static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static REALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static BYTES: AtomicUsize = AtomicUsize::new(0);
static DEALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

fn record(counter: &AtomicUsize, bytes: usize) {
    if ENABLED.load(Ordering::Relaxed) {
        counter.fetch_add(1, Ordering::Relaxed);
        BYTES.fetch_add(bytes, Ordering::Relaxed);
    }
}

// SAFETY: All allocation operations delegate to System with their original
// arguments and return values. Instrumentation only touches atomics: it neither
// allocates nor unwinds and does not inspect or change allocated memory.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record(&ALLOCATIONS, layout.size());
        // SAFETY: The caller supplies the layout required by GlobalAlloc.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record(&ALLOCATIONS, layout.size());
        // SAFETY: The caller supplies the layout required by GlobalAlloc.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        record(&REALLOCATIONS, new_size);
        // SAFETY: The pointer, original layout and new size are passed through.
        unsafe { System.realloc(ptr, layout, new_size) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if ENABLED.load(Ordering::Relaxed) {
            DEALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
        // SAFETY: The pointer and layout came from the same System allocator.
        unsafe { System.dealloc(ptr, layout) }
    }
}

pub struct Counts {
    pub allocations: usize,
    pub reallocations: usize,
    pub bytes: usize,
    pub deallocations: usize,
}

struct Tracking;

impl Drop for Tracking {
    fn drop(&mut self) {
        ENABLED.store(false, Ordering::SeqCst);
    }
}

pub fn observe(mut operation: impl FnMut() -> f64) -> Counts {
    assert!(
        !ENABLED.load(Ordering::Relaxed),
        "nested allocation observation"
    );
    for counter in [&ALLOCATIONS, &REALLOCATIONS, &BYTES, &DEALLOCATIONS] {
        counter.store(0, Ordering::Relaxed);
    }
    ENABLED.store(true, Ordering::SeqCst);
    let tracking = Tracking;
    std::hint::black_box(operation());
    drop(tracking);
    Counts {
        allocations: ALLOCATIONS.load(Ordering::Relaxed),
        reallocations: REALLOCATIONS.load(Ordering::Relaxed),
        bytes: BYTES.load(Ordering::Relaxed),
        deallocations: DEALLOCATIONS.load(Ordering::Relaxed),
    }
}
