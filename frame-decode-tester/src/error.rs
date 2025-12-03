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

use std::fmt;

/// Errors that can occur during testing.
#[derive(Debug)]
pub enum Error {
    /// No URL configured for testing.
    NoUrlsConfigured,
    /// No blocks specified for testing.
    NoBlocksSpecified,
    /// Failed to connect to RPC endpoint.
    ConnectionFailed(String),
    /// RPC request failed.
    RpcError(String),
    /// Failed to decode metadata.
    MetadataDecodeError(String),
    /// Block not found.
    BlockNotFound(u64),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NoUrlsConfigured => write!(f, "No RPC URL configured"),
            Error::NoBlocksSpecified => write!(f, "No blocks specified for testing"),
            Error::ConnectionFailed(msg) => write!(f, "Failed to connect: {msg}"),
            Error::RpcError(msg) => write!(f, "RPC error: {msg}"),
            Error::MetadataDecodeError(msg) => write!(f, "Metadata decode error: {msg}"),
            Error::BlockNotFound(num) => write!(f, "Block {num} not found"),
        }
    }
}

impl std::error::Error for Error {}

impl From<subxt::Error> for Error {
    fn from(e: subxt::Error) -> Self {
        Error::RpcError(e.to_string())
    }
}
