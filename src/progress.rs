//! Progress and cancelling for a file being opened on a worker thread. The worker installs a
//! `Job` with `install`; the readers call `report`, `stage` and `cancelled` without needing to
//! know whether anyone is watching (called from the window's own thread they do nothing).

use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering::Relaxed};
use std::sync::Arc;

/// What the worker is doing now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Stage {
    Reading = 0,
    Unpacking = 1,
    Values = 2,
    Showing = 3,
}

#[derive(Default, Debug)]
pub struct Job {
    stage: AtomicU8,
    done: AtomicU64,
    total: AtomicU64,
    cancel: AtomicBool,
}

impl Job {
    pub fn new() -> Arc<Job> {
        Arc::new(Job::default())
    }
    pub fn cancel(&self) {
        self.cancel.store(true, Relaxed);
    }
    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Relaxed)
    }
    pub fn current_stage(&self) -> Stage {
        match self.stage.load(Relaxed) {
            0 => Stage::Reading,
            1 => Stage::Unpacking,
            2 => Stage::Values,
            _ => Stage::Showing,
        }
    }
    /// How far the stage is, 0 to 1, or `None` when it can't tell.
    pub fn fraction(&self) -> Option<f32> {
        let t = self.total.load(Relaxed);
        (t > 0).then(|| (self.done.load(Relaxed) as f64 / t as f64).clamp(0.0, 1.0) as f32)
    }
}

thread_local! {
    static CURRENT: RefCell<Option<Arc<Job>>> = const { RefCell::new(None) };
}

/// Makes `job` the one this thread reports to until the guard is dropped.
pub fn install(job: Arc<Job>) -> Guard {
    CURRENT.with(|c| *c.borrow_mut() = Some(job));
    Guard
}

pub struct Guard;

impl Drop for Guard {
    fn drop(&mut self) {
        CURRENT.with(|c| *c.borrow_mut() = None);
    }
}

fn with(f: impl FnOnce(&Job)) {
    CURRENT.with(|c| {
        if let Some(j) = &*c.borrow() {
            f(j)
        }
    });
}

/// A new stage begins; its progress starts again unknown.
pub fn stage(s: Stage) {
    with(|j| {
        j.stage.store(s as u8, Relaxed);
        j.done.store(0, Relaxed);
        j.total.store(0, Relaxed);
    });
}

/// `done` of `total` units of the stage are finished.
pub fn report(done: u64, total: u64) {
    with(|j| {
        j.done.store(done, Relaxed);
        j.total.store(total, Relaxed);
    });
}

/// The player pressed Cancel: the reader should stop and give up.
pub fn cancelled() -> bool {
    CURRENT.with(|c| c.borrow().as_ref().is_some_and(|j| j.is_cancelled()))
}

/// Reads a file in pieces, reporting how far it is and giving up when cancelled.
#[cfg(not(target_arch = "wasm32"))]
pub fn read_file(p: &std::path::Path) -> std::io::Result<Vec<u8>> {
    use std::io::Read;
    let mut f = std::fs::File::open(p)?;
    let total = f.metadata().map(|m| m.len()).unwrap_or(0);
    let mut out = Vec::with_capacity(total.min(1 << 32) as usize);
    let mut buf = vec![0u8; 4 << 20];
    stage(Stage::Reading);
    loop {
        if cancelled() {
            return Err(std::io::Error::new(std::io::ErrorKind::Interrupted, "cancelled"));
        }
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        out.extend_from_slice(&buf[..n]);
        report(out.len() as u64, total);
    }
    Ok(out)
}

thread_local! {
    static TOO_BIG: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) };
}

/// A reader gave up on a save of `len` bytes because its tree wouldn't fit in the free memory.
pub fn note_too_big(len: usize) {
    TOO_BIG.with(|c| c.set(Some(len)));
}

/// The size of the save a reader gave up on since the last call, if there was one.
pub fn take_too_big() -> Option<usize> {
    TOO_BIG.with(|c| c.take())
}

/// The browser version keeps files in memory and has no worker threads.
#[cfg(target_arch = "wasm32")]
pub fn read_file(p: &std::path::Path) -> std::io::Result<Vec<u8>> {
    crate::vfs::read(p)
}
