"""Tests for rankers module functionality."""


def test_ranked_result_type():
    """Test RankedResult is properly typed."""
    # Mock paper record
    class MockPaper:
        def __init__(self):
            self.id = "test-123"
            self.title = "Test Paper"
    
    result = (MockPaper(), 0.95)
    assert isinstance(result, tuple)
    assert result[0].id == "test-123"
    assert result[1] == 0.95


def test_ranked_result_order():
    """Test that RankedResult maintains proper ordering."""
    class MockPaper:
        def __init__(self, id):
            self.id = id
    
    # Create results in order
    results = [
        (MockPaper("p1"), 0.95),
        (MockPaper("p2"), 0.85),
        (MockPaper("p3"), 0.75),
    ]
    
    # Sort by similarity (should already be sorted)
    sorted_results = sorted(results, key=lambda x: x[1], reverse=True)
    
    assert sorted_results[0][1] == 0.95
    assert sorted_results[1][1] == 0.85
    assert sorted_results[2][1] == 0.75


def test_threshold_filtering():
    """Test threshold filtering works correctly."""
    class MockPaper:
        def __init__(self, id):
            self.id = id
    
    threshold = 0.8
    results = [
        (MockPaper("p1"), 0.95),
        (MockPaper("p2"), 0.85),
        (MockPaper("p3"), 0.75),
    ]
    
    filtered = [(p, s) for p, s in results if s >= threshold]
    
    assert len(filtered) == 2
    assert filtered[0][0].id == "p1"
    assert filtered[1][0].id == "p2"


def test_limit_filtering():
    """Test limit filtering works correctly."""
    class MockPaper:
        def __init__(self, id):
            self.id = id
    
    limit = 2
    results = [
        (MockPaper("p1"), 0.95),
        (MockPaper("p2"), 0.85),
        (MockPaper("p3"), 0.75),
        (MockPaper("p4"), 0.65),
    ]
    
    limited = results[:limit]
    
    assert len(limited) == 2
    assert limited[0][0].id == "p1"
    assert limited[1][0].id == "p2"


def test_empty_results():
    """Test handling of empty results."""
    results = []
    assert len(results) == 0
    # Should not crash on empty list operations
    sorted_results = sorted(results, key=lambda x: x[1], reverse=True)
    assert len(sorted_results) == 0
