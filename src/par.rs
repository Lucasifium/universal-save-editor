//! Spreading independent pieces of work (compressed chunks, encrypted slices) over the CPU's
//! cores.

/// `f` applied to every item, using up to all cores; results stay in order.
pub fn map<T: Sync, R: Send>(items: &[T], f: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get()).min(items.len()).max(1);
    if threads == 1 {
        return items.iter().map(f).collect();
    }
    let per = items.len().div_ceil(threads);
    std::thread::scope(|s| {
        let handles: Vec<_> = items.chunks(per).map(|part| s.spawn(|| part.iter().map(&f).collect::<Vec<R>>())).collect();
        handles.into_iter().flat_map(|h| h.join().expect("worker thread panicked")).collect()
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn keeps_order() {
        let v: Vec<u32> = (0..1000).collect();
        assert_eq!(super::map(&v, |x| x * 2), v.iter().map(|x| x * 2).collect::<Vec<_>>());
        assert!(super::map(&Vec::<u32>::new(), |x| *x).is_empty());
    }
}
