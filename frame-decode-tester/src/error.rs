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

//! Error types for frame-decode-tester.

/// Errors that can occur during testing.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// No URL configured for testing.
    #[error("No RPC URL configured")]
    NoUrlsConfigured,
    /// No blocks specified for testing.
    #[error("No blocks specified for testing")]
    NoBlocksSpecified,
    /// No storage items specified for testing.
    #[error("No storage items specified for testing")]
    NoStorageItemsSpecified,
    /// Failed to connect to RPC endpoint.
    #[error("Failed to connect: {0}")]
    ConnectionFailed(String),
    /// RPC request failed.
    #[error("RPC error: {0}")]
    RpcError(String),
    /// Failed to decode metadata.
    #[error("Metadata decode error: {0}")]
    MetadataDecodeError(String),
    /// Block not found.
    #[error("Block {0} not found")]
    BlockNotFound(u64),
}

impl From<subxt::Error> for Error {
    fn from(e: subxt::Error) -> Self {
        Error::RpcError(e.to_string())
    }
}
