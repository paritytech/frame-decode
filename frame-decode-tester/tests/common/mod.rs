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

pub fn extra_block_samples_per_window(tier: TestTier) -> usize {
    match tier {
        // Keep PR tier fast.
        TestTier::Pr => 10,
        // Heavier sampling for the scheduled deep run.
        TestTier::Deep => 300,
    }
}

pub fn storage_blocks_per_marker(tier: TestTier) -> usize {
    match tier {
        // Storage is expensive; keep PR minimal.
        TestTier::Pr => 1, // just first item `b`
        // Deep tier: test b,b+1,b+2 around runtime transition.
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

pub fn sample_blocks_in_window(start: u64, end_exclusive: u64, samples: usize) -> Vec<u64> {
    if samples == 0 {
        return Vec::new();
    }
    if end_exclusive <= start + 1 {
        return Vec::new();
    }

    let len = end_exclusive - start;
    // Evenly spaced samples, deterministic.
    let step = (len / samples as u64).max(1);
    let mut out = Vec::with_capacity(samples);
    let mut n = start;
    while n < end_exclusive && out.len() < samples {
        out.push(n);
        n = n.saturating_add(step);
    }
    out
}

pub fn expand_markers(markers: &[u64], blocks_per_marker: usize) -> Vec<u64> {
    let mut out = Vec::with_capacity(markers.len() * blocks_per_marker);
    for &b in markers {
        for i in 0..blocks_per_marker {
            out.push(b + i as u64);
        }
    }
    out.sort_unstable();
    out.dedup();
    out
}

pub fn blocks_for_spec_windows(markers: &[u64], extra_samples_per_window: usize) -> Vec<u64> {
    // Sample only windows we can define (marker[i]..marker[i+1]).
    let mut out = Vec::new();
    for w in markers.windows(2) {
        let start = w[0];
        let end = w[1];
        out.extend(sample_blocks_in_window(
            start,
            end,
            extra_samples_per_window,
        ));
    }
    out.sort_unstable();
    out.dedup();
    out
}
