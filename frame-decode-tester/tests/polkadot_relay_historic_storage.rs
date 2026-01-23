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
#![cfg(feature = "polkadot-relay")]

mod common;

use common::{
    POLKADOT_RELAY_RPC_URLS, POLKADOT_RELAY_SPEC_MARKERS, TestTier, connections_for_storage,
    debug_enabled, discover_max_items_per_block, expand_markers, max_keys_per_item,
    max_values_per_block, storage_blocks_per_marker,
};
use frame_decode_tester::{ChainTypes, StorageValueTestResult, TestStorage};
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
async fn test_polkadot_relay_historic_storage() {
    let tier = TestTier::from_env();
    let connections = connections_for_storage(tier);
    let bpm = storage_blocks_per_marker(tier);
    let max_keys = max_keys_per_item(tier);
    let discover_max = discover_max_items_per_block(tier);
    let max_values = max_values_per_block(tier);

    let markers = POLKADOT_RELAY_SPEC_MARKERS;
    let blocks: Vec<u64> = expand_markers(markers, bpm).collect();
    let expected_blocks = blocks.len();

    let started = Instant::now();
    let tester = TestStorage::builder()
        .add_urls(POLKADOT_RELAY_RPC_URLS.iter().copied())
        .chain_types(ChainTypes::Polkadot)
        .test_blocks(blocks.iter().copied())
        .connections(connections)
        .discover_storage_entries(discover_max)
        .max_keys_per_item(max_keys)
        .max_values_per_block(max_values)
        .run()
        .await
        .expect("Failed to run test");
    let elapsed = started.elapsed().as_secs_f64().max(0.000_001);

    eprintln!(
        "METRIC decode_storage chain=polkadot_relay tier={tier:?} connections={connections} markers={} blocks_per_marker={bpm} blocks={} values={} failures={} secs={:.3}",
        markers.len(),
        tester.block_count(),
        tester.value_count(),
        tester.failure_count(),
        elapsed,
    );

    if debug_enabled() {
        for block in tester.results().iter() {
            eprintln!(
                "[debug] block={} kind={} hash={:?} spec_version={} items={} values={}",
                block.block_number,
                label_for_block(block.block_number, &markers, bpm),
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
