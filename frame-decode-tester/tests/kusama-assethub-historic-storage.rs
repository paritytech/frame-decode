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

use frame_decode_tester::{ChainTypes, StorageValueTestResult, TestStorage};
use common::{
    connections_for_storage, debug_enabled, expand_markers, max_keys_per_item, storage_blocks_per_marker,
    TestTier, KUSAMA_ASSETHUB_RPC_URLS,
};

fn failure_summary(tester: &TestStorage) -> String {
    let mut out = String::new();
    for block in tester.results() {
        for item in &block.items {
            for value in &item.values {
                if let StorageValueTestResult::Failure { key, error, .. } = value {
                    out.push_str(&format!(
                        "Block {}, {}.{} @ {}: {}\n",
                        block.block_number, item.pallet_name, item.storage_entry, key, error
                    ));
                }
            }
        }
    }
    out
}

#[tokio::test]
async fn test_kusama_asset_hub_historic_storage() {
    let tier = TestTier::from_env();
    let connections = connections_for_storage(tier);
    let blocks_per_marker = storage_blocks_per_marker(tier);
    let max_keys = max_keys_per_item(tier);

    let markers = [
        26668, 38244, 54248, 59658, 67650, 82191, 83237, 101503, 203466, 295787, 461692, 504329,
        569326, 587686, 653183, 693487, 901442,
    ];
    let blocks = expand_markers(&markers, blocks_per_marker);
    let expected_blocks = blocks.len();

    let tester = TestStorage::builder()
        .add_urls(KUSAMA_ASSETHUB_RPC_URLS.iter().copied())
        .chain_types(ChainTypes::KusamaAssetHub)
        .test_blocks(blocks.iter().copied())
        .connections(connections)
        .test_storage_item("System", "Number")
        .max_keys_per_item(max_keys)
        .run()
        .await
        .expect("Failed to run test");

    if debug_enabled() {
        eprintln!(
            "[debug] tier={tier:?} connections={connections} urls={} markers={} blocks_per_marker={blocks_per_marker} max_keys_per_item={max_keys} blocks={expected_blocks} blocks_tested={} values_tested={} failures={}",
            KUSAMA_ASSETHUB_RPC_URLS.len(),
            markers.len(),
            tester.block_count(),
            tester.value_count(),
            tester.failure_count(),
        );
        for block in tester.results().iter().take(5) {
            eprintln!(
                "[debug] sample block={} spec_version={} items={} values={}",
                block.block_number,
                block.spec_version,
                block.items.len(),
                block.value_count()
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
        "Failed to decode {} storage values out of {}\n{}",
        tester.failure_count(),
        tester.value_count(),
        failure_summary(&tester)
    );
}
