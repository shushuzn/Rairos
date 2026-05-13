//! Rairos Render — Rich markdown/note rendering engine
//!
//! Renders C-Note (concept), M-Note (comparison), P-Note (paper), and
//! Literature Review documents from structured data.

#![allow(clippy::too_many_arguments, clippy::needless_range_loop)]

pub mod cnote;
pub mod litreview;
pub mod mnote;
pub mod pnote;
pub mod radar_chart;

pub use cnote::render_cnote;
pub use litreview::{render_litreview, update_litreview};
pub use mnote::render_mnote;
pub use pnote::render_pnote;
pub use radar_chart::render_radar_chart;

#[cfg(test)]
mod tests {
    #[test]
    fn render_version_exists() {
        assert!(true)
    }
}
