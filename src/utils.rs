// Copyright (C) 2022-2023 Parity Technologies (UK) Ltd. (admin@parity.io)
// This file is a part of the scale-value crate.
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

mod decode_with_error_tracing;
mod list_storage_entries;
mod type_registry_from_metadata;

pub use decode_with_error_tracing::{decode_with_error_tracing, DecodeErrorTrace};
pub use list_storage_entries::{list_storage_entries, list_storage_entries_any, StorageEntry};
#[cfg(feature = "legacy")]
pub use type_registry_from_metadata::{
    type_registry_from_metadata, type_registry_from_metadata_any,
};

// We don't want to expose these traits at the moment, but want to test them.
#[cfg(all(test, feature = "legacy"))]
pub use list_storage_entries::ToStorageEntriesList;
#[cfg(test)]
pub use type_registry_from_metadata::ToTypeRegistry;

/// A utility function to unwrap the `DecodeDifferent` enum found in earlier metadata versions.
#[cfg(feature = "legacy")]
pub fn as_decoded<A, B>(item: &frame_metadata::decode_different::DecodeDifferent<A, B>) -> &B {
    match item {
        frame_metadata::decode_different::DecodeDifferent::Encode(_a) => {
            panic!("Expecting decoded data")
        }
        frame_metadata::decode_different::DecodeDifferent::Decoded(b) => b,
    }
}

pub trait InfoAndResolver {
    type Info;
    type Resolver;

    fn info(&self) -> &Self::Info;
    fn resolver(&self) -> &Self::Resolver;
}

impl InfoAndResolver for frame_metadata::v14::RuntimeMetadataV14 {
    type Info = frame_metadata::v14::RuntimeMetadataV14;
    type Resolver = scale_info::PortableRegistry;

    fn info(&self) -> &Self::Info {
        self
    }
    fn resolver(&self) -> &Self::Resolver {
        &self.types
    }
}

impl InfoAndResolver for frame_metadata::v15::RuntimeMetadataV15 {
    type Info = frame_metadata::v15::RuntimeMetadataV15;
    type Resolver = scale_info::PortableRegistry;

    fn info(&self) -> &Self::Info {
        self
    }
    fn resolver(&self) -> &Self::Resolver {
        &self.types
    }
}

impl InfoAndResolver for frame_metadata::v16::RuntimeMetadataV16 {
    type Info = frame_metadata::v16::RuntimeMetadataV16;
    type Resolver = scale_info::PortableRegistry;

    fn info(&self) -> &Self::Info {
        self
    }
    fn resolver(&self) -> &Self::Resolver {
        &self.types
    }
}
