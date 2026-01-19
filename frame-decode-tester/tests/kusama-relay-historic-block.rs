// Copyright (C) 2022-2025 Parity Technologies (UK) Ltd. (admin@parity.io)
// This file is a part of the frame-decode crate.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//         http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
#![cfg(feature = "kusama-relay")]

mod common;

use common::{
    KUSAMA_RELAY_RPC_URLS, TestTier, blocks_for_spec_windows, connections_for_blocks,
    debug_enabled, expand_markers, extra_block_samples_per_window,
};
use frame_decode_tester::{ChainTypes, ExtrinsicTestResult, TestBlocks};
use std::time::Instant;

fn label_for_block(block_number: u64, markers: &[u64]) -> String {
    for (idx, &marker) in markers.iter().enumerate() {
        if block_number >= marker && block_number < marker + 3 {
            let offset = block_number - marker;
            return format!("marker[{idx}]={marker}+{offset}");
        }
    }
    for (idx, window) in markers.windows(2).enumerate() {
        let start = window[0];
        let end = window[1];
        if block_number >= start && block_number < end {
            return format!("window[{idx}] {start}..{end}");
        }
    }
    "other".to_string()
}

fn failure_summary(tester: &TestBlocks) -> String {
    let mut out = String::new();
    for block in tester.results() {
        for (idx, ext) in block.extrinsics.iter().enumerate() {
            if let ExtrinsicTestResult::Failure { error, .. } = ext {
                out.push_str(&format!(
                    "Block {}, extrinsic {}: {}\n",
                    block.block_number, idx, error
                ));
            }
        }
    }
    out
}

#[tokio::test]
async fn test_kusama_relay_historic_blocks() {
    let tier = TestTier::from_env();
    let connections = connections_for_blocks(tier);
    let extra_samples = extra_block_samples_per_window(tier);

    // Kusama Relay Chain - blocks where spec version changes (pre-V14 only)
    // Pre-V14 range: Block 1 to Block 9,625,128 (spec 1020 to 9100)
    // V14 starts: Block 9,625,129 (spec 9111)
    let markers = [
        26668, 38244, 54248, 59658, 67650, 82191, 83237, 101503, 203466, 295787, 461692, 504329,
        569326, 587686, 653183, 693487, 901442, 1375086, 1445458, 1472960, 1475648, 1491596,
        1574408, 2064961, 2201991, 2671528, 2704202, 2728002, 2832534, 2962294, 3240000, 3274408,
        3323565, 3534175, 3860281, 4143129, 4401242, 4841367, 5961600, 6137912, 6561855, 7100891,
        7468792, 7668600, 7812476, 8010981, 8073833, 8555825, 8945245, 9611377,
    ];
    let mut blocks = expand_markers(&markers, 3);
    blocks.extend(blocks_for_spec_windows(&markers, extra_samples));
    blocks.sort_unstable();
    blocks.dedup();
    let expected_blocks = blocks.len();

    let started = Instant::now();
    let tester = TestBlocks::builder()
        .add_urls(KUSAMA_RELAY_RPC_URLS.iter().copied())
        .chain_types(ChainTypes::Kusama)
        .test_blocks(blocks.iter().copied())
        .connections(connections)
        .run()
        .await
        .expect("Failed to run test");
    let elapsed = started.elapsed().as_secs_f64().max(0.000_001);

    eprintln!(
        "METRIC decode_blocks chain=kusama_relay tier={tier:?} connections={connections} urls={} expected_blocks={expected_blocks} tested_blocks={} extrinsics={} failures={} secs={:.3} blocks_per_s={:.3} extrinsics_per_s={:.3}",
        KUSAMA_RELAY_RPC_URLS.len(),
        tester.block_count(),
        tester.extrinsic_count(),
        tester.failure_count(),
        elapsed,
        tester.block_count() as f64 / elapsed,
        tester.extrinsic_count() as f64 / elapsed,
    );

    if debug_enabled() {
        eprintln!(
            "[debug] tier={tier:?} connections={connections} urls={} markers={} extra_samples_per_window={extra_samples} blocks={expected_blocks} blocks_tested={} extrinsics_tested={} failures={}",
            KUSAMA_RELAY_RPC_URLS.len(),
            markers.len(),
            tester.block_count(),
            tester.extrinsic_count(),
            tester.failure_count(),
        );
        for block in tester.results().iter() {
            eprintln!(
                "[debug] block={} kind={} hash={:?} spec_version={} extrinsics={}",
                block.block_number,
                label_for_block(block.block_number, &markers),
                block.block_hash,
                block.spec_version,
                block.extrinsics.len()
            );
        }
    }

    assert_eq!(
        tester.block_count(),
        expected_blocks,
        "Not all blocks produced results"
    );

    assert!(
        tester.all_success(),
        "Failed to decode {} extrinsics out of {}\n{}",
        tester.failure_count(),
        tester.extrinsic_count(),
        failure_summary(&tester)
    );
}
