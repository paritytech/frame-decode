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
#![cfg(feature = "kusama-assethub")]

mod common;

use common::{
    KUSAMA_ASSETHUB_RPC_URLS, KUSAMA_ASSETHUB_SPEC_MARKERS, TestTier, connections_for_extrinsics,
    debug_enabled, expand_markers, extrinsic_blocks_per_marker,
};
use frame_decode_tester::{ChainTypes, ExtrinsicTestResult, TestBlocks};
use std::time::Instant;

fn label_for_block(block_number: u64, markers: &[u64], bpm: usize) -> String {
    for (idx, &marker) in markers.iter().enumerate() {
        let end = marker + bpm as u64;
        if block_number >= marker && block_number < end {
            let offset = block_number - marker;
            return format!("marker[{idx}]={marker}+{offset}");
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
async fn test_kusama_asset_hub_historic_blocks() {
    let tier = TestTier::from_env();
    let connections = connections_for_extrinsics(tier);
    let bpm = extrinsic_blocks_per_marker(tier);

    let markers = KUSAMA_ASSETHUB_SPEC_MARKERS;
    let blocks: Vec<u64> = expand_markers(markers, bpm).collect();
    let expected_blocks = blocks.len();

    let started = Instant::now();
    let tester = TestBlocks::builder()
        .add_urls(KUSAMA_ASSETHUB_RPC_URLS.iter().copied())
        .chain_types(ChainTypes::KusamaAssetHub)
        .test_blocks(blocks.iter().copied())
        .connections(connections)
        .run()
        .await
        .expect("Failed to run test");
    let elapsed = started.elapsed().as_secs_f64().max(0.000_001);

    eprintln!(
        "METRIC decode_extrinsics chain=kusama_assethub tier={tier:?} connections={connections} markers={} blocks_per_marker={bpm} blocks={} extrinsics={} failures={} secs={:.3}",
        markers.len(),
        tester.block_count(),
        tester.extrinsic_count(),
        tester.failure_count(),
        elapsed,
    );

    if debug_enabled() {
        for block in tester.results().iter() {
            eprintln!(
                "[debug] block={} kind={} hash={:?} spec_version={} extrinsics={}",
                block.block_number,
                label_for_block(block.block_number, &markers, bpm),
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
