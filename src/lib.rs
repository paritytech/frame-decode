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

//! Decode extrinsics and storage values from substrate based networks which expose `frame-metadata::RuntimeMetadata`
//! like Polkadot.
//!
//! - See [`extrinsics`] for decoding Extrinsics.
//! - See [`storage`] for encoding/decoding storage keys and decoding values.
//! - See [`runtime_apis`] for encoding Runtime API inputs and decoding Runtime API responses
//! - See [`legacy_types`] to access historic type information for certain chains.
//!
#![deny(missing_docs)]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod methods;
mod utils;

pub mod extrinsics {
    //! This module contains functions for decoding extrinsics.
    //!
    //! - See [`decode_extrinsic`] for a general function to decode modern or historic extrinsics.
    //! - See [`ExtrinsicTypeInfo`] for the underlying trait which extracts the relevant information.

    pub use crate::methods::extrinsic_decoder::{
        Extrinsic, ExtrinsicDecodeError, ExtrinsicExtensions, ExtrinsicOwned, ExtrinsicSignature,
        ExtrinsicType, NamedArg, decode_extrinsic,
    };
    pub use crate::methods::extrinsic_type_info::{
        ExtrinsicCallInfo, ExtrinsicExtensionInfo, ExtrinsicInfoArg, ExtrinsicInfoError,
        ExtrinsicSignatureInfo, ExtrinsicTypeInfo,
    };
}

pub mod storage {
    //! This module contains functions for decoding storage keys and values.
    //!
    //! - See [`decode_storage_key`] and [`decode_storage_value`] to decode storage keys or values
    //!   from modern or historic runtimes.
    //! - See [`encode_prefix`] to encode storage prefixes, and [`encode_storage_key`] to encode
    //!   storage keys.
    //! - See [`StorageTypeInfo`] for the underlying trait which extracts the relevant information.

    pub use crate::methods::storage_decoder::{
        StorageKey, StorageKeyDecodeError, StorageKeyPart, StorageKeyPartValue,
        StorageValueDecodeError, decode_storage_key, decode_storage_key_values,
        decode_storage_key_with_info, decode_storage_value, decode_storage_value_with_info,
        decode_default_storage_value_with_info,
    };
    pub use crate::methods::storage_encoder::{
        StorageKeyEncodeError, encode_storage_key, encode_storage_key_prefix,
        encode_storage_key_suffix, encode_storage_key_suffix_to,
        encode_storage_key_suffix_with_info_to, encode_storage_key_to,
        encode_storage_key_with_info_to,
    };
    pub use crate::methods::storage_type_info::{
        StorageEntry, StorageHasher, StorageInfo, StorageInfoError, StorageKeyInfo, StorageTypeInfo,
    };
    pub use crate::utils::{
        DecodableValues, EncodableValues, IntoDecodableValues, IntoEncodableValues,
    };
}

pub mod runtime_apis {
    //! This module contains types and functions for working with Runtime APIs.
    //!
    //! - See [`encode_runtime_api_name`] and [`encode_runtime_api_inputs`] to encode
    //!   the name and inputs to make a Runtime API call.
    //! - See [`decode_runtime_api_response`] to decode Runtime API responses.
    //! - See [`RuntimeApiTypeInfo`] for the underlying trait which extracts the relevant information.

    pub use crate::methods::runtime_api_decoder::{
        RuntimeApiDecodeError, decode_runtime_api_response, decode_runtime_api_response_with_info,
    };
    pub use crate::methods::runtime_api_encoder::{
        RuntimeApiInputsEncodeError, encode_runtime_api_inputs, encode_runtime_api_inputs_to,
        encode_runtime_api_inputs_with_info_to, encode_runtime_api_name,
    };
    pub use crate::methods::runtime_api_type_info::{
        RuntimeApi, RuntimeApiInfo, RuntimeApiInfoError, RuntimeApiInput, RuntimeApiTypeInfo,
    };
    pub use crate::utils::{EncodableValues, IntoEncodableValues};
}

pub mod view_functions {
    //! This module contains types and functions for working with View Functions.
    //!
    //! - See [`RUNTIME_API_NAME`] and [`encode_view_function_inputs`] to obtain the Runtime API name
    //!   and the encoded input data required to call a given View Function.
    //! - See [`decode_view_function_response`] to decode View Function responses.
    //! - See [`ViewFunctionTypeInfo`] for the underlying trait which extracts the relevant information.

    pub use crate::methods::view_function_decoder::{
        ViewFunctionDecodeError, decode_view_function_response,
        decode_view_function_response_with_info,
    };
    pub use crate::methods::view_function_encoder::{
        RUNTIME_API_NAME, ViewFunctionInputsEncodeError, encode_view_function_inputs,
        encode_view_function_inputs_to, encode_view_function_inputs_with_info_to,
    };
    pub use crate::methods::view_function_type_info::{
        ViewFunction, ViewFunctionInfo, ViewFunctionInfoError, ViewFunctionInput,
        ViewFunctionTypeInfo,
    };
    pub use crate::utils::{EncodableValues, IntoEncodableValues};
}

#[cfg(feature = "legacy-types")]
pub mod legacy_types {
    //! This module contains legacy types that can be used to decode pre-V14 blocks and storage.

    pub mod polkadot {
        //! Legacy types for Polkadot chains.

        /// Legacy types for the Polkadot Relay Chain.
        pub fn relay_chain() -> scale_info_legacy::ChainTypeRegistry {
            // This is a convenience function to load the Polkadot relay chain types.
            // It is used in the examples in this crate.
            let bytes = include_bytes!("../types/polkadot_types.yaml");
            serde_yaml::from_slice(bytes).expect("Polkadot types are valid YAML")
        }
    }
}

pub mod helpers {
    //! Helper functions and types to assist with decoding.
    //!
    //! - [`type_registry_from_metadata`] is expected to be used when decoding things from historic
    //!   runtimes, adding the ability to decode some types from information in the metadata.
    //! - [`decode_with_error_tracing`] is like [`decode_with_visitor`], but
    //!   will use a tracing visitor (if the `error-tracing` feature is enabled) to provide more
    //!   information in the event that decoding fails.

    pub use crate::utils::{
        DecodableValues, DecodeErrorTrace, EncodableValues, IntoDecodableValues,
        IntoEncodableValues, decode_with_error_tracing, list_storage_entries_any,
    };
    #[cfg(feature = "legacy")]
    pub use crate::utils::{type_registry_from_metadata, type_registry_from_metadata_any};

    /// An alias to [`scale_decode::visitor::decode_with_visitor`]. This can be used to decode the byte ranges
    /// given back from functions like [`crate::extrinsics::decode_extrinsic_current`] or
    /// [`crate::storage::decode_storage_key_current`].
    pub use scale_decode::visitor::decode_with_visitor;

    /// An alias to the underlying [`scale-decode`] crate.
    pub use scale_decode;
}

#[cfg(test)]
mod test {
    use crate::methods::extrinsic_type_info::ExtrinsicTypeInfo;
    use crate::methods::runtime_api_type_info::RuntimeApiTypeInfo;
    use crate::methods::storage_type_info::StorageTypeInfo;
    use crate::methods::view_function_type_info::ViewFunctionTypeInfo;
    use crate::utils::ToTypeRegistry;

    // This will panic if there is any issue decoding the legacy types we provide.
    #[test]
    fn test_deserializing_legacy_types() {
        let _ = crate::legacy_types::polkadot::relay_chain();
    }

    macro_rules! impls_trait {
        ($type:ty, $trait:path) => {
            const _: () = {
                const fn assert_impl<T: $trait>() {}
                assert_impl::<$type>();
            };
        };
    }

    // Just a sanity check that all of the metadata versions we expect implement
    // all of the key traits. Makes it harder to miss something when adding a new metadata
    // version; just add it below and implement the traits until everything compiles.
    #[rustfmt::skip]
    const _: () = {
        impls_trait!(frame_metadata::v8::RuntimeMetadataV8, ExtrinsicTypeInfo);
        impls_trait!(frame_metadata::v9::RuntimeMetadataV9, ExtrinsicTypeInfo);
        impls_trait!(frame_metadata::v10::RuntimeMetadataV10, ExtrinsicTypeInfo);
        impls_trait!(frame_metadata::v11::RuntimeMetadataV11, ExtrinsicTypeInfo);
        impls_trait!(frame_metadata::v12::RuntimeMetadataV12, ExtrinsicTypeInfo);
        impls_trait!(frame_metadata::v13::RuntimeMetadataV13, ExtrinsicTypeInfo);
        impls_trait!(frame_metadata::v14::RuntimeMetadataV14, ExtrinsicTypeInfo);
        impls_trait!(frame_metadata::v15::RuntimeMetadataV15, ExtrinsicTypeInfo);
        impls_trait!(frame_metadata::v16::RuntimeMetadataV16, ExtrinsicTypeInfo);

        impls_trait!(frame_metadata::v8::RuntimeMetadataV8, StorageTypeInfo);
        impls_trait!(frame_metadata::v9::RuntimeMetadataV9, StorageTypeInfo);
        impls_trait!(frame_metadata::v10::RuntimeMetadataV10, StorageTypeInfo);
        impls_trait!(frame_metadata::v11::RuntimeMetadataV11, StorageTypeInfo);
        impls_trait!(frame_metadata::v12::RuntimeMetadataV12, StorageTypeInfo);
        impls_trait!(frame_metadata::v13::RuntimeMetadataV13, StorageTypeInfo);
        impls_trait!(frame_metadata::v14::RuntimeMetadataV14, StorageTypeInfo);
        impls_trait!(frame_metadata::v15::RuntimeMetadataV15, StorageTypeInfo);
        impls_trait!(frame_metadata::v16::RuntimeMetadataV16, StorageTypeInfo);

        // Only V16+ metadata contains any view function information. Prior to this,
        // hardly any view functions existed. We _could_ extend our legacy type information
        // to support them if necessary, but it's unlikely it will be.
        impls_trait!(frame_metadata::v16::RuntimeMetadataV16, ViewFunctionTypeInfo);

        // Only V15+ metadata has Runtime API info in. For earlier, we lean on
        // our scale-Info-legacy type registry to provide the information.
        impls_trait!(scale_info_legacy::TypeRegistry, RuntimeApiTypeInfo);
        impls_trait!(scale_info_legacy::TypeRegistrySet, RuntimeApiTypeInfo);
        impls_trait!(frame_metadata::v15::RuntimeMetadataV15, RuntimeApiTypeInfo);
        impls_trait!(frame_metadata::v16::RuntimeMetadataV16, RuntimeApiTypeInfo);

        // This is a legacy trait and so only legacy metadata versions implement it:
        impls_trait!(frame_metadata::v8::RuntimeMetadataV8, ToTypeRegistry);
        impls_trait!(frame_metadata::v9::RuntimeMetadataV9, ToTypeRegistry);
        impls_trait!(frame_metadata::v10::RuntimeMetadataV10, ToTypeRegistry);
        impls_trait!(frame_metadata::v11::RuntimeMetadataV11, ToTypeRegistry);
        impls_trait!(frame_metadata::v12::RuntimeMetadataV12, ToTypeRegistry);
        impls_trait!(frame_metadata::v13::RuntimeMetadataV13, ToTypeRegistry);
    };
}
