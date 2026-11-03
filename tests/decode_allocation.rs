// SPDX-FileCopyrightText: 2026 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

// This binary holds a single test, because the global allocator below counts the allocations of
// every thread in the process.

use std::{
    alloc::{GlobalAlloc, Layout, System},
    sync::atomic::{AtomicUsize, Ordering},
};

use mimi_content::MimiContent;

/// Forwards to the system allocator and records the peak number of live bytes.
struct PeakTracking;

static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

// SAFETY: every call forwards to `System` with the caller's arguments unchanged.
unsafe impl GlobalAlloc for PeakTracking {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            let live = LIVE.fetch_add(layout.size(), Ordering::Relaxed) + layout.size();
            PEAK.fetch_max(live, Ordering::Relaxed);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
        LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
    }
}

#[global_allocator]
static ALLOCATOR: PeakTracking = PeakTracking;

/// Returns the peak number of bytes `f` held allocated at once, beyond what was live before.
fn peak_allocation<T>(f: impl FnOnce() -> T) -> (T, usize) {
    let baseline = LIVE.load(Ordering::Relaxed);
    PEAK.store(baseline, Ordering::Relaxed);
    let result = f();
    (result, PEAK.load(Ordering::Relaxed) - baseline)
}

#[test]
fn nested_declared_array_lengths_do_not_drive_allocation() {
    // A container whose only extension nests arrays declaring u64::MAX elements past the depth
    // limit, followed by 1 MiB of padding.
    let mut input = vec![0x87, 0x40, 0xf6, 0x40, 0xf6, 0xf6, 0xa1, 0x00];
    for _ in 0..32 {
        input.extend([0x9b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
    }
    input.resize(input.len() + (1 << 20), 0xf6);
    let budget = 1 << 20;

    let (result, peak) = peak_allocation(|| MimiContent::deserialize(&input));
    result.unwrap_err();
    assert!(peak < budget, "minicbor decoding allocated {peak} bytes");

    #[cfg(feature = "serde")]
    {
        let (result, peak) = peak_allocation(|| minicbor_serde::from_slice::<MimiContent>(&input));
        result.unwrap_err();
        assert!(peak < budget, "serde decoding allocated {peak} bytes");
    }
}
