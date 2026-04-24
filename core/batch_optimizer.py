"""
Batch Operation Optimizer.

Optimizes batch operations for efficiency.
"""
from typing import List, Callable, Any, Optional
from concurrent.futures import ThreadPoolExecutor, as_completed
from dataclasses import dataclass
import time


@dataclass
class BatchResult:
    """Result of a batch operation."""
    success_count: int
    failure_count: int
    total_time: float
    results: List[Any]
    errors: List[str]


class BatchOptimizer:
    """
    Optimize batch operations.
    
    Features:
    - Parallel execution
    - Progress tracking
    - Error handling
    - Resource limits
    """

    def __init__(self, max_workers: int = 4):
        self.max_workers = max_workers

    def process_batch(
        self,
        items: List[Any],
        processor: Callable[[Any], Any],
        error_handler: Optional[Callable] = None
    ) -> BatchResult:
        """Process items in batch with parallel execution."""
        start_time = time.time()
        results = []
        errors = []
        success_count = 0
        failure_count = 0

        with ThreadPoolExecutor(max_workers=self.max_workers) as executor:
            futures = {executor.submit(processor, item): item for item in items}

            for future in as_completed(futures):
                item = futures[future]
                try:
                    result = future.result()
                    results.append(result)
                    success_count += 1
                except Exception as e:
                    failure_count += 1
                    errors.append(str(e))
                    if error_handler:
                        error_handler(e, item)

        total_time = time.time() - start_time

        return BatchResult(
            success_count=success_count,
            failure_count=failure_count,
            total_time=total_time,
            results=results,
            errors=errors
        )

    def process_sequential(
        self,
        items: List[Any],
        processor: Callable[[Any], Any],
        error_handler: Optional[Callable] = None
    ) -> BatchResult:
        """Process items sequentially with timing."""
        start_time = time.time()
        results = []
        errors = []
        success_count = 0
        failure_count = 0

        for item in items:
            try:
                result = processor(item)
                results.append(result)
                success_count += 1
            except Exception as e:
                failure_count += 1
                errors.append(str(e))
                if error_handler:
                    error_handler(e, item)

        total_time = time.time() - start_time

        return BatchResult(
            success_count=success_count,
            failure_count=failure_count,
            total_time=total_time,
            results=results,
            errors=errors
        )
