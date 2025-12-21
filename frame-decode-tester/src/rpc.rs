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

//! RPC client utilities for connecting to Substrate nodes.

use crate::Error;
use frame_metadata::RuntimeMetadata;
use parity_scale_codec::Decode;
use subxt::backend::legacy::rpc_methods::{Bytes, NumberOrHex};
use subxt::utils::H256;
use subxt_rpcs::client::{RpcClient, RpcParams};
use serde::Deserialize;

/// A thin wrapper around the low-level RPC client for making Substrate RPC calls.
pub struct SubstrateRpc {
    client: RpcClient,
}

impl SubstrateRpc {
    /// Connect to a Substrate node at the given URL.
    pub async fn connect(url: &str) -> Result<Self, Error> {
        let client = RpcClient::from_url(url)
            .await
            .map_err(|e| Error::ConnectionFailed(format!("{url}: {e}")))?;
        Ok(SubstrateRpc { client })
    }

    /// Get the block hash for a given block number.
    pub async fn get_block_hash(&self, block_number: u64) -> Result<Option<H256>, Error> {
        let mut params = RpcParams::new();
        params
            .push(NumberOrHex::Number(block_number))
            .map_err(|e| Error::RpcError(format!("params: {e}")))?;

        let hash = self
            .client
            .request::<Option<H256>>("chain_getBlockHash", params)
            .await
            .map_err(|e| Error::RpcError(format!("chain_getBlockHash: {e}")))?;
        Ok(hash)
    }

    /// Get the block body (extrinsics) for a given block hash.
    pub async fn get_block_body(&self, hash: H256) -> Result<Vec<Bytes>, Error> {
        let mut params = RpcParams::new();
        params
            .push(format!("{hash:?}"))
            .map_err(|e| Error::RpcError(format!("params: {e}")))?;

        let block: Option<SignedBlock<Bytes>> = self
            .client
            .request("chain_getBlock", params)
            .await
            .map_err(|e| Error::RpcError(format!("chain_getBlock: {e}")))?;

        match block {
            Some(b) => Ok(b.block.extrinsics),
            None => Ok(vec![]),
        }
    }

    /// Get the runtime version at a given block hash.
    pub async fn get_runtime_version(&self, hash: Option<H256>) -> Result<u32, Error> {
        let mut params = RpcParams::new();
        if let Some(h) = hash {
            params
                .push(format!("{h:?}"))
                .map_err(|e| Error::RpcError(format!("params: {e}")))?;
        }

        let version = self
            .client
            .request::<RuntimeVersion>("state_getRuntimeVersion", params)
            .await
            .map_err(|e| Error::RpcError(format!("state_getRuntimeVersion: {e}")))?;
        Ok(version.spec_version)
    }

    /// Get the metadata at a given block hash.
    ///
    /// This uses `state_getMetadata` which returns V14 or earlier metadata.
    pub async fn get_metadata(&self, hash: Option<H256>) -> Result<RuntimeMetadata, Error> {
        let metadata_bytes = self.get_metadata_bytes(hash).await?;
        decode_metadata(&metadata_bytes)
    }

    /// Get raw metadata bytes at a given block hash.
    pub async fn get_metadata_bytes(&self, hash: Option<H256>) -> Result<Vec<u8>, Error> {
        let mut params = RpcParams::new();
        if let Some(h) = hash {
            params
                .push(format!("{h:?}"))
                .map_err(|e| Error::RpcError(format!("params: {e}")))?;
        }

        let result: String = self
            .client
            .request("state_getMetadata", params)
            .await
            .map_err(|e| Error::RpcError(format!("state_getMetadata: {e}")))?;

        // Remove 0x prefix and decode hex
        let hex_str = result.strip_prefix("0x").unwrap_or(&result);
        let bytes =
            hex::decode(hex_str).map_err(|e| Error::MetadataDecodeError(format!("hex: {e}")))?;

        Ok(bytes)
    }
}

/// Minimal representation of the runtime version returned by `state_getRuntimeVersion`.
#[derive(Deserialize)]
struct RuntimeVersion {
    spec_version: u32,
}

/// Minimal representation of a signed block returned by `chain_getBlock`.
#[derive(Deserialize)]
struct SignedBlock<T> {
    block: Block<T>,
}

/// Minimal representation of a block containing just the extrinsics field we care about.
#[derive(Deserialize)]
struct Block<T> {
    extrinsics: Vec<T>,
}

/// Decode metadata from raw bytes.
pub fn decode_metadata(bytes: &[u8]) -> Result<RuntimeMetadata, Error> {
    // Skip the magic number prefix (4 bytes) if present
    let bytes = if bytes.len() >= 4 && &bytes[0..4] == b"meta" {
        &bytes[4..]
    } else {
        bytes
    };

    RuntimeMetadata::decode(&mut &*bytes)
        .map_err(|e| Error::MetadataDecodeError(format!("decode: {e}")))
}
