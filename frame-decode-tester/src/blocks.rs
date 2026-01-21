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

//! Block testing functionality.

use crate::Error;
use crate::rpc_state::RpcTestState;
use crate::types::{ChainTypes, DecodedArg, DecodedExtrinsic};
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

/// Result of testing a single block.
#[derive(Debug)]
pub struct BlockTestResult {
    /// The block number.
    pub block_number: u64,
    /// The block hash.
    pub block_hash: H256,
    /// The spec version at this block.
    pub spec_version: u32,
    /// Results for each extrinsic in the block.
    pub extrinsics: Vec<ExtrinsicTestResult>,
}

impl BlockTestResult {
    /// Returns true if all extrinsics decoded successfully.
    pub fn is_success(&self) -> bool {
        self.extrinsics.iter().all(|e| e.is_success())
    }

    /// Returns the number of successful decodes.
    pub fn success_count(&self) -> usize {
        self.extrinsics.iter().filter(|e| e.is_success()).count()
    }

    /// Returns the number of failed decodes.
    pub fn failure_count(&self) -> usize {
        self.extrinsics.iter().filter(|e| !e.is_success()).count()
    }
}

/// Result of testing a single extrinsic.
#[derive(Debug)]
pub enum ExtrinsicTestResult {
    /// Successfully decoded extrinsic.
    Success(DecodedExtrinsic),
    /// Failed to decode extrinsic.
    Failure {
        /// The error message.
        error: String,
        /// The raw extrinsic bytes (hex encoded).
        raw_bytes: String,
    },
}

impl ExtrinsicTestResult {
    /// Returns true if this extrinsic decoded successfully.
    pub fn is_success(&self) -> bool {
        matches!(self, ExtrinsicTestResult::Success(_))
    }
}

/// Builder for configuring block tests.
pub struct TestBlocksBuilder {
    /// One or more RPC URLs to connect to.
    ///
    /// Multiple URLs allow us to spread load across several nodes. Each worker
    /// will pick one of the configured URLs to connect to.
    urls: Vec<String>,
    chain_types: ChainTypes,
    blocks: Vec<u64>,
    connections: usize,
}

impl Default for TestBlocksBuilder {
    fn default() -> Self {
        TestBlocksBuilder {
            urls: Vec::new(),
            chain_types: ChainTypes::default(),
            blocks: Vec::new(),
            connections: 10,
        }
    }
}

impl TestBlocksBuilder {
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

    /// Set the number of parallel connections to use (default: 10).
    pub fn connections(mut self, count: usize) -> Self {
        self.connections = count.max(1);
        self
    }

    /// Build and run the block tests.
    pub async fn run(mut self) -> Result<TestBlocks, Error> {
        if self.urls.is_empty() {
            return Err(Error::NoUrlsConfigured);
        }

        if self.blocks.is_empty() {
            return Err(Error::NoBlocksSpecified);
        }

        // Sort and deduplicate blocks
        self.blocks.sort_unstable();
        self.blocks.dedup();

        let test_blocks = TestBlocks {
            urls: self.urls,
            chain_types: self.chain_types,
            blocks: self.blocks,
            connections: self.connections,
            results: Vec::new(),
        };

        test_blocks.execute().await
    }
}

/// Block tester that connects to a Substrate node and tests extrinsic decoding.
pub struct TestBlocks {
    /// RPC URLs to connect to.
    ///
    /// When multiple URLs are provided, workers will be distributed across
    /// these URLs in a simple round-robin fashion to help parallelise work.
    urls: Vec<String>,
    chain_types: ChainTypes,
    blocks: Vec<u64>,
    connections: usize,
    results: Vec<BlockTestResult>,
}

impl TestBlocks {
    /// Create a new builder for configuring block tests.
    pub fn builder() -> TestBlocksBuilder {
        TestBlocksBuilder::default()
    }

    /// Get the test results.
    pub fn results(&self) -> &[BlockTestResult] {
        &self.results
    }

    /// Returns true if all blocks were tested successfully.
    pub fn all_success(&self) -> bool {
        self.results.iter().all(|r| r.is_success())
    }

    /// Returns the total number of blocks tested.
    pub fn block_count(&self) -> usize {
        self.results.len()
    }

    /// Returns the total number of extrinsics tested.
    pub fn extrinsic_count(&self) -> usize {
        self.results.iter().map(|r| r.extrinsics.len()).sum()
    }

    /// Returns the number of successful extrinsic decodes.
    pub fn success_count(&self) -> usize {
        self.results.iter().map(|r| r.success_count()).sum()
    }

    /// Returns the number of failed extrinsic decodes.
    pub fn failure_count(&self) -> usize {
        self.results.iter().map(|r| r.failure_count()).sum()
    }

    /// Execute the block tests.
    async fn execute(mut self) -> Result<TestBlocks, Error> {
        let historic_types = Arc::new(self.chain_types.load());
        let urls = Arc::new(std::mem::take(&mut self.urls));
        let num_connections = self.connections.min(self.blocks.len());
        let total_blocks = self.blocks.len();

        // Create a shared index into the blocks list
        let next_block_idx = Arc::new(AtomicU64::new(0));
        let blocks = Arc::new(std::mem::take(&mut self.blocks));

        // Channel for collecting results
        let (tx, mut rx) = mpsc::channel::<(usize, BlockTestResult)>(num_connections * 2);

        // Spawn worker tasks
        for worker_idx in 0..num_connections {
            let urls = urls.clone();
            let blocks = blocks.clone();
            let next_block_idx = next_block_idx.clone();
            let historic_types = historic_types.clone();
            let tx = tx.clone();

            tokio::spawn(async move {
                // Each worker creates its own connection
                let mut state = match RpcTestState::new(urls.clone(), worker_idx).await {
                    Ok(s) => s,
                    Err(_) => return,
                };

                loop {
                    // Get next block index
                    let idx = next_block_idx.fetch_add(1, Ordering::Relaxed) as usize;
                    if idx >= blocks.len() {
                        break;
                    }

                    let block_number = blocks[idx];
                    let block_result =
                        test_single_block_with_retry(block_number, &mut state, &historic_types)
                            .await;

                    if tx.send((idx, block_result)).await.is_err() {
                        break;
                    }
                }
            });
        }

        // Drop our sender so rx completes when all workers are done
        drop(tx);

        // Collect results (may arrive out of order)
        let mut results_map: HashMap<usize, BlockTestResult> = HashMap::new();
        let mut debug_seen_specs = HashSet::new();
        let debug_enabled = std::env::var("FRAME_DECODE_TEST_DEBUG")
            .ok()
            .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));
        let mut tested_blocks = 0usize;
        let mut extrinsics_tested = 0usize;
        let mut last_log = Instant::now();
        let log_every = Duration::from_secs(30);
        while let Some((idx, result)) = rx.recv().await {
            tested_blocks += 1;
            extrinsics_tested += result.extrinsics.len();
            results_map.insert(idx, result);

            if debug_enabled {
                let is_new_spec = debug_seen_specs.insert(results_map[&idx].spec_version);
                let has_failures = results_map[&idx].failure_count() > 0;
                if is_new_spec || has_failures {
                    let block = &results_map[&idx];
                    eprintln!(
                        "[debug-progress] block={} hash={:?} spec_version={} extrinsics={} failures={}",
                        block.block_number,
                        block.block_hash,
                        block.spec_version,
                        block.extrinsics.len(),
                        block.failure_count(),
                    );
                }
            }

            if last_log.elapsed() >= log_every {
                eprintln!(
                    "[progress] blocks tested={tested_blocks}/{total_blocks} extrinsics={extrinsics_tested}"
                );
                last_log = Instant::now();
            }
        }

        // Sort results back into order
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

fn block_level_failure(block_number: u64, spec_version_hint: u32, error: Error) -> BlockTestResult {
    BlockTestResult {
        block_number,
        block_hash: H256::from([0u8; 32]),
        spec_version: if spec_version_hint == u32::MAX {
            0
        } else {
            spec_version_hint
        },
        extrinsics: vec![ExtrinsicTestResult::Failure {
            error: format!("Failed to test block {block_number}: {error}"),
            raw_bytes: "0x".to_string(),
        }],
    }
}

async fn test_single_block_with_retry(
    block_number: u64,
    state: &mut RpcTestState,
    historic_types: &Arc<ChainTypeRegistry>,
) -> BlockTestResult {
    const MAX_ATTEMPTS: usize = 5;

    let mut last_err: Option<Error> = None;

    for attempt in 0..MAX_ATTEMPTS {
        match test_single_block(block_number, state, historic_types).await {
            Ok(ok) => return ok,
            Err(e) => {
                last_err = Some(e);

                // Reconnect on transient RPC/connection failures.
                if RpcTestState::is_transient(last_err.as_ref().unwrap()) {
                    state.recover_from_transient().await;
                }

                // Exponential backoff with a cap.
                let backoff_ms = (200u64 << attempt).min(2000);
                sleep(Duration::from_millis(backoff_ms)).await;
            }
        }
    }

    block_level_failure(
        block_number,
        state.current_spec_version,
        last_err.unwrap_or_else(|| Error::RpcError("unknown error".into())),
    )
}

/// Test a single block.
async fn test_single_block(
    block_number: u64,
    state: &mut RpcTestState,
    historic_types: &Arc<ChainTypeRegistry>,
) -> Result<BlockTestResult, Error> {
    // Check if we need to update metadata (runtime updates take effect the block after)
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

    // Update metadata if spec version changed
    if spec_version != state.current_spec_version
        || state.current_metadata.is_none()
        || state.current_types_for_spec.is_none()
    {
        let metadata = state.rpc.get_metadata(Some(runtime_update_hash)).await?;

        // Prepare historic type info for this spec version
        let mut types_for_spec = historic_types
            .for_spec_version(spec_version as u64)
            .to_owned();

        // Extend with types from metadata so things like utility.batch work
        if let Ok(metadata_types) =
            frame_decode::helpers::type_registry_from_metadata_any(&metadata)
        {
            types_for_spec.prepend(metadata_types);
        }

        state.current_types_for_spec = Some(types_for_spec);
        state.current_metadata = Some(metadata);
        state.current_spec_version = spec_version;
    }

    // Get the block
    let block_hash = state
        .rpc
        .get_block_hash(block_number)
        .await?
        .ok_or(Error::BlockNotFound(block_number))?;

    let extrinsics_bytes = state.rpc.get_block_body(block_hash).await?;

    // Decode each extrinsic
    let mut extrinsic_results = Vec::new();

    let metadata = state.current_metadata.as_ref().unwrap();
    let types_for_spec = state.current_types_for_spec.as_ref().unwrap();

    for (idx, ext_bytes) in extrinsics_bytes.iter().enumerate() {
        let result =
            decode_extrinsic_to_result(&ext_bytes.0, metadata, types_for_spec, block_number, idx);
        extrinsic_results.push(result);
    }

    Ok(BlockTestResult {
        block_number,
        block_hash,
        spec_version,
        extrinsics: extrinsic_results,
    })
}

/// Decode an extrinsic and convert to a test result.
fn decode_extrinsic_to_result(
    bytes: &[u8],
    metadata: &RuntimeMetadata,
    historic_types: &TypeRegistrySet,
    block_number: u64,
    extrinsic_index: usize,
) -> ExtrinsicTestResult {
    let result = with_metadata_versioned!(metadata, historic_types, |m, resolver| {
        decode_extrinsic_inner(bytes, m, resolver)
    });

    match result {
        Ok(decoded) => ExtrinsicTestResult::Success(decoded),
        Err(error) => ExtrinsicTestResult::Failure {
            error: format!(
                "Block {}, extrinsic {}: {}",
                block_number, extrinsic_index, error
            ),
            raw_bytes: format!("0x{}", hex::encode(bytes)),
        },
    }
}

/// Inner function to decode an extrinsic with specific type info.
fn decode_extrinsic_inner<Info, Resolver>(
    bytes: &[u8],
    info: &Info,
    type_resolver: &Resolver,
) -> Result<DecodedExtrinsic, String>
where
    Info: frame_decode::extrinsics::ExtrinsicTypeInfo,
    Info::TypeId: Clone + core::fmt::Display + core::fmt::Debug + Send + Sync + 'static,
    Resolver: TypeResolver<TypeId = Info::TypeId>,
{
    let cursor = &mut &*bytes;
    let extrinsic_info = frame_decode::extrinsics::decode_extrinsic(cursor, info, type_resolver)
        .map_err(|e| format!("{e}"))?;

    // Decode each call data argument into a Value<String>
    let args = extrinsic_info
        .call_data()
        .map(|arg| {
            let decoded_arg = scale_value::scale::decode_as_type(
                &mut &bytes[arg.range()],
                arg.ty().clone(),
                type_resolver,
            )
            .map_err(|e| format!("Failed to decode arg '{}': {e}", arg.name()))?
            .map_context(|ctx| ctx.to_string());

            Ok(DecodedArg {
                name: arg.name().to_owned(),
                value: decoded_arg,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    // Check for leftover bytes
    if !cursor.is_empty() {
        return Err(format!(
            "{} leftover bytes after decoding {}.{}",
            cursor.len(),
            extrinsic_info.pallet_name(),
            extrinsic_info.call_name()
        ));
    }

    let is_signed = extrinsic_info.signature_payload().is_some();

    Ok(DecodedExtrinsic {
        pallet_name: extrinsic_info.pallet_name().to_owned(),
        call_name: extrinsic_info.call_name().to_owned(),
        is_signed,
        args,
    })
}
