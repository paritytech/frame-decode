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

mod decode_with_error_tracing;
mod either;
mod encodable_values;
mod list_storage_entries_any;
mod type_registry_from_metadata;

pub use encodable_values::{EncodableValues, IntoEncodableValues};
pub use list_storage_entries_any::list_storage_entries_any;

pub use decode_with_error_tracing::{decode_with_error_tracing, DecodeErrorTrace};
pub use either::Either;
#[cfg(feature = "legacy")]
pub use type_registry_from_metadata::{
    type_registry_from_metadata, type_registry_from_metadata_any,
};

// We don't want to expose these traits at the moment, but want to test them.
#[cfg(all(test, feature = "legacy"))]
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
