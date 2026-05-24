//! Mock Database for testing — implements DatabaseProvider trait.
//!
//! Example:
//! ```rust
//! use rairos_core::prelude::*;
//! let mock = MockDatabase::new();
//! let papers = mock.list_papers(None, 10, 0).unwrap();
//! assert!(papers.is_empty());
//! ```

use crate::prelude::*;
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
pub struct MockDatabase {
    papers: Arc<Mutex<Vec<Paper>>>,
    gaps: Arc<Mutex<Vec<ResearchGap>>>,
}

impl MockDatabase {
    pub fn new() -> Self {
        Self::default()
    }
}

impl DatabaseProvider for MockDatabase {
    fn insert_paper(&self, paper: &Paper) -> Result<()> {
        self.papers.lock().unwrap().push(paper.clone());
        Ok(())
    }
    fn get_paper(&self, id: &str) -> Result<Paper> {
        self.papers.lock().unwrap().iter()
            .find(|p| p.id == id)
            .cloned()
            .ok_or_else(|| CoreError::NotFound(id.to_string()))
    }
    fn get_paper_by_arxiv(&self, arxiv_id: &str) -> Result<Option<Paper>> {
        Ok(self.papers.lock().unwrap().iter()
            .find(|p| p.arxiv_id.as_deref() == Some(arxiv_id))
            .cloned())
    }
    fn list_papers(&self, _status: Option<ParseStatus>, _limit: usize, _offset: usize) -> Result<Vec<Paper>> {
        Ok(self.papers.lock().unwrap().clone())
    }
    fn delete_paper(&self, id: &str) -> Result<()> {
        self.papers.lock().unwrap().retain(|p| p.id != id);
        Ok(())
    }
    fn paper_exists(&self, id: &str) -> bool {
        self.papers.lock().unwrap().iter().any(|p| p.id == id)
    }
    fn search_papers(&self, _query: &str, _limit: usize) -> Result<Vec<Paper>> {
        Ok(self.papers.lock().unwrap().clone())
    }
    fn search_papers_smart(&self, _query: &str, _limit: usize) -> Result<Vec<Paper>> {
        Ok(self.papers.lock().unwrap().clone())
    }
    fn stats(&self) -> Result<DbStats> {
        let count = self.papers.lock().unwrap().len() as i64;
        Ok(DbStats { total: count, pending: 0, done: count, gaps: 0 })
    }
    fn list_gaps(&self, _limit: usize, _offset: usize) -> Result<Vec<ResearchGap>> {
        Ok(self.gaps.lock().unwrap().clone())
    }
    fn insert_gap(&self, gap: &ResearchGap) -> Result<()> {
        self.gaps.lock().unwrap().push(gap.clone());
        Ok(())
    }
    fn merge_papers(&self, _primary_id: &str, _duplicate_ids: &[&str]) -> Result<bool> {
        Ok(false)
    }
    fn update_paper_status(&self, _id: &str, _status: ParseStatus) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_insert_and_get() {
        let mock = MockDatabase::new();
        let paper = Paper::new(None, "Test Title".into(), "Test abstract".into());
        mock.insert_paper(&paper).unwrap();
        
        let fetched = mock.get_paper(&paper.id).unwrap();
        assert_eq!(fetched.title, "Test Title");
        assert!(mock.paper_exists(&paper.id));
    }

    #[test]
    fn test_mock_stats() {
        let mock = MockDatabase::new();
        let stats = mock.stats().unwrap();
        assert_eq!(stats.total, 0);
        
        let paper = Paper::new(None, "Title".into(), "Abstract".into());
        mock.insert_paper(&paper).unwrap();
        let stats = mock.stats().unwrap();
        assert_eq!(stats.total, 1);
    }

    #[test]
    fn test_mock_delete_nonexistent() {
        let mock = MockDatabase::new();
        mock.delete_paper("nonexistent").unwrap();
        let stats = mock.stats().unwrap();
        assert_eq!(stats.total, 0);
    }
}
