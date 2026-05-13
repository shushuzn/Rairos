//! Rairos Notes — C/M/P Note System
//!
//! C-Note: Concept note (core definitions, background, technical nature)
//! M-Note: Comparison note (comparing 3 P-Notes on the same topic)
//! P-Note: Paper note (individual paper annotations with tags)

pub mod cnote;
pub mod frontmatter;
pub mod keyword_tags;
pub mod mnote;
pub mod pnote;
pub mod render;

pub use cnote::{ensure_cnote, update_cnote_links, upsert_link_under_heading};
pub use frontmatter::{parse_date_from_frontmatter, parse_tags_from_frontmatter, Frontmatter};
pub use keyword_tags::{get_all_tags, get_keywords_signature, get_tags_count, infer_tags_if_empty};
pub use mnote::{ensure_or_update_mnote, mnote_filename};
pub use pnote::{collect_pnotes, pnotes_by_tag, read_pnote_metadata, wikilink_for_pnote};
pub use render::{render_cnote, render_mnote, render_pnote, PnoteMetadata};

#[cfg(test)]
mod tests {
    use crate::frontmatter::Frontmatter;
    use crate::keyword_tags::get_tags_count;

    #[test]
    fn test_frontmatter_parse_roundtrip() {
        let md = "title: Test\n------------------\n# Content";
        let fm = Frontmatter::parse(md);
        assert_eq!(fm.get_str("title"), Some("Test".to_string()));
    }

    #[test]
    fn test_keyword_tags_nonempty() {
        assert!(get_tags_count() > 30);
    }
}
