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

use frame_decode_tester::{ChainTypes, ExtrinsicTestResult, TestBlocks};

#[tokio::test]
async fn test_kusama_asset_hub_historic_blocks() {
    let tester = TestBlocks::builder()
        .add_url("wss://kusama-asset-hub-rpc.polkadot.io")
        .chain_types(ChainTypes::KusamaAssetHub)
        .test_blocks([
            26668, 38244, 54248, 59658, 67650, 82191, 83237, 101503, 203466, 295787, 461692,
            504329, 569326, 587686, 653183, 693487, 901442,
        ])
        .run()
        .await
        .expect("Failed to run test");

    println!("Blocks tested: {}", tester.block_count());
    println!("Extrinsics tested: {}", tester.extrinsic_count());
    println!("Successful: {}", tester.success_count());
    println!("Failed: {}", tester.failure_count());

    // Print failures for debugging
    for result in tester.results() {
        for (idx, ext) in result.extrinsics.iter().enumerate() {
            if let ExtrinsicTestResult::Failure { error, .. } = ext {
                println!(
                    "Block {}, extrinsic {}: {}",
                    result.block_number, idx, error
                );
            }
        }
    }

    assert!(
        tester.all_success(),
        "Failed to decode {} extrinsics out of {}",
        tester.failure_count(),
        tester.extrinsic_count()
    );
}
