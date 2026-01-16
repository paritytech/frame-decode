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

use frame_decode_tester::{ChainTypes, ExtrinsicTestResult, TestBlocks};
use common::{
    blocks_for_spec_windows, connections_for_blocks, debug_enabled, expand_markers,
    extra_block_samples_per_window, TestTier, KUSAMA_ASSETHUB_RPC_URLS,
};

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
    let connections = connections_for_blocks(tier);
    let extra_samples = extra_block_samples_per_window(tier);

    let markers = [
        26668, 38244, 54248, 59658, 67650, 82191, 83237, 101503, 203466, 295787, 461692, 504329,
        569326, 587686, 653183, 693487, 901442,
    ];
    let mut blocks = expand_markers(&markers, 3);
    blocks.extend(blocks_for_spec_windows(&markers, extra_samples));
    blocks.sort_unstable();
    blocks.dedup();
    let expected_blocks = blocks.len();

    let tester = TestBlocks::builder()
        .add_urls(KUSAMA_ASSETHUB_RPC_URLS.iter().copied())
        .chain_types(ChainTypes::KusamaAssetHub)
        .test_blocks(blocks.iter().copied())
        .connections(connections)
        .run()
        .await
        .expect("Failed to run test");

    if debug_enabled() {
        eprintln!(
            "[debug] tier={tier:?} connections={connections} urls={} markers={} extra_samples_per_window={extra_samples} blocks={expected_blocks} blocks_tested={} extrinsics_tested={} failures={}",
            KUSAMA_ASSETHUB_RPC_URLS.len(),
            markers.len(),
            tester.block_count(),
            tester.extrinsic_count(),
            tester.failure_count(),
        );
        for block in tester.results().iter().take(5) {
            eprintln!(
                "[debug] sample block={} spec_version={} extrinsics={}",
                block.block_number,
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
