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

//! Common types used across the crate.

use scale_info_legacy::ChainTypeRegistry;

/// Historic chain types configuration.
pub enum ChainTypes {
    /// Use Polkadot relay chain types.
    Polkadot,
    /// Use Kusama relay chain types.
    Kusama,
    /// Use Kusama Asset Hub types.
    KusamaAssetHub,
}

impl Default for ChainTypes {
    fn default() -> Self {
        ChainTypes::Polkadot
    }
}

impl ChainTypes {
    /// Load the chain type registry.
    pub fn load(&self) -> ChainTypeRegistry {
        match self {
            ChainTypes::Polkadot => frame_decode::legacy_types::polkadot::relay_chain(),
            ChainTypes::Kusama => frame_decode::legacy_types::kusama::relay_chain(),
            ChainTypes::KusamaAssetHub => frame_decode::legacy_types::kusama::asset_hub(),
        }
    }
}

/// A successfully decoded extrinsic.
#[derive(Debug, Clone)]
pub struct DecodedExtrinsic {
    /// The pallet name.
    pub pallet_name: String,
    /// The call name.
    pub call_name: String,
    /// Whether the extrinsic is signed.
    pub is_signed: bool,
    /// The decoded call arguments.
    pub args: Vec<DecodedArg>,
}

/// A decoded argument.
#[derive(Debug, Clone)]
pub struct DecodedArg {
    /// The argument name.
    pub name: String,
    /// The decoded value.
    pub value: scale_value::Value<String>,
}
