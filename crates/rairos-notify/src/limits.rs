//! Platform-specific API limits for Discord and Feishu webhooks.

pub mod discord {
    pub const TITLE_MAX_LEN: usize = 256;
    pub const MESSAGE_MAX_LEN: usize = 2048;
    pub const PAPER_NAME_MAX_LEN: usize = 32;
}

pub mod feishu {
    pub const TITLE_MAX_LEN: usize = 100;
    pub const MESSAGE_MAX_LEN: usize = 2000;
}
