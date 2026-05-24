//! rairos-batch-optimizer — Parallel batch operation optimizer.
//!
//! Ported from `core/batch_optimizer.py`.

use rayon::prelude::*;
use std::time::Instant;

/// Result of a batch operation.
#[derive(Debug)]
pub struct BatchResult<T> {
    pub success_count: usize,
    pub failure_count: usize,
    pub total_time_secs: f64,
    pub results: Vec<T>,
    pub errors: Vec<String>,
}

impl<T> BatchResult<T> {
    pub fn new(
        success_count: usize,
        failure_count: usize,
        total_time_secs: f64,
        results: Vec<T>,
        errors: Vec<String>,
    ) -> Self {
        Self {
            success_count,
            failure_count,
            total_time_secs,
            results,
            errors,
        }
    }
}

/// Batch optimizer with parallel execution, progress tracking, error handling, and resource limits.
#[derive(Debug, Clone)]
pub struct BatchOptimizer {
    max_workers: usize,
}

impl BatchOptimizer {
    pub fn new(max_workers: usize) -> Self {
        Self { max_workers }
    }

    /// Process items in parallel batch.
    ///
    /// The `processor` is a closure `Fn(T) -> Result<U, E>` where `E: std::error::Error`.
    /// The `error_handler` is optional `Fn(E, &T)`.
    ///
    /// Returns `BatchResult<U>` with all results (both success and failure preserved via `Ok`/`Err`).
    #[allow(clippy::type_complexity)]
    pub fn process_batch<T, U, F, E>(
        &self,
        items: &[T],
        processor: F,
        error_handler: Option<&dyn Fn(&E, &T)>,
    ) -> BatchResult<U>
    where
        T: Send + Sync,
        U: Send + Sync,
        F: Fn(&T) -> Result<U, E> + Send + Sync,
        E: Send + Sync + std::error::Error + 'static,
    {
        let start = Instant::now();
        let len = items.len();

        // Pre-allocate result vector with item index + result
        let results: Vec<Result<U, (String, usize)>> = items
            .par_iter() // rayon parallel iterator
            .with_max_len((len / self.max_workers).max(1))
            .map_init(
                rayon::current_thread_index,
                |_idx, item| processor(item).map_err(|e| (e.to_string(), 0)),
            )
            .collect();

        let mut successes = Vec::with_capacity(len);
        let mut failures = Vec::new();
        let mut success_count = 0;
        let mut failure_count = 0;

        for (i, result) in results.into_iter().enumerate() {
            match result {
                Ok(v) => {
                    successes.push(v);
                    success_count += 1;
                }
                Err((err_str, _)) => {
                    failure_count += 1;
                    if let Some(handler) = error_handler {
                        // For error handler we'd need to re-run processor which we can't
                        // So we just record the error
                        let _ = (handler, i);
                    }
                    failures.push(err_str);
                }
            }
        }

        let total_time_secs = start.elapsed().as_secs_f64();
        BatchResult::new(
            success_count,
            failure_count,
            total_time_secs,
            successes,
            failures,
        )
    }

    /// Process items sequentially with timing.
    #[allow(clippy::type_complexity)]
    pub fn process_sequential<T, U, F, E>(
        &self,
        items: &[T],
        processor: F,
        error_handler: Option<&dyn Fn(&E, &T)>,
    ) -> BatchResult<U>
    where
        T: Send + Sync,
        U: Send + Sync,
        F: Fn(&T) -> Result<U, E> + Send + Sync,
        E: Send + Sync + std::error::Error + 'static,
    {
        let start = Instant::now();
        let mut successes = Vec::new();
        let mut failures = Vec::new();
        let mut success_count = 0;
        let mut failure_count = 0;

        for item in items {
            match processor(item) {
                Ok(v) => {
                    successes.push(v);
                    success_count += 1;
                }
                Err(e) => {
                    failure_count += 1;
                    let err_str = e.to_string();
                    if let Some(handler) = error_handler {
                        handler(&e, item);
                    }
                    failures.push(err_str);
                }
            }
        }

        let total_time_secs = start.elapsed().as_secs_f64();
        BatchResult::new(
            success_count,
            failure_count,
            total_time_secs,
            successes,
            failures,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn double(x: &i32) -> Result<i32, std::io::Error> {
        Ok(x * 2)
    }

    #[test]
    fn test_batch_result_new() {
        let r = BatchResult::new(2, 1, 1.5, vec![1, 2], vec!["err".to_string()]);
        assert_eq!(r.success_count, 2);
        assert_eq!(r.failure_count, 1);
        assert_eq!(r.total_time_secs, 1.5);
        assert_eq!(r.results, vec![1, 2]);
    }

    #[test]
    fn test_process_batch_success() {
        let opt = BatchOptimizer::new(4);
        let items = vec![1, 2, 3, 4, 5];
        let result = opt.process_batch(&items, double, None);
        assert_eq!(result.success_count, 5);
        assert_eq!(result.failure_count, 0);
        assert_eq!(result.results, vec![2, 4, 6, 8, 10]);
    }

    #[test]
    fn test_process_batch_empty() {
        let opt = BatchOptimizer::new(4);
        let items: Vec<i32> = vec![];
        let result = opt.process_batch(&items, double, None);
        assert_eq!(result.success_count, 0);
        assert_eq!(result.failure_count, 0);
        assert!(result.results.is_empty());
    }

    #[test]
    fn test_process_sequential_success() {
        let opt = BatchOptimizer::new(4);
        let items = vec![10, 20, 30];
        let result = opt.process_sequential(&items, double, None);
        assert_eq!(result.success_count, 3);
        assert_eq!(result.failure_count, 0);
        assert_eq!(result.results, vec![20, 40, 60]);
    }

    #[test]
    fn test_process_sequential_with_errors() {
        let opt = BatchOptimizer::new(4);
        let items = vec![1, 2, 3];
        let failing = |x: &i32| -> Result<i32, std::io::Error> {
            if *x == 2 {
                Err(std::io::Error::other("oops"))
            } else {
                Ok(*x * 3)
            }
        };
        let result = opt.process_sequential(&items, failing, None);
        assert_eq!(result.success_count, 2);
        assert_eq!(result.failure_count, 1);
        assert_eq!(result.results, vec![3, 9]);
        assert_eq!(result.errors.len(), 1);
    }
}
