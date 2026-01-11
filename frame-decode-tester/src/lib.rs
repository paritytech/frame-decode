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

//! Testing utility for `frame-decode` against live Substrate chains.
//!
//! This crate provides a convenient way to test extrinsic decoding
//! against real blockchain data from live nodes.
//!
//! # Example
//!
//! ```ignore
//! use frame_decode_tester::TestBlocks;
//!
//! TestBlocks::builder()
//!     // Configure URL to connect to:
//!     .add_url("wss://polkadot-public-rpc.blockops.network/ws")
//!     // We can test specific blocks too:
//!     .test_block(123456)
//!     // Start testing the above:
//!     .run()
//!     // Wait for tests to finish
//!     .await?;
//! ```

mod blocks;
mod error;
mod rpc;
mod storage;
mod types;

pub use blocks::{BlockTestResult, ExtrinsicTestResult, TestBlocks};
pub use error::Error;
pub use storage::{
    StorageBlockTestResult, StorageItem, StorageItemTestResult, StorageValueTestResult, TestStorage,
};
pub use types::{ChainTypes, DecodedExtrinsic};
