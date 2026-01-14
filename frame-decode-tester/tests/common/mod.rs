//! Shared helpers for integration tests in `frame-decode-tester`.
// Integration tests are compiled as separate crates, so some helpers will be unused per-test.
#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestTier {
    Pr,
    Deep,
}

impl TestTier {
    pub fn from_env() -> Self {
        match std::env::var("FRAME_DECODE_TIER")
            .ok()
            .as_deref()
        {
            Some("deep") | Some("DEEP") => TestTier::Deep,
            _ => TestTier::Pr,
        }
    }
}


pub const KUSAMA_ASSETHUB_RPC_URLS: &[&str] = &[
    // official one first.
    // https://docs.polkadot.com/smart-contracts/connect/#__tabbed_1_2
    "wss://kusama-asset-hub-rpc.polkadot.io",
    // https://www.dwellir.com/public-rpc-endpoints
    // Backup public RPCs:
    "wss://asset-hub-kusama-rpc.n.dwellir.com",
];

pub fn debug_enabled() -> bool {
    std::env::var("FRAME_DECODE_TEST_DEBUG")
        .ok()
        .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
}

pub fn connections_for_blocks(tier: TestTier) -> usize {
    match tier {
        TestTier::Pr => 2,
        TestTier::Deep => 4,
    }
}

pub fn connections_for_storage(tier: TestTier) -> usize {
    match tier {
        TestTier::Pr => 1,
        TestTier::Deep => 2,
    }
}

