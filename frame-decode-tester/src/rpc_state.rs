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

//! Shared RPC connection state for block and storage testing.

use crate::Error;
use crate::rpc::SubstrateRpc;
use frame_metadata::RuntimeMetadata;
use scale_info_legacy::TypeRegistrySet;
use std::sync::Arc;

/// Shared state for RPC-based testing (used by both block and storage testers).
pub(crate) struct RpcTestState {
    pub urls: Arc<Vec<String>>,
    pub url_idx: usize,
    pub url: String,
    pub rpc: SubstrateRpc,
    pub current_spec_version: u32,
    pub current_metadata: Option<RuntimeMetadata>,
    pub current_types_for_spec: Option<TypeRegistrySet<'static>>,
}

impl RpcTestState {
    /// Create a new RPC test state, connecting to the given URL.
    pub async fn new(urls: Arc<Vec<String>>, url_idx: usize) -> Result<Self, Error> {
        let url = urls[url_idx % urls.len()].clone();
        let rpc = SubstrateRpc::connect(&url).await?;
        Ok(Self {
            urls,
            url_idx,
            url,
            rpc,
            current_spec_version: u32::MAX,
            current_metadata: None,
            current_types_for_spec: None,
        })
    }

    /// Check if an error is transient (retryable).
    pub fn is_transient(err: &Error) -> bool {
        match err {
            Error::ConnectionFailed(_) => true,
            Error::RpcError(msg) => {
                let msg = msg.to_ascii_lowercase();
                msg.contains("timeout")
                    || msg.contains("timed out")
                    || msg.contains("429")
                    || msg.contains("too many requests")
                    || msg.contains("connection closed")
                    || msg.contains("restart required")
                    || msg.contains("temporarily unavailable")
            }
            _ => false,
        }
    }

    /// Try to rotate to a different RPC URL. Returns true if successful.
    pub async fn rotate_rpc(&mut self) -> bool {
        if self.urls.is_empty() {
            return false;
        }

        let tries = self.urls.len();
        for _ in 0..tries {
            self.url_idx = (self.url_idx + 1) % self.urls.len();
            let url = self.urls[self.url_idx].clone();
            if let Ok(rpc) = SubstrateRpc::connect(&url).await {
                self.url = url;
                self.rpc = rpc;
                self.current_metadata = None;
                self.current_types_for_spec = None;
                self.current_spec_version = u32::MAX;
                return true;
            }
        }
        false
    }

    /// Try to reconnect to the same URL. Returns true if successful.
    pub async fn reconnect_same_url(&mut self) -> bool {
        if let Ok(rpc) = SubstrateRpc::connect(&self.url).await {
            self.rpc = rpc;
            self.current_metadata = None;
            self.current_types_for_spec = None;
            self.current_spec_version = u32::MAX;
            return true;
        }
        false
    }

    /// Attempt recovery from a transient error (rotate or reconnect).
    pub async fn recover_from_transient(&mut self) {
        if !self.rotate_rpc().await {
            let _ = self.reconnect_same_url().await;
        }
    }
}
