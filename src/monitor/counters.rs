//! Track counters.

use std::fmt::{self, Debug};
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::Relaxed;

use strum::{EnumCount, IntoEnumIterator};
use strum_macros::{EnumCount, EnumIter};

#[derive(Debug, Clone, Copy, Eq, PartialEq, EnumCount, EnumIter)]
pub enum Counter {
    BandsDone,
    BandsTotal,
    FilesDone,
    IndexBytesDone,
    BlockBytesDone,
    BlockRead,
    BlockWrite,
    BlockMatchExisting,
    BlockCacheHit,
    // ...
}
#[derive(Default)]
pub struct Counters {
    counters: [AtomicUsize; Counter::COUNT],
}

impl Counters {
    pub fn count(&self, counter: Counter, increment: usize) {
        self.counters[counter as usize].fetch_add(increment, Relaxed);
    }

    pub fn set(&self, counter: Counter, value: usize) {
        self.counters[counter as usize].store(value, Relaxed);
    }

    pub fn get(&self, counter: Counter) -> usize {
        self.counters[counter as usize].load(Relaxed)
    }
}

impl Debug for Counters {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = f.debug_struct("Counters");
        for i in Counter::iter() {
            s.field(
                &format!("{:?}", i),
                &self.counters[i as usize].load(Relaxed),
            );
        }
        s.finish()
    }
}
