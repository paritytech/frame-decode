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

//! Storage testing functionality.

use crate::Error;
use crate::rpc::SubstrateRpc;
use crate::types::ChainTypes;
use frame_metadata::RuntimeMetadata;
use scale_info_legacy::{ChainTypeRegistry, TypeRegistrySet};
use scale_type_resolver::TypeResolver;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use subxt::utils::H256;
use tokio::sync::mpsc;

/// A storage item to test (pallet + storage entry name).
#[derive(Debug, Clone)]
pub struct StorageItem {
    pub pallet_name: String,
    pub storage_entry: String,
}

impl StorageItem {
    pub fn new(pallet_name: impl Into<String>, storage_entry: impl Into<String>) -> Self {
        StorageItem {
            pallet_name: pallet_name.into(),
            storage_entry: storage_entry.into(),
        }
    }
}

/// Result of testing a single storage value.
#[derive(Debug)]
pub enum StorageValueTestResult {
    /// Successfully decoded the value bytes.
    Success {
        /// Storage key bytes (hex encoded).
        key: String,
        /// Decoded value.
        value: scale_value::Value<String>,
    },
    /// Failed to fetch or decode the value.
    Failure {
        /// Storage key bytes (hex encoded).
        key: String,
        /// Error message.
        error: String,
        /// Raw value bytes (hex encoded), if any were returned.
        raw_bytes: Option<String>,
    },
}

impl StorageValueTestResult {
    pub fn is_success(&self) -> bool {
        matches!(self, StorageValueTestResult::Success { .. })
    }
}

/// Result of testing a single storage item (a pallet.storage entry) at a single block.
#[derive(Debug)]
pub struct StorageItemTestResult {
    pub pallet_name: String,
    pub storage_entry: String,
    pub values: Vec<StorageValueTestResult>,
}

impl StorageItemTestResult {
    pub fn is_success(&self) -> bool {
        self.values.iter().all(|v| v.is_success())
    }
    pub fn success_count(&self) -> usize {
        self.values.iter().filter(|v| v.is_success()).count()
    }
    pub fn failure_count(&self) -> usize {
        self.values.len() - self.success_count()
    }
    pub fn value_count(&self) -> usize {
        self.values.len()
    }
}

/// Result of testing storage for a single block.
#[derive(Debug)]
pub struct StorageBlockTestResult {
    /// The block number.
    pub block_number: u64,
    /// The block hash.
    pub block_hash: H256,
    /// The spec version at this block.
    pub spec_version: u32,
    /// Results for each tested storage entry.
    pub items: Vec<StorageItemTestResult>,
}

impl StorageBlockTestResult {
    pub fn is_success(&self) -> bool {
        self.items.iter().all(|i| i.is_success())
    }
    pub fn success_count(&self) -> usize {
        self.items.iter().map(|i| i.success_count()).sum()
    }
    pub fn failure_count(&self) -> usize {
        self.items.iter().map(|i| i.failure_count()).sum()
    }
    pub fn value_count(&self) -> usize {
        self.items.iter().map(|i| i.value_count()).sum()
    }
}

/// Builder for configuring storage tests.
pub struct TestStorageBuilder {
    urls: Vec<String>,
    chain_types: ChainTypes,
    blocks: Vec<u64>,
    connections: usize,
    items: Vec<StorageItem>,
    keys_page_size: u32,
    max_keys_per_item: usize,
}

impl Default for TestStorageBuilder {
    fn default() -> Self {
        TestStorageBuilder {
            urls: Vec::new(),
            chain_types: ChainTypes::default(),
            blocks: Vec::new(),
            connections: 10,
            items: Vec::new(),
            keys_page_size: 256,
            max_keys_per_item: 256,
        }
    }
}

impl TestStorageBuilder {
    /// Set the RPC URL to connect to.
    pub fn add_url(mut self, url: impl Into<String>) -> Self {
        self.urls.push(url.into());
        self
    }

    /// Add multiple RPC URLs to connect to.
    pub fn add_urls<I, S>(mut self, urls: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.urls.extend(urls.into_iter().map(|url| url.into()));
        self
    }

    /// Set the historic chain types to use for decoding.
    pub fn chain_types(mut self, types: ChainTypes) -> Self {
        self.chain_types = types;
        self
    }

    /// Test a specific block by number.
    pub fn test_block(mut self, block_number: u64) -> Self {
        self.blocks.push(block_number);
        self
    }

    /// Test multiple blocks by number.
    ///
    /// Blocks will be sorted and deduplicated when `.run()` is called.
    pub fn test_blocks<I>(mut self, blocks: I) -> Self
    where
        I: IntoIterator<Item = u64>,
    {
        self.blocks.extend(blocks);
        self
    }

    /// Add a storage item to test.
    ///
    /// This will fetch keys under the pallet+storage prefix and attempt to decode each corresponding value.
    pub fn test_storage_item(
        mut self,
        pallet_name: impl Into<String>,
        storage_entry: impl Into<String>,
    ) -> Self {
        self.items
            .push(StorageItem::new(pallet_name, storage_entry));
        self
    }

    /// Add multiple storage items to test.
    pub fn test_storage_items<I>(mut self, items: I) -> Self
    where
        I: IntoIterator<Item = StorageItem>,
    {
        self.items.extend(items);
        self
    }

    /// Set the number of parallel connections to use (default: 10).
    pub fn connections(mut self, count: usize) -> Self {
        self.connections = count.max(1);
        self
    }

    /// Set the key page size used when querying `state_getKeysPaged` (default: 256).
    pub fn keys_page_size(mut self, count: u32) -> Self {
        self.keys_page_size = count.max(1);
        self
    }

    /// Cap the number of keys tested per storage item per block (default: 256).
    pub fn max_keys_per_item(mut self, count: usize) -> Self {
        self.max_keys_per_item = count.max(1);
        self
    }

    /// Build and run the storage tests.
    pub async fn run(mut self) -> Result<TestStorage, Error> {
        if self.urls.is_empty() {
            return Err(Error::NoUrlsConfigured);
        }
        if self.blocks.is_empty() {
            return Err(Error::NoBlocksSpecified);
        }
        if self.items.is_empty() {
            return Err(Error::NoStorageItemsSpecified);
        }

        self.blocks.sort_unstable();
        self.blocks.dedup();

        let test_storage = TestStorage {
            urls: self.urls,
            chain_types: self.chain_types,
            blocks: self.blocks,
            connections: self.connections,
            items: self.items,
            keys_page_size: self.keys_page_size,
            max_keys_per_item: self.max_keys_per_item,
            results: Vec::new(),
        };

        test_storage.execute().await
    }
}

/// Storage tester that connects to a Substrate node and tests storage value decoding.
pub struct TestStorage {
    urls: Vec<String>,
    chain_types: ChainTypes,
    blocks: Vec<u64>,
    connections: usize,
    items: Vec<StorageItem>,
    keys_page_size: u32,
    max_keys_per_item: usize,
    results: Vec<StorageBlockTestResult>,
}

impl TestStorage {
    /// Create a new builder for configuring storage tests.
    pub fn builder() -> TestStorageBuilder {
        TestStorageBuilder::default()
    }

    /// Get the test results.
    pub fn results(&self) -> &[StorageBlockTestResult] {
        &self.results
    }

    /// Returns true if all tested storage values decoded successfully.
    pub fn all_success(&self) -> bool {
        self.results.iter().all(|r| r.is_success())
    }

    pub fn block_count(&self) -> usize {
        self.results.len()
    }

    pub fn value_count(&self) -> usize {
        self.results.iter().map(|r| r.value_count()).sum()
    }

    pub fn success_count(&self) -> usize {
        self.results.iter().map(|r| r.success_count()).sum()
    }

    pub fn failure_count(&self) -> usize {
        self.results.iter().map(|r| r.failure_count()).sum()
    }

    async fn execute(mut self) -> Result<TestStorage, Error> {
        let historic_types = Arc::new(self.chain_types.load());
        let urls = Arc::new(self.urls.clone());
        let num_connections = self.connections.min(self.blocks.len());

        let next_block_idx = Arc::new(AtomicU64::new(0));
        let blocks = Arc::new(self.blocks.clone());
        let items = Arc::new(self.items.clone());
        let keys_page_size = self.keys_page_size;
        let max_keys_per_item = self.max_keys_per_item;

        let (tx, mut rx) = mpsc::channel::<(usize, StorageBlockTestResult)>(num_connections * 2);

        for worker_idx in 0..num_connections {
            let urls = urls.clone();
            let blocks = blocks.clone();
            let items = items.clone();
            let next_block_idx = next_block_idx.clone();
            let historic_types = historic_types.clone();
            let tx = tx.clone();

            tokio::spawn(async move {
                let url = urls[worker_idx % urls.len()].clone();
                let rpc = match SubstrateRpc::connect(&url).await {
                    Ok(rpc) => rpc,
                    Err(_) => return,
                };

                let mut state = StorageTestState {
                    rpc,
                    current_spec_version: u32::MAX,
                    current_metadata: None,
                    current_types_for_spec: None,
                };

                loop {
                    let idx = next_block_idx.fetch_add(1, Ordering::Relaxed) as usize;
                    if idx >= blocks.len() {
                        break;
                    }
                    let block_number = blocks[idx];
                    let result = test_single_storage_block(
                        block_number,
                        &mut state,
                        &historic_types,
                        &items,
                        keys_page_size,
                        max_keys_per_item,
                    )
                    .await;

                    match result {
                        Ok(block_result) => {
                            if tx.send((idx, block_result)).await.is_err() {
                                break;
                            }
                        }
                        Err(e) => {
                            eprintln!(
                                "Worker {worker_idx}: failed to test storage at block {block_number}: {e}"
                            );
                        }
                    }
                }
            });
        }

        drop(tx);

        let mut results_map: HashMap<usize, StorageBlockTestResult> = HashMap::new();
        while let Some((idx, result)) = rx.recv().await {
            results_map.insert(idx, result);
        }

        let mut sorted_indices: Vec<_> = results_map.keys().copied().collect();
        sorted_indices.sort_unstable();
        for idx in sorted_indices {
            if let Some(result) = results_map.remove(&idx) {
                self.results.push(result);
            }
        }

        Ok(self)
    }
}

struct StorageTestState {
    rpc: SubstrateRpc,
    current_spec_version: u32,
    current_metadata: Option<RuntimeMetadata>,
    current_types_for_spec: Option<TypeRegistrySet<'static>>,
}

async fn test_single_storage_block(
    block_number: u64,
    state: &mut StorageTestState,
    historic_types: &Arc<ChainTypeRegistry>,
    items: &Arc<Vec<StorageItem>>,
    keys_page_size: u32,
    max_keys_per_item: usize,
) -> Result<StorageBlockTestResult, Error> {
    // Same rule as in TestBlocks: runtime updates take effect the block after.
    let runtime_update_block = block_number.saturating_sub(1);
    let runtime_update_hash = state
        .rpc
        .get_block_hash(runtime_update_block)
        .await?
        .ok_or(Error::BlockNotFound(runtime_update_block))?;

    let spec_version = state
        .rpc
        .get_runtime_version(Some(runtime_update_hash))
        .await?;

    if spec_version != state.current_spec_version
        || state.current_metadata.is_none()
        || state.current_types_for_spec.is_none()
    {
        let metadata = state.rpc.get_metadata(Some(runtime_update_hash)).await?;

        let mut types_for_spec = historic_types
            .for_spec_version(spec_version as u64)
            .to_owned();

        if let Ok(metadata_types) =
            frame_decode::helpers::type_registry_from_metadata_any(&metadata)
        {
            types_for_spec.prepend(metadata_types);
        }

        state.current_types_for_spec = Some(types_for_spec);
        state.current_metadata = Some(metadata);
        state.current_spec_version = spec_version;
    }

    let block_hash = state
        .rpc
        .get_block_hash(block_number)
        .await?
        .ok_or(Error::BlockNotFound(block_number))?;

    let metadata = state.current_metadata.as_ref().unwrap();
    let types_for_spec = state.current_types_for_spec.as_ref().unwrap();

    let mut item_results = Vec::with_capacity(items.len());
    for item in items.iter() {
        let prefix = frame_decode::storage::encode_storage_key_prefix(
            &item.pallet_name,
            &item.storage_entry,
        );
        let keys = fetch_keys_for_prefix(
            &state.rpc,
            &prefix,
            Some(block_hash),
            keys_page_size,
            max_keys_per_item,
        )
        .await?;

        let mut values = Vec::with_capacity(keys.len());
        for key in keys {
            let key_hex = format!("0x{}", hex::encode(&key));
            let raw = state.rpc.get_storage(&key, Some(block_hash)).await;
            match raw {
                Ok(Some(bytes)) => {
                    let result = decode_storage_value_to_result(
                        &item.pallet_name,
                        &item.storage_entry,
                        &bytes,
                        metadata,
                        types_for_spec,
                    );
                    values.push(match result {
                        Ok(value) => StorageValueTestResult::Success {
                            key: key_hex,
                            value,
                        },
                        Err(error) => StorageValueTestResult::Failure {
                            key: key_hex,
                            error,
                            raw_bytes: Some(format!("0x{}", hex::encode(&bytes))),
                        },
                    });
                }
                Ok(None) => {
                    values.push(StorageValueTestResult::Failure {
                        key: key_hex,
                        error: "No value returned for storage key".to_string(),
                        raw_bytes: None,
                    });
                }
                Err(e) => {
                    values.push(StorageValueTestResult::Failure {
                        key: key_hex,
                        error: format!("{e}"),
                        raw_bytes: None,
                    });
                }
            }
        }

        item_results.push(StorageItemTestResult {
            pallet_name: item.pallet_name.clone(),
            storage_entry: item.storage_entry.clone(),
            values,
        });
    }

    Ok(StorageBlockTestResult {
        block_number,
        block_hash,
        spec_version,
        items: item_results,
    })
}

async fn fetch_keys_for_prefix(
    rpc: &SubstrateRpc,
    prefix: &[u8],
    at: Option<H256>,
    keys_page_size: u32,
    max_keys: usize,
) -> Result<Vec<Vec<u8>>, Error> {
    let mut all = Vec::new();
    let mut start_key: Option<Vec<u8>> = None;

    while all.len() < max_keys {
        let remaining = max_keys - all.len();
        let count = keys_page_size.min(remaining as u32);

        let page = rpc
            .get_keys_paged(prefix, count, start_key.as_deref(), at)
            .await?;

        if page.is_empty() {
            break;
        }

        start_key = page.last().cloned();
        all.extend(page);
    }

    Ok(all)
}

fn decode_storage_value_to_result(
    pallet_name: &str,
    storage_entry: &str,
    bytes: &[u8],
    metadata: &RuntimeMetadata,
    legacy_types_for_spec: &TypeRegistrySet,
) -> Result<scale_value::Value<String>, String> {
    let mut cursor = &*bytes;

    let value = match metadata {
        RuntimeMetadata::V8(m) => decode_storage_value_inner(
            &mut cursor,
            pallet_name,
            storage_entry,
            m,
            legacy_types_for_spec,
        ),
        RuntimeMetadata::V9(m) => decode_storage_value_inner(
            &mut cursor,
            pallet_name,
            storage_entry,
            m,
            legacy_types_for_spec,
        ),
        RuntimeMetadata::V10(m) => decode_storage_value_inner(
            &mut cursor,
            pallet_name,
            storage_entry,
            m,
            legacy_types_for_spec,
        ),
        RuntimeMetadata::V11(m) => decode_storage_value_inner(
            &mut cursor,
            pallet_name,
            storage_entry,
            m,
            legacy_types_for_spec,
        ),
        RuntimeMetadata::V12(m) => decode_storage_value_inner(
            &mut cursor,
            pallet_name,
            storage_entry,
            m,
            legacy_types_for_spec,
        ),
        RuntimeMetadata::V13(m) => decode_storage_value_inner(
            &mut cursor,
            pallet_name,
            storage_entry,
            m,
            legacy_types_for_spec,
        ),
        RuntimeMetadata::V14(m) => {
            decode_storage_value_inner(&mut cursor, pallet_name, storage_entry, m, &m.types)
        }
        RuntimeMetadata::V15(m) => {
            decode_storage_value_inner(&mut cursor, pallet_name, storage_entry, m, &m.types)
        }
        RuntimeMetadata::V16(m) => {
            decode_storage_value_inner(&mut cursor, pallet_name, storage_entry, m, &m.types)
        }
        _ => Err("Unsupported metadata version".to_string()),
    }?;

    if !cursor.is_empty() {
        return Err(format!(
            "{} leftover bytes after decoding storage {}.{} value",
            cursor.len(),
            pallet_name,
            storage_entry
        ));
    }

    Ok(value)
}

fn decode_storage_value_inner<'scale, Info, Resolver>(
    cursor: &mut &'scale [u8],
    pallet_name: &str,
    storage_entry: &str,
    info: &Info,
    type_resolver: &Resolver,
) -> Result<scale_value::Value<String>, String>
where
    Info: frame_decode::storage::StorageTypeInfo,
    Info::TypeId: Clone + core::fmt::Debug,
    Resolver: TypeResolver<TypeId = Info::TypeId>,
{
    let value = frame_decode::storage::decode_storage_value(
        pallet_name,
        storage_entry,
        cursor,
        info,
        type_resolver,
        scale_value::scale::ValueVisitor::new(),
    )
    .map_err(|e| format!("{e:?}"))?
    .map_context(|ctx| format!("{ctx:?}"));

    Ok(value)
}
