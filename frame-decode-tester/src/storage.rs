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
use crate::rpc_state::RpcTestState;
use crate::types::ChainTypes;
use frame_decode::storage::StorageEntryInfo;
use frame_metadata::RuntimeMetadata;
use scale_info_legacy::{ChainTypeRegistry, TypeRegistrySet};
use scale_type_resolver::TypeResolver;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use subxt::utils::H256;
use tokio::sync::mpsc;
use tokio::time::{Duration, sleep};

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

/// A rule to ignore specific decode issues for a storage item within a block range.
#[derive(Debug, Clone)]
pub struct IgnoreRule {
    /// Pallet name to match.
    pub pallet_name: String,
    /// Storage entry name to match.
    pub storage_entry: String,
    /// Block range (inclusive start, exclusive end). None means all blocks.
    pub block_range: Option<std::ops::Range<u64>>,
    /// Reason for ignoring (for documentation).
    pub reason: String,
}

impl IgnoreRule {
    /// Check if this rule matches the given pallet, storage entry, and block number.
    pub fn matches(&self, pallet: &str, entry: &str, block: u64) -> bool {
        if self.pallet_name != pallet || self.storage_entry != entry {
            return false;
        }
        match &self.block_range {
            Some(range) => range.contains(&block),
            None => true,
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
    discover_entries: bool,
    discover_max_items_per_block: usize,
    keys_page_size: u32,
    max_keys_per_item: usize,
    max_values_per_block: usize,
    /// Rules for ignoring leftover bytes after decoding specific storage items.
    ignore_leftover_bytes_rules: Vec<IgnoreRule>,
}

impl Default for TestStorageBuilder {
    fn default() -> Self {
        TestStorageBuilder {
            urls: Vec::new(),
            chain_types: ChainTypes::default(),
            blocks: Vec::new(),
            connections: 10,
            items: Vec::new(),
            discover_entries: false,
            discover_max_items_per_block: 50,
            keys_page_size: 256,
            max_keys_per_item: 256,
            max_values_per_block: usize::MAX,
            ignore_leftover_bytes_rules: Vec::new(),
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

    /// Auto-discover storage entries from metadata at each tested block.
    ///
    /// This is for broad, sampled storage coverage. Discovered entries are capped to
    /// `max_items_per_block`.
    pub fn discover_storage_entries(mut self, max_items_per_block: usize) -> Self {
        self.discover_entries = true;
        self.discover_max_items_per_block = max_items_per_block.max(1);
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

    /// Cap the total number of storage values tested per block (across all storage items).
    pub fn max_values_per_block(mut self, count: usize) -> Self {
        self.max_values_per_block = count.max(1);
        self
    }

    /// Ignore leftover bytes for a specific storage item within a block range.
    ///
    /// This is useful when runtime upgrades add new fields to storage structs but
    /// existing entries aren't migrated immediately (lazy migration). Both old and
    /// new formats coexist at the same block.
    ///
    /// # Arguments
    /// * `pallet` - Pallet name (e.g., "Staking")
    /// * `entry` - Storage entry name (e.g., "Ledger")
    /// * `blocks` - Block range to apply this rule (None = all blocks)
    /// * `reason` - Documentation of why this is ignored
    ///
    /// # Example
    /// ```ignore
    /// .ignore_leftover_bytes("Staking", "Ledger", Some(1000..2000),
    ///     "lazy migration of claimed_rewards field")
    /// ```
    pub fn ignore_leftover_bytes(
        mut self,
        pallet: impl Into<String>,
        entry: impl Into<String>,
        blocks: Option<std::ops::Range<u64>>,
        reason: impl Into<String>,
    ) -> Self {
        self.ignore_leftover_bytes_rules.push(IgnoreRule {
            pallet_name: pallet.into(),
            storage_entry: entry.into(),
            block_range: blocks,
            reason: reason.into(),
        });
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
        if !self.discover_entries && self.items.is_empty() {
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
            discover_entries: self.discover_entries,
            discover_max_items_per_block: self.discover_max_items_per_block,
            keys_page_size: self.keys_page_size,
            max_keys_per_item: self.max_keys_per_item,
            max_values_per_block: self.max_values_per_block,
            ignore_leftover_bytes_rules: self.ignore_leftover_bytes_rules,
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
    discover_entries: bool,
    discover_max_items_per_block: usize,
    keys_page_size: u32,
    max_keys_per_item: usize,
    max_values_per_block: usize,
    /// Rules for ignoring leftover bytes after decoding specific storage items.
    ignore_leftover_bytes_rules: Vec<IgnoreRule>,
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
        let total_blocks = self.blocks.len();

        let next_block_idx = Arc::new(AtomicU64::new(0));
        let blocks = Arc::new(self.blocks.clone());
        let items = Arc::new(self.items.clone());
        let discover_entries = self.discover_entries;
        let discover_max_items_per_block = self.discover_max_items_per_block;
        let keys_page_size = self.keys_page_size;
        let max_keys_per_item = self.max_keys_per_item;
        let max_values_per_block = self.max_values_per_block;
        let ignore_leftover_rules = Arc::new(self.ignore_leftover_bytes_rules.clone());

        let (tx, mut rx) = mpsc::channel::<(usize, StorageBlockTestResult)>(num_connections * 2);

        for worker_idx in 0..num_connections {
            let urls = urls.clone();
            let blocks = blocks.clone();
            let items = items.clone();
            let next_block_idx = next_block_idx.clone();
            let historic_types = historic_types.clone();
            let tx = tx.clone();
            let ignore_leftover_rules = ignore_leftover_rules.clone();

            tokio::spawn(async move {
                let mut state = match RpcTestState::new(urls.clone(), worker_idx).await {
                    Ok(s) => s,
                    Err(_) => return,
                };

                loop {
                    let idx = next_block_idx.fetch_add(1, Ordering::Relaxed) as usize;
                    if idx >= blocks.len() {
                        break;
                    }
                    let block_number = blocks[idx];
                    let block_result = test_single_storage_block_with_retry(
                        block_number,
                        &mut state,
                        &historic_types,
                        &items,
                        keys_page_size,
                        max_keys_per_item,
                        discover_entries,
                        discover_max_items_per_block,
                        max_values_per_block,
                        &ignore_leftover_rules,
                    )
                    .await;

                    if tx.send((idx, block_result)).await.is_err() {
                        break;
                    }
                }
            });
        }

        drop(tx);

        let mut results_map: HashMap<usize, StorageBlockTestResult> = HashMap::new();
        let mut debug_seen_specs = HashSet::new();
        let debug_enabled = std::env::var("FRAME_DECODE_TEST_DEBUG")
            .ok()
            .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));
        let mut tested_blocks = 0usize;
        let mut values_tested = 0usize;
        let mut failures_total = 0usize;
        let mut last_log = Instant::now();
        let log_every = Duration::from_secs(30);
        while let Some((idx, result)) = rx.recv().await {
            tested_blocks += 1;
            values_tested += result.value_count();
            failures_total += result.failure_count();
            results_map.insert(idx, result);

            let block = &results_map[&idx];
            let failure_count = block.failure_count();

            if failure_count > 0 {
                for item in &block.items {
                    for value in &item.values {
                        if let StorageValueTestResult::Failure { key, error, .. } = value {
                            eprintln!(
                                "[FAILURE] block={} spec={} {}.{} key={} error={}",
                                block.block_number,
                                block.spec_version,
                                item.pallet_name,
                                item.storage_entry,
                                key,
                                error
                            );
                        }
                    }
                }
            }

            if debug_enabled {
                let is_new_spec = debug_seen_specs.insert(block.spec_version);
                if is_new_spec || failure_count > 0 {
                    eprintln!(
                        "[debug-progress] block={} hash={:?} spec_version={} items={} values={} failures={}",
                        block.block_number,
                        block.block_hash,
                        block.spec_version,
                        block.items.len(),
                        block.value_count(),
                        failure_count,
                    );
                }
            }

            if last_log.elapsed() >= log_every {
                let block = &results_map[&idx];
                eprintln!(
                    "[progress] blocks={tested_blocks}/{total_blocks} values={values_tested} failures={failures_total} spec={} block={}",
                    block.spec_version, block.block_number
                );
                last_log = Instant::now();
            }
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

fn storage_block_level_failure(
    block_number: u64,
    spec_version_hint: u32,
    error: Error,
) -> StorageBlockTestResult {
    StorageBlockTestResult {
        block_number,
        block_hash: H256::from([0u8; 32]),
        spec_version: if spec_version_hint == u32::MAX {
            0
        } else {
            spec_version_hint
        },
        items: vec![StorageItemTestResult {
            pallet_name: "__rpc".to_string(),
            storage_entry: "__rpc".to_string(),
            values: vec![StorageValueTestResult::Failure {
                key: "(n/a)".to_string(),
                error: format!("Failed to test storage at block {block_number}: {error}"),
                raw_bytes: None,
            }],
        }],
    }
}

async fn test_single_storage_block_with_retry(
    block_number: u64,
    state: &mut RpcTestState,
    historic_types: &Arc<ChainTypeRegistry>,
    items: &Arc<Vec<StorageItem>>,
    keys_page_size: u32,
    max_keys_per_item: usize,
    discover_entries: bool,
    discover_max_items_per_block: usize,
    max_values_per_block: usize,
    ignore_leftover_rules: &[IgnoreRule],
) -> StorageBlockTestResult {
    const MAX_ATTEMPTS: usize = 5;
    let mut last_err: Option<Error> = None;

    for attempt in 0..MAX_ATTEMPTS {
        match test_single_storage_block(
            block_number,
            state,
            historic_types,
            items,
            keys_page_size,
            max_keys_per_item,
            discover_entries,
            discover_max_items_per_block,
            max_values_per_block,
            ignore_leftover_rules,
        )
        .await
        {
            Ok(ok) => return ok,
            Err(e) => {
                last_err = Some(e);

                if RpcTestState::is_transient(last_err.as_ref().unwrap()) {
                    state.recover_from_transient().await;
                }

                let backoff_ms = (200u64 << attempt).min(2000);
                sleep(Duration::from_millis(backoff_ms)).await;
            }
        }
    }

    storage_block_level_failure(
        block_number,
        state.current_spec_version,
        last_err.unwrap_or_else(|| Error::RpcError("unknown error".into())),
    )
}

async fn test_single_storage_block(
    block_number: u64,
    state: &mut RpcTestState,
    historic_types: &Arc<ChainTypeRegistry>,
    items: &Arc<Vec<StorageItem>>,
    keys_page_size: u32,
    max_keys_per_item: usize,
    discover_entries: bool,
    discover_max_items_per_block: usize,
    max_values_per_block: usize,
    ignore_leftover_rules: &[IgnoreRule],
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

    let metadata_ref = state.current_metadata.as_ref().unwrap();

    let mut selected_items: Vec<StorageItem> = if discover_entries {
        storage_items_from_metadata(metadata_ref)?
    } else {
        items.as_ref().clone()
    };

    // Sort/dedup to keep deterministic ordering.
    selected_items.sort_by(|a, b| {
        (a.pallet_name.as_str(), a.storage_entry.as_str())
            .cmp(&(b.pallet_name.as_str(), b.storage_entry.as_str()))
    });
    selected_items
        .dedup_by(|a, b| a.pallet_name == b.pallet_name && a.storage_entry == b.storage_entry);

    if discover_entries {
        // Cap the number of discovered items per block.
        let cap = discover_max_items_per_block;
        if selected_items.len() > cap {
            selected_items.truncate(cap);
        }
    }

    let mut item_results = Vec::with_capacity(selected_items.len());
    let mut remaining_values_budget = max_values_per_block;

    for item in selected_items.iter() {
        if remaining_values_budget == 0 {
            break;
        }
        let prefix = frame_decode::storage::encode_storage_key_prefix(
            &item.pallet_name,
            &item.storage_entry,
        );

        let per_item_cap = remaining_values_budget.min(max_keys_per_item);
        let keys = fetch_keys_for_prefix(
            state,
            &prefix,
            Some(block_hash),
            keys_page_size,
            per_item_cap,
        )
        .await?;

        let mut values = Vec::with_capacity(keys.len());
        for key in keys {
            if remaining_values_budget == 0 {
                break;
            }
            let key_hex = format!("0x{}", hex::encode(&key));
            let raw = state.rpc.get_storage(&key, Some(block_hash)).await;
            match raw {
                Ok(Some(bytes)) => {
                    let metadata = state.current_metadata.as_ref().unwrap();
                    let types_for_spec = state.current_types_for_spec.as_ref().unwrap();

                    // Check if any ignore rules match this item/block
                    let leftover_rule = ignore_leftover_rules
                        .iter()
                        .find(|r| r.matches(&item.pallet_name, &item.storage_entry, block_number));

                    let result = decode_storage_value_to_result(
                        &item.pallet_name,
                        &item.storage_entry,
                        block_number,
                        &bytes,
                        metadata,
                        types_for_spec,
                        leftover_rule,
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
            remaining_values_budget = remaining_values_budget.saturating_sub(1);
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

// extracts a list of storage entries (pallet + storage name) from runtime metadata,
// so the storage test can auto‑discover what to query instead of hardcoding a list
fn storage_items_from_metadata(metadata: &RuntimeMetadata) -> Result<Vec<StorageItem>, Error> {
    let tuples: Vec<(String, String)> = with_metadata_uniform!(metadata, |m| {
        m.storage_tuples()
            .map(|(p, s)| (p.into_owned(), s.into_owned()))
            .collect::<Vec<_>>()
    })
    .map_err(|e| Error::RpcError(e.into()))?;

    tuples
        .into_iter()
        .map(|(pallet, entry)| {
            if pallet.is_empty() || entry.is_empty() {
                // If this ever happens, we'd rather fail loudly than silently skip.
                Err(Error::MetadataDecodeError(format!(
                    "Invalid metadata storage tuple: pallet={pallet:?} entry={entry:?}"
                )))
            } else {
                Ok(StorageItem::new(pallet, entry))
            }
        })
        .collect()
}

async fn fetch_keys_for_prefix(
    state: &mut RpcTestState,
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

        let page = state
            .rpc
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
    block_number: u64,
    bytes: &[u8],
    metadata: &RuntimeMetadata,
    legacy_types_for_spec: &TypeRegistrySet,
    leftover_rule: Option<&IgnoreRule>,
) -> Result<scale_value::Value<String>, String> {
    let mut cursor = &*bytes;

    let value = with_metadata_versioned!(metadata, legacy_types_for_spec, |m, resolver| {
        decode_storage_value_inner(&mut cursor, pallet_name, storage_entry, m, resolver)
    })?;

    // Check for leftover bytes
    if !cursor.is_empty() {
        if let Some(rule) = leftover_rule {
            eprintln!(
                "[IgnoreRule] {}.{} at block {}: {} leftover bytes ignored - {}",
                pallet_name,
                storage_entry,
                block_number,
                cursor.len(),
                rule.reason
            );
        } else {
            return Err(format!(
                "{} leftover bytes after decoding storage {}.{} value",
                cursor.len(),
                pallet_name,
                storage_entry
            ));
        }
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
