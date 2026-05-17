//! Comprehensive database tests for rairos-core
//!
//! Tests cover: init, upsert_paper roundtrip, get_papers_bulk, tag operations,
//! FTS search, job queue (subscriptions), settings (stats), paper_count,
//! paper_exists, citations, experiment_tables (research_gaps)

use rairos_core::{Database, Paper, PaperMetadata, ParseStatus, Subscription, Tag};
use tempfile::TempDir;

fn create_test_db() -> (Database, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");
    let db = Database::open(&db_path).expect("Failed to open database");
    (db, temp_dir)
}

fn create_test_paper(arxiv_id: Option<&str>, title: &str) -> Paper {
    Paper::with_metadata(
        arxiv_id.map(String::from),
        title.to_string(),
        format!("Abstract for {}", title),
        vec!["Author One".to_string(), "Author Two".to_string()],
        vec!["cs.AI".to_string(), "cs.LG".to_string()],
        PaperMetadata {
            cited_by: 10,
            references: 5,
            doi: Some("10.1234/test".to_string()),
            pdf_url: Some("https://example.com/paper.pdf".to_string()),
        },
    )
}

// ============================================================================
// Test: init
// ============================================================================

#[test]
fn test_db_init_creates_tables() {
    let (db, _temp_dir) = create_test_db();
    let count = db.count_papers().expect("count_papers should work");
    assert_eq!(count, 0, "New database should have 0 papers");
}

#[test]
fn test_db_init_creates_fts_index() {
    let (db, _temp_dir) = create_test_db();
    // FTS is created via trigger, search should work
    let results = db
        .search_papers_fts("test", 10)
        .expect("search_papers_fts should work");
    assert!(
        results.is_empty(),
        "Empty FTS search should return empty vec"
    );
}

// ============================================================================
// Test: upsert_paper roundtrip
// ============================================================================

#[test]
fn test_insert_paper_roundtrip() {
    let (db, _temp_dir) = create_test_db();
    let paper = create_test_paper(Some("2301.00001"), "Test Paper Title");
    let paper_id = paper.id.clone();

    // Insert
    db.insert_paper(&paper)
        .expect("insert_paper should succeed");

    // Retrieve by id
    let retrieved = db.get_paper(&paper_id).expect("get_paper should succeed");
    assert_eq!(retrieved.id, paper_id);
    assert_eq!(retrieved.title, "Test Paper Title");
    assert_eq!(retrieved.arxiv_id, Some("2301.00001".to_string()));
    assert_eq!(retrieved.authors.len(), 2);
    assert_eq!(retrieved.parse_status, ParseStatus::Pending);
    assert_eq!(retrieved.metadata.cited_by, 10);
}

#[test]
fn test_insert_paper_by_arxiv_roundtrip() {
    let (db, _temp_dir) = create_test_db();
    let paper = create_test_paper(Some("2301.99999"), "arXiv Paper");
    let arxiv_id = paper.arxiv_id.clone().unwrap();

    db.insert_paper(&paper)
        .expect("insert_paper should succeed");

    // Retrieve by arxiv_id
    let retrieved = db
        .get_paper_by_arxiv(&arxiv_id)
        .expect("get_paper_by_arxiv should succeed")
        .expect("Paper should be found");
    assert_eq!(retrieved.arxiv_id, Some("2301.99999".to_string()));
    assert_eq!(retrieved.title, "arXiv Paper");
}

#[test]
fn test_update_paper_status() {
    let (db, _temp_dir) = create_test_db();
    let paper = create_test_paper(Some("2301.00002"), "Status Test");
    let paper_id = paper.id.clone();

    db.insert_paper(&paper)
        .expect("insert_paper should succeed");

    // Update status
    db.update_paper_status(&paper_id, ParseStatus::Done)
        .expect("update_paper_status should succeed");

    // Verify
    let retrieved = db.get_paper(&paper_id).expect("get_paper should succeed");
    assert_eq!(retrieved.parse_status, ParseStatus::Done);
}

#[test]
fn test_delete_paper() {
    let (db, _temp_dir) = create_test_db();
    let paper = create_test_paper(Some("2301.00003"), "Delete Me");
    let paper_id = paper.id.clone();

    db.insert_paper(&paper)
        .expect("insert_paper should succeed");
    db.delete_paper(&paper_id)
        .expect("delete_paper should succeed");

    let result = db.get_paper(&paper_id);
    assert!(result.is_err(), "Deleted paper should not be found");
}

// ============================================================================
// Test: get_papers_bulk (list_papers)
// ============================================================================

#[test]
fn test_list_papers_empty() {
    let (db, _temp_dir) = create_test_db();
    let papers = db
        .list_papers(None, 10, 0)
        .expect("list_papers should work");
    assert!(papers.is_empty(), "Empty DB should return empty vec");
}

#[test]
fn test_list_papers_bulk() {
    let (db, _temp_dir) = create_test_db();

    // Insert 15 papers
    for i in 0..15 {
        let paper = create_test_paper(Some(&format!("2301.{:05}", i)), &format!("Paper {}", i));
        db.insert_paper(&paper)
            .expect("insert_paper should succeed");
    }

    // List all with limit
    let papers = db
        .list_papers(None, 10, 0)
        .expect("list_papers should work");
    assert_eq!(papers.len(), 10, "Should return up to limit papers");

    // List next page
    let papers_page2 = db
        .list_papers(None, 10, 10)
        .expect("list_papers page 2 should work");
    assert_eq!(papers_page2.len(), 5, "Should return remaining papers");
}

#[test]
fn test_list_papers_by_status() {
    let (db, _temp_dir) = create_test_db();

    // Insert papers with different statuses
    for i in 0..5 {
        let mut paper = create_test_paper(
            Some(&format!("2301.00{:02}", i)),
            &format!("Pending Paper {}", i),
        );
        paper.parse_status = ParseStatus::Pending;
        db.insert_paper(&paper)
            .expect("insert_paper should succeed");
    }
    for i in 5..10 {
        let mut paper = create_test_paper(
            Some(&format!("2301.01{:02}", i)),
            &format!("Done Paper {}", i),
        );
        paper.parse_status = ParseStatus::Done;
        db.insert_paper(&paper)
            .expect("insert_paper should succeed");
    }

    // Filter by status
    let pending = db
        .list_papers(Some(ParseStatus::Pending), 10, 0)
        .expect("list_papers with status should work");
    assert_eq!(pending.len(), 5, "Should have 5 pending papers");

    let done = db
        .list_papers(Some(ParseStatus::Done), 10, 0)
        .expect("list_papers with status should work");
    assert_eq!(done.len(), 5, "Should have 5 done papers");
}

// ============================================================================
// Test: Tag operations
// ============================================================================

#[test]
fn test_insert_tag() {
    let (db, _temp_dir) = create_test_db();
    let tag = Tag::new("machine-learning");

    db.insert_tag(&tag).expect("insert_tag should succeed");

    let tags = db.list_tags().expect("list_tags should work");
    assert_eq!(tags.len(), 1, "Should have 1 tag");
    assert_eq!(tags[0].name, "machine-learning");
}

#[test]
fn test_list_tags_multiple() {
    let (db, _temp_dir) = create_test_db();

    let tag1 = Tag::new("alpha");
    let tag2 = Tag::new("beta");
    let tag3 = Tag::new("gamma");

    db.insert_tag(&tag1).expect("insert_tag should succeed");
    db.insert_tag(&tag2).expect("insert_tag should succeed");
    db.insert_tag(&tag3).expect("insert_tag should succeed");

    let tags = db.list_tags().expect("list_tags should work");
    assert_eq!(tags.len(), 3, "Should have 3 tags");
    // Tags should be sorted by name
    assert_eq!(tags[0].name, "alpha");
    assert_eq!(tags[1].name, "beta");
    assert_eq!(tags[2].name, "gamma");
}

#[test]
fn test_add_paper_tag() {
    let (db, _temp_dir) = create_test_db();
    let paper = create_test_paper(Some("2301.10001"), "Tagged Paper");
    let paper_id = paper.id.clone();

    db.insert_paper(&paper)
        .expect("insert_paper should succeed");

    let tag = Tag::new("important");
    let tag_id = tag.id.clone();
    db.insert_tag(&tag).expect("insert_tag should succeed");

    db.add_paper_tag(&paper_id, &tag_id)
        .expect("add_paper_tag should succeed");

    let tags = db
        .get_paper_tags(&paper_id)
        .expect("get_paper_tags should work");
    assert_eq!(tags.len(), 1, "Paper should have 1 tag");
    assert_eq!(tags[0].name, "important");
}

#[test]
fn test_remove_paper_tag() {
    let (db, _temp_dir) = create_test_db();
    let paper = create_test_paper(Some("2301.10002"), "Untag Paper");
    let paper_id = paper.id.clone();

    db.insert_paper(&paper)
        .expect("insert_paper should succeed");

    let tag = Tag::new("to-remove");
    let tag_id = tag.id.clone();
    db.insert_tag(&tag).expect("insert_tag should succeed");

    db.add_paper_tag(&paper_id, &tag_id)
        .expect("add_paper_tag should succeed");
    db.remove_paper_tag(&paper_id, &tag_id)
        .expect("remove_paper_tag should succeed");

    let tags = db
        .get_paper_tags(&paper_id)
        .expect("get_paper_tags should work");
    assert!(tags.is_empty(), "Paper should have no tags after removal");
}

#[test]
fn test_delete_tag() {
    let (db, _temp_dir) = create_test_db();
    let tag = Tag::new("delete-me");
    let tag_id = tag.id.clone();

    db.insert_tag(&tag).expect("insert_tag should succeed");
    db.delete_tag(&tag_id).expect("delete_tag should succeed");

    let tags = db.list_tags().expect("list_tags should work");
    assert!(tags.is_empty(), "Tag should be deleted");
}

// ============================================================================
// Test: FTS search
// ============================================================================

#[test]
fn test_search_papers_fts_basic() {
    let (db, _temp_dir) = create_test_db();

    let paper1 = create_test_paper(Some("2301.20001"), "Deep Learning for Images");
    let paper2 = create_test_paper(Some("2301.20002"), "Natural Language Processing");
    let paper3 = create_test_paper(Some("2301.20003"), "Reinforcement Learning");

    db.insert_paper(&paper1)
        .expect("insert_paper should succeed");
    db.insert_paper(&paper2)
        .expect("insert_paper should succeed");
    db.insert_paper(&paper3)
        .expect("insert_paper should succeed");

    let results = db
        .search_papers_fts("deep learning", 10)
        .expect("search_papers_fts should work");
    assert_eq!(results.len(), 1, "Should find 1 paper about deep learning");
    assert!(results[0].title.contains("Deep Learning"));
}

#[test]
fn test_search_papers_fts_multiple_terms() {
    let (db, _temp_dir) = create_test_db();

    let paper = create_test_paper(Some("2301.20004"), "Transformers for Machine Translation");
    db.insert_paper(&paper)
        .expect("insert_paper should succeed");

    let results = db
        .search_papers_fts("transformers translation", 10)
        .expect("search_papers_fts should work");
    assert!(!results.is_empty(), "Should find papers matching terms");
}

#[test]
fn test_search_papers_fts_abstract() {
    let (db, _temp_dir) = create_test_db();

    let paper = create_test_paper(Some("2301.20005"), "Novel Approach Paper");
    let _paper_id = paper.id.clone();
    db.insert_paper(&paper)
        .expect("insert_paper should succeed");

    let results = db
        .search_papers_fts("Abstract for Novel Approach Paper", 10)
        .expect("search_papers_fts should work");
    assert!(!results.is_empty(), "Should find paper by abstract text");
}

#[test]
fn test_search_papers_fts_no_results() {
    let (db, _temp_dir) = create_test_db();

    let paper = create_test_paper(Some("2301.20006"), "Specific Topic Paper");
    db.insert_paper(&paper)
        .expect("insert_paper should succeed");

    let results = db
        .search_papers_fts("nonexistent query xyz123", 10)
        .expect("search_papers_fts should work");
    assert!(results.is_empty(), "Should return empty for no matches");
}

#[test]
fn test_search_papers_fts_limit() {
    let (db, _temp_dir) = create_test_db();

    for i in 0..20 {
        let paper = create_test_paper(
            Some(&format!("2301.300{:02}", i)),
            &format!("Machine Learning Paper {}", i),
        );
        db.insert_paper(&paper)
            .expect("insert_paper should succeed");
    }

    let results = db
        .search_papers_fts("machine learning", 5)
        .expect("search_papers_fts should work");
    assert_eq!(results.len(), 5, "Should respect limit parameter");
}

// ============================================================================
// Test: Job queue (Subscriptions)
// ============================================================================

#[test]
fn test_insert_subscription() {
    let (db, _temp_dir) = create_test_db();
    let sub = Subscription::new("transformer attention");

    db.insert_subscription(&sub)
        .expect("insert_subscription should succeed");

    let subs = db
        .list_subscriptions(false)
        .expect("list_subscriptions should work");
    assert_eq!(subs.len(), 1, "Should have 1 subscription");
    assert_eq!(subs[0].query, "transformer attention");
}

#[test]
fn test_list_subscriptions_enabled_only() {
    let (db, _temp_dir) = create_test_db();

    let sub1 = Subscription::new("enabled-query");
    let mut sub2 = Subscription::new("disabled-query");
    sub2.enabled = false;

    db.insert_subscription(&sub1)
        .expect("insert_subscription should succeed");
    db.insert_subscription(&sub2)
        .expect("insert_subscription should succeed");

    let all = db
        .list_subscriptions(false)
        .expect("list_subscriptions should work");
    assert_eq!(all.len(), 2, "Should have 2 total subscriptions");

    let enabled = db
        .list_subscriptions(true)
        .expect("list_subscriptions enabled_only should work");
    assert_eq!(enabled.len(), 1, "Should have 1 enabled subscription");
}

#[test]
fn test_delete_subscription() {
    let (db, _temp_dir) = create_test_db();
    let sub = Subscription::new("to-delete");
    let sub_id = sub.id.clone();

    db.insert_subscription(&sub)
        .expect("insert_subscription should succeed");
    db.delete_subscription(&sub_id)
        .expect("delete_subscription should succeed");

    let subs = db
        .list_subscriptions(false)
        .expect("list_subscriptions should work");
    assert!(subs.is_empty(), "Subscription should be deleted");
}

#[test]
fn test_update_subscription_last_check() {
    let (db, _temp_dir) = create_test_db();
    let sub = Subscription::new("check-test");
    let sub_id = sub.id.clone();

    db.insert_subscription(&sub)
        .expect("insert_subscription should succeed");
    db.update_subscription_last_check(&sub_id, "2024-01-01T00:00:00Z", "1 result")
        .expect("update_subscription_last_check should succeed");

    let subs = db
        .list_subscriptions(false)
        .expect("list_subscriptions should work");
    assert_eq!(subs[0].last_check, Some("2024-01-01T00:00:00Z".to_string()));
    assert_eq!(subs[0].last_results, Some("1 result".to_string()));
}

// ============================================================================
// Test: Settings (stats)
// ============================================================================

#[test]
fn test_stats_empty_db() {
    let (db, _temp_dir) = create_test_db();
    let stats = db.stats().expect("stats should work");

    assert_eq!(stats.total, 0, "Total papers should be 0");
    assert_eq!(stats.pending, 0, "Pending papers should be 0");
    assert_eq!(stats.done, 0, "Done papers should be 0");
    assert_eq!(stats.gaps, 0, "Gaps should be 0");
}

#[test]
fn test_stats_with_papers() {
    let (db, _temp_dir) = create_test_db();

    // Add pending papers
    for i in 0..3 {
        let mut paper = create_test_paper(
            Some(&format!("2301.400{:02}", i)),
            &format!("Pending {}", i),
        );
        paper.parse_status = ParseStatus::Pending;
        db.insert_paper(&paper)
            .expect("insert_paper should succeed");
    }

    // Add done papers
    for i in 3..7 {
        let mut paper =
            create_test_paper(Some(&format!("2301.400{:02}", i)), &format!("Done {}", i));
        paper.parse_status = ParseStatus::Done;
        db.insert_paper(&paper)
            .expect("insert_paper should succeed");
    }

    let stats = db.stats().expect("stats should work");
    assert_eq!(stats.total, 7, "Total should be 7 papers");
    assert_eq!(stats.pending, 3, "Pending should be 3");
    assert_eq!(stats.done, 4, "Done should be 4");
}

// ============================================================================
// Test: paper_count
// ============================================================================

#[test]
fn test_count_papers_empty() {
    let (db, _temp_dir) = create_test_db();
    let count = db.count_papers().expect("count_papers should work");
    assert_eq!(count, 0, "Empty DB should have 0 papers");
}

#[test]
fn test_count_papers_after_insert() {
    let (db, _temp_dir) = create_test_db();

    for i in 0..5 {
        let paper = create_test_paper(
            Some(&format!("2301.500{:02}", i)),
            &format!("Count Test {}", i),
        );
        db.insert_paper(&paper)
            .expect("insert_paper should succeed");
    }

    let count = db.count_papers().expect("count_papers should work");
    assert_eq!(count, 5, "Should have 5 papers");
}

#[test]
fn test_count_papers_after_delete() {
    let (db, _temp_dir) = create_test_db();

    let paper1 = create_test_paper(Some("2301.50010"), "Delete Test 1");
    let paper2 = create_test_paper(Some("2301.50011"), "Delete Test 2");
    let paper1_id = paper1.id.clone();

    db.insert_paper(&paper1)
        .expect("insert_paper should succeed");
    db.insert_paper(&paper2)
        .expect("insert_paper should succeed");

    assert_eq!(db.count_papers().expect("count_papers should work"), 2);

    db.delete_paper(&paper1_id)
        .expect("delete_paper should succeed");
    assert_eq!(db.count_papers().expect("count_papers should work"), 1);
}

// ============================================================================
// Test: paper_exists (via get_paper_by_arxiv)
// ============================================================================

#[test]
fn test_paper_exists_by_arxiv() {
    let (db, _temp_dir) = create_test_db();
    let paper = create_test_paper(Some("2301.60001"), "Existence Test");
    let arxiv_id = paper.arxiv_id.clone().unwrap();

    db.insert_paper(&paper)
        .expect("insert_paper should succeed");

    let found = db
        .get_paper_by_arxiv(&arxiv_id)
        .expect("get_paper_by_arxiv should work");
    assert!(found.is_some(), "Paper should exist by arxiv_id");
}

#[test]
fn test_paper_not_exists_by_arxiv() {
    let (db, _temp_dir) = create_test_db();

    let found = db
        .get_paper_by_arxiv("nonexistent.arxiv")
        .expect("get_paper_by_arxiv should work");
    assert!(found.is_none(), "Paper should not exist");
}

#[test]
fn test_paper_exists_by_id() {
    let (db, _temp_dir) = create_test_db();
    let paper = create_test_paper(Some("2301.60002"), "ID Existence Test");
    let paper_id = paper.id.clone();

    db.insert_paper(&paper)
        .expect("insert_paper should succeed");

    let result = db.get_paper(&paper_id);
    assert!(result.is_ok(), "Paper should exist by id");
}

// ============================================================================
// Test: Citations
// ============================================================================

#[test]
fn test_insert_citation() {
    let (db, _temp_dir) = create_test_db();
    let paper1 = create_test_paper(Some("2301.70001"), "Citing Paper");
    let paper2 = create_test_paper(Some("2301.70002"), "Cited Paper");
    let paper1_id = paper1.id.clone();
    let paper2_id = paper2.id.clone();

    db.insert_paper(&paper1)
        .expect("insert_paper should succeed");
    db.insert_paper(&paper2)
        .expect("insert_paper should succeed");

    db.insert_citation(&paper1_id, &paper2_id)
        .expect("insert_citation should succeed");

    let citations = db
        .get_citations(&paper2_id)
        .expect("get_citations should work");
    assert_eq!(
        citations.citing.len(),
        1,
        "paper2 should have 1 citing paper"
    );
    assert_eq!(citations.citing[0], paper1_id, "Citing paper should match");
}

#[test]
fn test_get_citations_references() {
    let (db, _temp_dir) = create_test_db();
    let paper1 = create_test_paper(Some("2301.70003"), "Source Paper");
    let paper2 = create_test_paper(Some("2301.70004"), "Target Paper");
    let paper1_id = paper1.id.clone();
    let paper2_id = paper2.id.clone();

    db.insert_paper(&paper1)
        .expect("insert_paper should succeed");
    db.insert_paper(&paper2)
        .expect("insert_paper should succeed");

    // paper1 references paper2
    db.insert_citation(&paper1_id, &paper2_id)
        .expect("insert_citation should succeed");

    let citations = db
        .get_citations(&paper1_id)
        .expect("get_citations should work");
    assert_eq!(
        citations.references.len(),
        1,
        "paper1 should have 1 reference"
    );
    assert_eq!(citations.references[0], paper2_id, "Reference should match");
}

#[test]
fn test_get_citations_bidirectional() {
    let (db, _temp_dir) = create_test_db();
    let paper1 = create_test_paper(Some("2301.70005"), "Paper A");
    let paper2 = create_test_paper(Some("2301.70006"), "Paper B");
    let paper1_id = paper1.id.clone();
    let paper2_id = paper2.id.clone();

    db.insert_paper(&paper1)
        .expect("insert_paper should succeed");
    db.insert_paper(&paper2)
        .expect("insert_paper should succeed");

    // A cites B
    db.insert_citation(&paper1_id, &paper2_id)
        .expect("insert_citation should succeed");
    // B cites A
    db.insert_citation(&paper2_id, &paper1_id)
        .expect("insert_citation should succeed");

    let citations_a = db
        .get_citations(&paper1_id)
        .expect("get_citations should work");
    assert_eq!(citations_a.citing.len(), 1, "A should be cited by B");
    assert_eq!(citations_a.references.len(), 1, "A should reference B");

    let citations_b = db
        .get_citations(&paper2_id)
        .expect("get_citations should work");
    assert_eq!(citations_b.citing.len(), 1, "B should be cited by A");
    assert_eq!(citations_b.references.len(), 1, "B should reference A");
}

#[test]
fn test_insert_citation_duplicate_no_error() {
    let (db, _temp_dir) = create_test_db();
    let paper1 = create_test_paper(Some("2301.70007"), "Dup Test 1");
    let paper2 = create_test_paper(Some("2301.70008"), "Dup Test 2");
    let paper1_id = paper1.id.clone();
    let paper2_id = paper2.id.clone();

    db.insert_paper(&paper1)
        .expect("insert_paper should succeed");
    db.insert_paper(&paper2)
        .expect("insert_paper should succeed");

    // Insert same citation twice (should not error due to OR IGNORE)
    db.insert_citation(&paper1_id, &paper2_id)
        .expect("insert_citation should succeed first time");
    db.insert_citation(&paper1_id, &paper2_id)
        .expect("insert_citation should succeed second time (no error)");

    let citations = db
        .get_citations(&paper2_id)
        .expect("get_citations should work");
    assert_eq!(
        citations.citing.len(),
        1,
        "Should still only have 1 citation"
    );
}

// ============================================================================
// Test: Experiment tables (research_gaps)
// Fixed: insert_gap, list_gaps, get_gap now match the full schema.
// The ResearchGap struct includes: id, topic, session_id, gap_type,
// gap_title, gap_title_hash, category, description, severity, novelty_score,
// priority, paper_ids, created_at.
// ============================================================================

#[test]
fn test_delete_gap_nonexistent() {
    let (db, _temp_dir) = create_test_db();
    // Deleting non-existent gap should not error
    db.delete_gap("nonexistent-id")
        .expect("delete_gap should not error");
}
// Test: Update paper full text
// ============================================================================

#[test]
fn test_update_paper_full_text() {
    let (db, _temp_dir) = create_test_db();
    let paper = create_test_paper(Some("2301.90001"), "Full Text Test");
    let paper_id = paper.id.clone();

    db.insert_paper(&paper)
        .expect("insert_paper should succeed");

    db.update_paper_full_text(&paper_id, "This is the plain text content", 5, 3, 1000, 10)
        .expect("update_paper_full_text should succeed");

    // Note: We can't directly read plain_text back with current API
    // This test verifies the function doesn't error
}

// ============================================================================
// Test: Paper with metadata
// ============================================================================

#[test]
fn test_paper_metadata_preserved() {
    let (db, _temp_dir) = create_test_db();
    let paper = create_test_paper(Some("2301.95001"), "Metadata Test");
    let paper_id = paper.id.clone();

    db.insert_paper(&paper)
        .expect("insert_paper should succeed");

    let retrieved = db.get_paper(&paper_id).expect("get_paper should succeed");
    assert_eq!(retrieved.metadata.cited_by, 10);
    assert_eq!(retrieved.metadata.references, 5);
    assert_eq!(retrieved.metadata.doi, Some("10.1234/test".to_string()));
    assert_eq!(
        retrieved.metadata.pdf_url,
        Some("https://example.com/paper.pdf".to_string())
    );
}

// ============================================================================
// Test: Search papers (LIKE-based, non-FTS)
// ============================================================================

#[test]
fn test_search_papers_basic() {
    let (db, _temp_dir) = create_test_db();
    let paper = create_test_paper(Some("2301.96001"), "Attention Is All You Need");
    db.insert_paper(&paper)
        .expect("insert_paper should succeed");

    let results = db
        .search_papers("Attention", 10)
        .expect("search_papers should work");
    assert_eq!(results.len(), 1, "Should find 1 paper with 'Attention'");
}

#[test]
fn test_search_papers_no_results() {
    let (db, _temp_dir) = create_test_db();
    let paper = create_test_paper(Some("2301.96002"), "Specific Title");
    db.insert_paper(&paper)
        .expect("insert_paper should succeed");

    let results = db
        .search_papers("nonexistent term", 10)
        .expect("search_papers should work");
    assert!(results.is_empty(), "Should return empty for no matches");
}

// ============================================================================
// Test: Embeddings
// ============================================================================

#[test]
fn test_set_and_get_embedding() {
    let (db, _temp_dir) = create_test_db();
    let paper = create_test_paper(Some("2301.97001"), "Embedding Test");
    let paper_id = paper.id.clone();

    db.insert_paper(&paper)
        .expect("insert_paper should succeed");

    let embedding: Vec<f32> = vec![0.1, 0.2, 0.3, 0.4];
    db.set_paper_embedding(&paper_id, &embedding)
        .expect("set_paper_embedding should succeed");

    let retrieved = db
        .get_embedding(&paper_id)
        .expect("get_embedding should work");
    assert!(retrieved.is_some(), "Embedding should be found");
    let emb = retrieved.unwrap();
    assert_eq!(emb.len(), 4, "Embedding should have 4 dimensions");
    assert!((emb[0] - 0.1).abs() < 1e-6, "First dimension should be 0.1");
}

#[test]
fn test_list_papers_with_embeddings() {
    let (db, _temp_dir) = create_test_db();

    let paper1 = create_test_paper(Some("2301.98001"), "With Embedding");
    let paper2 = create_test_paper(Some("2301.98002"), "Without Embedding");
    let paper1_id = paper1.id.clone();

    db.insert_paper(&paper1)
        .expect("insert_paper should succeed");
    db.insert_paper(&paper2)
        .expect("insert_paper should succeed");

    let embedding: Vec<f32> = vec![0.1, 0.2, 0.3];
    db.set_paper_embedding(&paper1_id, &embedding)
        .expect("set_paper_embedding should succeed");

    let ids = db
        .list_papers_with_embeddings()
        .expect("list_papers_with_embeddings should work");
    assert_eq!(ids.len(), 1, "Should have 1 paper with embedding");
    assert_eq!(ids[0], paper1_id);
}

// ============================================================================
// Test: Rate Limiter (via Database access patterns)
// ============================================================================

#[test]
fn test_rate_limiter_basic() {
    let rl = rairos_core::RateLimiter::new();
    let handle = rl.get_or_create("test-endpoint");
    assert!(handle.can(), "Should be able to make request");
}

#[test]
fn test_rate_limiter_reset() {
    let rl = rairos_core::RateLimiter::new();
    let handle = rl.get_or_create("reset-test");
    handle.reset();
    assert!(handle.can(), "After reset should be able to make request");
}

// ============================================================================
// Tests: paper_exists
// ============================================================================

#[test]
fn test_paper_exists_finds_existing() {
    let (db, _temp_dir) = create_test_db();
    let paper = create_test_paper(Some("2301.00001"), "Test Paper");
    db.insert_paper(&paper).unwrap();
    assert!(db.paper_exists(&paper.id));
}

#[test]
fn test_paper_exists_not_found() {
    let (db, _temp_dir) = create_test_db();
    assert!(!db.paper_exists("nonexistent"));
}

// ============================================================================
// Tests: get_paper_plain_text
// ============================================================================

#[test]
fn test_get_paper_plain_text_returns_none_when_missing() {
    let (db, _temp_dir) = create_test_db();
    let paper = create_test_paper(Some("2301.00001"), "Test Paper");
    db.insert_paper(&paper).unwrap();
    let text = db.get_paper_plain_text(&paper.id).unwrap();
    assert!(text.is_none());
}

#[test]
fn test_get_paper_plain_text_returns_text() {
    let (db, _temp_dir) = create_test_db();
    let paper = create_test_paper(Some("2301.00001"), "Test Paper");
    db.insert_paper(&paper).unwrap();
    // Set plain_text via public API
    db.update_paper_full_text(&paper.id, "plain text content", 0, 0, 0, 0).unwrap();
    let text = db.get_paper_plain_text(&paper.id).unwrap();
    assert_eq!(text, Some("plain text content".to_string()));
}

// ============================================================================
// Tests: merge_papers
// ============================================================================

#[test]
fn test_merge_papers_basic() {
    let (db, _temp_dir) = create_test_db();

    let primary = create_test_paper(Some("2301.00001"), "Primary Paper");
    db.insert_paper(&primary).unwrap();

    let mut duplicate = create_test_paper(Some("2301.00002"), "Duplicate Paper");
    duplicate.id = "dup-001".to_string();
    db.insert_paper(&duplicate).unwrap();

    let result = db.merge_papers(&primary.id, &["dup-001"]).unwrap();
    assert!(result, "merge should return true");

    // Duplicate should be deleted
    assert!(!db.paper_exists("dup-001"));
}

#[test]
fn test_merge_papers_primary_not_found() {
    let (db, _temp_dir) = create_test_db();
    let result = db.merge_papers("nonexistent", &["dup-001"]).unwrap();
    assert!(!result, "merge should return false when primary not found");
}

#[test]
fn test_merge_papers_copies_empty_fields() {
    let (db, _temp_dir) = create_test_db();

    // Primary has empty abstract_text
    let mut primary = create_test_paper(Some("2301.00001"), "Primary Paper");
    primary.abstract_text = String::new();
    db.insert_paper(&primary).unwrap();

    let mut duplicate = create_test_paper(Some("2301.00002"), "Duplicate Paper");
    duplicate.id = "dup-002".to_string();
    duplicate.abstract_text = "Detailed abstract from duplicate".to_string();
    db.insert_paper(&duplicate).unwrap();

    db.merge_papers(&primary.id, &["dup-002"]).unwrap();

    // Verify primary got the abstract from duplicate
    let merged = db.get_paper(&primary.id).unwrap();
    assert_eq!(merged.abstract_text, "Detailed abstract from duplicate");
}

#[test]
fn test_merge_papers_does_not_overwrite_filled_fields() {
    let (db, _temp_dir) = create_test_db();

    // Primary has a DOI — should NOT be overwritten
    let primary = create_test_paper(Some("2301.00001"), "Primary Paper");
    db.insert_paper(&primary).unwrap();

    let mut duplicate = create_test_paper(Some("2301.00002"), "Duplicate Paper");
    duplicate.id = "dup-003".to_string();
    // Remove duplicate's DOI to ensure it doesn't overwrite
    duplicate.metadata.doi = None;
    db.insert_paper(&duplicate).unwrap();

    db.merge_papers(&primary.id, &["dup-003"]).unwrap();

    let merged = db.get_paper(&primary.id).unwrap();
    assert_eq!(merged.metadata.doi, Some("10.1234/test".to_string()));
}

#[test]
fn test_merge_papers_merges_multiple_duplicates() {
    let (db, _temp_dir) = create_test_db();

    let primary = create_test_paper(Some("2301.00001"), "Primary Paper");
    db.insert_paper(&primary).unwrap();

    let mut dup1 = create_test_paper(Some("2301.00002"), "Dup 1");
    dup1.id = "dup-a".to_string();
    db.insert_paper(&dup1).unwrap();

    let mut dup2 = create_test_paper(Some("2301.00003"), "Dup 2");
    dup2.id = "dup-b".to_string();
    db.insert_paper(&dup2).unwrap();

    let result = db.merge_papers(&primary.id, &["dup-a", "dup-b"]).unwrap();
    assert!(result);
    assert!(!db.paper_exists("dup-a"));
    assert!(!db.paper_exists("dup-b"));
    assert!(db.paper_exists(&primary.id));
}

#[test]
fn test_merge_papers_does_not_merge_into_self() {
    let (db, _temp_dir) = create_test_db();
    let paper = create_test_paper(Some("2301.00001"), "Self Merge Test");
    db.insert_paper(&paper).unwrap();
    let result = db.merge_papers(&paper.id, &[&paper.id]).unwrap();
    assert!(!result, "merge should return false when merging paper into itself");
    assert!(db.paper_exists(&paper.id));
}

// ============================================================================
// Tests: log_dedup + get_dedup_log
// ============================================================================

#[test]
fn test_log_dedup_and_retrieve() {
    let (db, _temp_dir) = create_test_db();
    db.log_dedup("target-001", "dup-001", "semantic-auto").unwrap();

    let log = db.get_dedup_log(10).unwrap();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].target_id, "target-001");
    assert_eq!(log[0].duplicate_id, "dup-001");
    assert_eq!(log[0].keep_policy, "semantic-auto");
}

#[test]
fn test_get_dedup_log_empty() {
    let (db, _temp_dir) = create_test_db();
    let log = db.get_dedup_log(10).unwrap();
    assert!(log.is_empty());
}

#[test]
fn test_log_dedup_multiple_entries() {
    let (db, _temp_dir) = create_test_db();
    db.log_dedup("t1", "d1", "older").unwrap();
    db.log_dedup("t2", "d2", "newer").unwrap();
    db.log_dedup("t3", "d3", "parsed").unwrap();

    let log = db.get_dedup_log(10).unwrap();
    assert_eq!(log.len(), 3);
    // Verify all targets are present (ordering non-deterministic due to SQLite datetime resolution)
    let targets: Vec<&str> = log.iter().map(|e| e.target_id.as_str()).collect();
    assert!(targets.contains(&"t1"));
    assert!(targets.contains(&"t2"));
    assert!(targets.contains(&"t3"));
}

#[test]
fn test_log_dedup_respects_limit() {
    let (db, _temp_dir) = create_test_db();
    for i in 0..10 {
        db.log_dedup(&format!("t{}", i), &format!("d{}", i), "test").unwrap();
    }
    let log = db.get_dedup_log(5).unwrap();
    assert_eq!(log.len(), 5);
}
