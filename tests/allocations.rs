//! Steps of the sampler must not allocate once the chain is set up.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use rove::data::GapTreatment;
use rove::mcmc::{Conditioning, Sampler, Settings};
use rove::nexus;

struct Counting;

static COUNTING: AtomicBool = AtomicBool::new(false);
static CALLS: AtomicUsize = AtomicUsize::new(0);

// SAFETY: every call is forwarded unchanged to the system allocator.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if COUNTING.load(Ordering::Relaxed) {
            CALLS.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        if COUNTING.load(Ordering::Relaxed) {
            CALLS.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.realloc(ptr, layout, size) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

#[test]
fn steps_do_not_allocate() {
    let data = nexus::parse(include_str!("../fixtures/reference/20x50/matrix.nex")).unwrap();
    let sites = 0..data.character_count();
    let patterns = data
        .compress(std::slice::from_ref(&sites), GapTreatment::Reject)
        .unwrap()
        .remove(0);
    for base in [Settings::default(), Settings::reference()] {
        for conditioning in [Conditioning::Variable, Conditioning::None] {
            let settings = Settings {
                conditioning,
                monophyly: vec![(0..19).collect(), vec![0, 1]],
                tuning_delay: 100,
                ..base.clone()
            };
            let mut sampler = Sampler::new(&patterns, &settings, 11).unwrap();
            for _ in 0..1000 {
                sampler.step();
            }
            COUNTING.store(true, Ordering::Relaxed);
            for _ in 0..20_000 {
                sampler.step();
            }
            COUNTING.store(false, Ordering::Relaxed);
            assert_eq!(CALLS.swap(0, Ordering::Relaxed), 0, "{conditioning:?}");
            let multistate =
                nexus::parse(include_str!("../fixtures/validation/multistate/matrix.nex"))
                    .unwrap()
                    .partition_by_states(GapTreatment::Reject)
                    .unwrap();
            let mut sampler = Sampler::new(
                &multistate,
                &Settings {
                    monophyly: Vec::new(),
                    ..settings.clone()
                },
                11,
            )
            .unwrap();
            for _ in 0..1000 {
                sampler.step();
            }
            COUNTING.store(true, Ordering::Relaxed);
            for _ in 0..20_000 {
                sampler.step();
            }
            COUNTING.store(false, Ordering::Relaxed);
            assert_eq!(
                CALLS.swap(0, Ordering::Relaxed),
                0,
                "multistate {conditioning:?}"
            );
        }
    }
}
