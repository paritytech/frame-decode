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
    KUSAMA_RELAY_RPC_URLS, TestTier, connections_for_storage, debug_enabled,
    discover_max_items_per_block, expand_markers, max_keys_per_item, max_values_per_block,
    storage_blocks_per_marker,
};
use frame_decode_tester::{ChainTypes, StorageValueTestResult, TestStorage};
use std::time::Instant;

fn label_for_block(block_number: u64, markers: &[u64], blocks_per_marker: usize) -> String {
    for (idx, &marker) in markers.iter().enumerate() {
        let end = marker + blocks_per_marker as u64;
        if block_number >= marker && block_number < end {
            let offset = block_number - marker;
            return format!("marker[{idx}]={marker}+{offset}");
        }
    }
    "other".to_string()
}

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
async fn test_kusama_relay_historic_storage() {
    let tier = TestTier::from_env();
    let connections = connections_for_storage(tier);
    let blocks_per_marker = storage_blocks_per_marker(tier);
    let max_keys = max_keys_per_item(tier);
    let discover_max = discover_max_items_per_block(tier);
    let max_values = max_values_per_block(tier);

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
    let blocks = expand_markers(&markers, blocks_per_marker);
    let expected_blocks = blocks.len();

    let started = Instant::now();
    let tester = TestStorage::builder()
        .add_urls(KUSAMA_RELAY_RPC_URLS.iter().copied())
        .chain_types(ChainTypes::Kusama)
        .test_blocks(blocks.iter().copied())
        .connections(connections)
        .discover_storage_entries(discover_max)
        .always_include_storage_item("System", "Number")
        .always_include_storage_item("Timestamp", "Now")
        // Skip known huge or noisy entries.
        .skip_storage_item("System", "Events")
        .max_keys_per_item(max_keys)
        .max_values_per_block(max_values)
        .run()
        .await
        .expect("Failed to run test");
    let elapsed = started.elapsed().as_secs_f64().max(0.000_001);

    eprintln!(
        "METRIC decode_storage chain=kusama_relay tier={tier:?} connections={connections} urls={} expected_blocks={expected_blocks} tested_blocks={} values={} failures={} secs={:.3} blocks_per_s={:.3} values_per_s={:.3}",
        KUSAMA_RELAY_RPC_URLS.len(),
        tester.block_count(),
        tester.value_count(),
        tester.failure_count(),
        elapsed,
        tester.block_count() as f64 / elapsed,
        tester.value_count() as f64 / elapsed,
    );

    if debug_enabled() {
        eprintln!(
            "[debug] tier={tier:?} connections={connections} urls={} markers={} blocks_per_marker={blocks_per_marker} discover_max_items_per_block={discover_max} max_keys_per_item={max_keys} max_values_per_block={max_values} blocks={expected_blocks} blocks_tested={} values_tested={} failures={}",
            KUSAMA_RELAY_RPC_URLS.len(),
            markers.len(),
            tester.block_count(),
            tester.value_count(),
            tester.failure_count(),
        );
        for block in tester.results().iter() {
            eprintln!(
                "[debug] block={} kind={} hash={:?} spec_version={} items={} values={}",
                block.block_number,
                label_for_block(block.block_number, &markers, blocks_per_marker),
                block.block_hash,
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
