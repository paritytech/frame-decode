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
        match std::env::var("FRAME_DECODE_TIER").ok().as_deref() {
            Some("deep") | Some("DEEP") => TestTier::Deep,
            _ => TestTier::Pr,
        }
    }
}

pub const KUSAMA_ASSETHUB_RPC_URLS: &[&str] = &[
    // https://docs.polkadot.com/smart-contracts/connect/#__tabbed_1_2
    "wss://kusama-asset-hub-rpc.polkadot.io",
    // https://www.dwellir.com/public-rpc-endpoints
    "wss://asset-hub-kusama-rpc.n.dwellir.com",
];

pub const KUSAMA_RELAY_RPC_URLS: &[&str] = &[
    // https://docs.polkadot.com/getting-started/networks/#kusama-network
    "wss://kusama-rpc.polkadot.io",
    // https://www.dwellir.com/public-rpc-endpoints
    "wss://kusama-rpc.n.dwellir.com",
];

/// Kusama AssetHub spec version change markers (pre-V14 metadata).
/// V14 metadata starts at block 1,057,370 (spec 504).
pub const KUSAMA_ASSETHUB_SPEC_MARKERS: &[u64] = &[
    66686,  // spec 1
    406583, // spec 2
    647941, // spec 3
    955744, // spec 4
    963005, // spec 5 = LAST PRE-V14 SPEC
];

/// Kusama Relay Chain spec version change markers (pre-V14 only).
/// Pre-V14 range: Block 1 to Block 9,625,128 (spec 1020 to 9100).
/// V14 starts: Block 9,625,129 (spec 9111).
pub const KUSAMA_RELAY_SPEC_MARKERS: &[u64] = &[
    0, 26668, 38244, 54248, 59658, 67650, 82191, 83237, 101503, 203466, 295787, 461692, 504329,
    569326, 587686, 653183, 693487, 901442, 1375086, 1445458, 1472960, 1475648, 1491596, 1574408,
    2064961, 2201991, 2671528, 2704202, 2728002, 2832534, 2962294, 3240000, 3274408, 3323565,
    3534175, 3860281, 4143129, 4401242, 4841367, 5961600, 6137912, 6561855, 7100891, 7468792,
    7668600, 7812476, 8010981, 8073833, 8555825, 8945245, 9611377,
];

pub fn debug_enabled() -> bool {
    std::env::var("FRAME_DECODE_TEST_DEBUG")
        .ok()
        .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
}

pub fn connections_for_extrinsics(tier: TestTier) -> usize {
    match tier {
        TestTier::Pr => 4,
        TestTier::Deep => 10,
    }
}

pub fn connections_for_storage(tier: TestTier) -> usize {
    match tier {
        TestTier::Pr => 2,
        TestTier::Deep => 5,
    }
}

/// How many blocks to test around each spec-change marker for extrinsic decoding.
pub fn extrinsic_blocks_per_marker(tier: TestTier) -> usize {
    match tier {
        // Keep PR tier fast.
        TestTier::Pr => 10,
        // Heavier sampling for the scheduled deep run.
        TestTier::Deep => 300,
    }
}

/// How many blocks to test around each spec-change marker for storage decoding.
pub fn storage_blocks_per_marker(tier: TestTier) -> usize {
    match tier {
        TestTier::Pr => 1,
        TestTier::Deep => 5,
    }
}

pub fn max_keys_per_item(tier: TestTier) -> usize {
    match tier {
        TestTier::Pr => 1,
        TestTier::Deep => 5,
    }
}

pub fn discover_max_items_per_block(tier: TestTier) -> usize {
    match tier {
        TestTier::Pr => 20,
        TestTier::Deep => 250,
    }
}

pub fn max_values_per_block(tier: TestTier) -> usize {
    match tier {
        TestTier::Pr => 250,
        TestTier::Deep => 5000,
    }
}

/// Expand markers into blocks: for each marker, emit marker + 0..blocks_per_marker.
pub fn expand_markers(markers: &[u64], blocks_per_marker: usize) -> impl Iterator<Item = u64> + '_ {
    markers
        .iter()
        .flat_map(move |&b| (0..blocks_per_marker).map(move |i| b + i as u64))
}
