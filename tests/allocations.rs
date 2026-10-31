//! Steps of the sampler must not allocate once the chain is set up.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicBool, Ordering};

use rove::data::GapTreatment;
use rove::mcmc::{Conditioning, Sampler, Settings};
use rove::nexus;

#[cfg(feature = "experimental-repeats")]
#[path = "support/repeat_matrix.rs"]
mod repeat_matrix;

struct Counting;

// Per thread, so the test harness and concurrent tests never add to a count.
thread_local! {
    static COUNTING: Cell<bool> = const { Cell::new(false) };
    static CALLS: Cell<usize> = const { Cell::new(0) };
}

fn record() {
    if COUNTING.get() {
        CALLS.set(CALLS.get() + 1);
    }
}

// SAFETY: every call is forwarded unchanged to the system allocator.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record();
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        record();
        unsafe { System.realloc(ptr, layout, size) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// Allocations made by the calling thread while `work` runs.
fn allocations(work: impl FnOnce()) -> usize {
    CALLS.set(0);
    COUNTING.set(true);
    work();
    COUNTING.set(false);
    CALLS.get()
}

/// Allocations during `counted` steps after `warmup` uncounted ones.
fn step_allocations(sampler: &mut Sampler, warmup: usize, counted: usize) -> usize {
    for _ in 0..warmup {
        sampler.step();
    }
    allocations(|| {
        for _ in 0..counted {
            sampler.step();
        }
    })
}

#[test]
fn other_threads_are_not_counted() {
    static GO: AtomicBool = AtomicBool::new(false);
    static DONE: AtomicBool = AtomicBool::new(false);
    let worker = std::thread::spawn(|| {
        while !GO.load(Ordering::Acquire) {
            std::thread::yield_now();
        }
        std::hint::black_box(Vec::<u8>::with_capacity(4096));
        DONE.store(true, Ordering::Release);
    });
    let elsewhere = allocations(|| {
        GO.store(true, Ordering::Release);
        while !DONE.load(Ordering::Acquire) {
            std::thread::yield_now();
        }
    });
    worker.join().unwrap();
    assert_eq!(elsewhere, 0);
    let here = allocations(|| {
        std::hint::black_box(Vec::<u8>::with_capacity(4096));
    });
    assert_eq!(here, 1);
}

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
            assert_eq!(
                step_allocations(&mut sampler, 1000, 20_000),
                0,
                "{conditioning:?}"
            );
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
            assert_eq!(
                step_allocations(&mut sampler, 1000, 20_000),
                0,
                "multistate {conditioning:?}"
            );
            #[cfg(feature = "experimental-repeats")]
            {
                let partitions = nexus::parse(&repeat_matrix::matrix())
                    .unwrap()
                    .partition_by_states(GapTreatment::Reject)
                    .unwrap();
                for backend in rove::Backend::ALL.into_iter().filter(|b| b.is_available()) {
                    let mut sampler = Sampler::new(
                        &partitions,
                        &Settings {
                            backend,
                            monophyly: Vec::new(),
                            ..settings.clone()
                        },
                        11,
                    )
                    .unwrap();
                    assert_eq!(
                        step_allocations(&mut sampler, 1000, 2000),
                        0,
                        "repeat indices"
                    );
                }
            }
        }
    }
    let parts = nexus::parse(include_str!(
        "../fixtures/validation/informative/matrix.nex"
    ))
    .unwrap()
    .partition_by_states(GapTreatment::Reject)
    .unwrap();
    for base in [Settings::default(), Settings::reference()] {
        let settings = Settings {
            conditioning: Conditioning::Informative,
            ..base
        };
        let mut sampler = Sampler::new(&parts, &settings, 99).unwrap();
        assert_eq!(
            step_allocations(&mut sampler, 1000, 20_000),
            0,
            "informative conditioning"
        );
    }
    let mut dated = Settings::reference();
    dated.set_dating(rove::dating::Dating {
        process_priors: None,
        process: rove::dating::FossilizedBirthDeath {
            birth: 0.8,
            death: 0.3,
            fossil_sampling: 0.2,
            present_sampling: 0.7,
            origin: 4.0,
        },
        ages: (0..20)
            .map(|i| rove::dating::TipAge {
                minimum: if i % 2 == 0 { 0.0 } else { 0.4 },
                maximum: if i % 2 == 0 { 0.0 } else { 1.2 },
            })
            .collect(),
        clock_rate: 0.1,
        clock_prior_mean: Some(0.1),
    });
    for estimated in [false, true] {
        if estimated {
            let mut dating = dated.dating.clone().unwrap();
            dating.process_priors = Some(rove::dating::ProcessPriors {
                birth_rate_mean: Some(1.1),
                death_rate_mean: Some(0.6),
                fossil_sampling_rate_mean: Some(0.3),
                present_sampling_beta: Some((2.0, 3.0)),
                origin_excess_mean: Some(2.0),
            });
            dated.set_dating(dating);
        }
        let mut sampler = Sampler::new(&patterns, &dated, 17).unwrap();
        assert_eq!(
            step_allocations(&mut sampler, 100, 20_000),
            0,
            "dated relaxed clock; estimated process {estimated}"
        );
    }
    let ordered = nexus::parse(include_str!("../fixtures/validation/ordered/matrix.nex"))
        .unwrap()
        .partition_by_states(GapTreatment::Reject)
        .unwrap();
    for base in [Settings::default(), Settings::reference()] {
        for conditioning in [Conditioning::None, Conditioning::Variable] {
            let mut chain = Sampler::new(
                &ordered,
                &Settings {
                    conditioning,
                    ..base.clone()
                },
                5,
            )
            .unwrap();
            assert_eq!(
                step_allocations(&mut chain, 100, 2000),
                0,
                "ordered partitions"
            );
        }
    }
}
