//! Rairos Notes — C/M/P Note System
//!
//! C-Note: Concept note (core definitions, background, technical nature)
//! M-Note: Comparison note (comparing 3 P-Notes on the same topic)
//! P-Note: Paper note (individual paper annotations with tags)

pub mod cnote;
pub mod frontmatter;
pub mod mnote;
pub mod pnote;
pub mod render;

pub use cnote::{ensure_cnote, update_cnote_links, upsert_link_under_heading};
pub use frontmatter::{parse_date_from_frontmatter, parse_tags_from_frontmatter, Frontmatter};
pub use mnote::{ensure_or_update_mnote, mnote_filename};
pub use pnote::{collect_pnotes, pnotes_by_tag, read_pnote_metadata, wikilink_for_pnote};
pub use render::{render_cnote, render_mnote, render_pnote, PnoteMetadata};
