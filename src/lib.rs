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

//! Decode extrinsics and storage values from substrate based networks which expose `frame-metadata::RuntimeMetadata`
//! like Polkadot.
//!
//! - See [`extrinsics`] for decoding Extrinsics.
//! - See [`storage`] for decoding storage keys and values.
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
    //! - See [`decode_extrinsic_current`] for a helper to decode modern extrinsics.
    //!

    use crate::utils::InfoAndResolver;

    pub use crate::methods::extrinsic_decoder::{
        decode_extrinsic, Extrinsic, ExtrinsicDecodeError, ExtrinsicExtensions, ExtrinsicOwned,
        ExtrinsicSignature, ExtrinsicType, NamedArg,
    };
    pub use crate::methods::extrinsic_type_info::{
        ExtrinsicCallInfo, ExtrinsicExtensionInfo, ExtrinsicInfoArg, ExtrinsicInfoError,
        ExtrinsicSignatureInfo, ExtrinsicTypeInfo,
    };

    /// Decode an extrinsic in a modern runtime (ie one exposing V14+ metadata).
    ///
    /// See [`decode_extrinsic`] for a more comprehensive example.
    ///
    /// # Example
    ///
    /// ```rust
    /// use frame_decode::extrinsics::decode_extrinsic_current;
    /// use frame_metadata::RuntimeMetadata;
    /// use parity_scale_codec::Decode;
    ///
    /// let metadata_bytes = std::fs::read("artifacts/metadata_10000000_9180.scale").unwrap();
    /// let RuntimeMetadata::V14(metadata) = RuntimeMetadata::decode(&mut &*metadata_bytes).unwrap() else { panic!() };
    ///
    /// let extrinsics_bytes = std::fs::read("artifacts/exts_10000000_9180.json").unwrap();
    /// let extrinsics_hex: Vec<String> = serde_json::from_slice(&extrinsics_bytes).unwrap();
    ///
    /// for ext_hex in extrinsics_hex {
    ///     let ext_bytes = hex::decode(ext_hex.trim_start_matches("0x")).unwrap();
    ///
    ///     // Decode the extrinsic, returning information about it:
    ///     let ext_info = decode_extrinsic_current(&mut &*ext_bytes, &metadata).unwrap();
    ///
    ///     // Now we can use this information to inspect the extrinsic and decode the
    ///     // different values inside it (see the `decode_extrinsic` docs).
    /// }
    /// ```
    pub fn decode_extrinsic_current<'info_and_resolver, T>(
        cursor: &mut &[u8],
        metadata: &'info_and_resolver T,
    ) -> Result<
        Extrinsic<'info_and_resolver, <T::Info as ExtrinsicTypeInfo>::TypeId>,
        ExtrinsicDecodeError,
    >
    where
        T: InfoAndResolver,
        T::Info: ExtrinsicTypeInfo,
        <T::Info as ExtrinsicTypeInfo>::TypeId: core::fmt::Debug + Clone,
        T::Resolver:
            scale_type_resolver::TypeResolver<TypeId = <T::Info as ExtrinsicTypeInfo>::TypeId>,
    {
        decode_extrinsic(cursor, metadata.info(), metadata.resolver())
    }
}

pub mod storage {
    //! This module contains functions for decoding storage keys and values.
    //!
    //! - See [`decode_storage_key`] and [`decode_storage_value`] to decode storage keys or values
    //!   from modern or historic runtimes.
    //! - See [`decode_storage_key_current`] and [`decode_storage_value_current`] to decode modern
    //!   storage keys and values.
    //! - See [`encode_prefix`] to encode storage prefixes, and [`encode_storage_key`] to encode
    //!   storage keys.

    use crate::utils::InfoAndResolver;
    use scale_decode::Visitor;
    use scale_type_resolver::TypeResolver;

    pub use crate::methods::storage_type_info::{
        StorageHasher, StorageInfo, StorageInfoError, StorageKeyInfo, StorageTypeInfo,
    };

    pub use crate::methods::storage_decoder::{
        decode_storage_key, decode_storage_key_with_info, decode_storage_value,
        decode_storage_value_with_info, StorageKey, StorageKeyDecodeError, StorageKeyPart,
        StorageKeyPartValue, StorageValueDecodeError,
    };
    pub use crate::methods::storage_encoder::{
        encode_prefix, encode_storage_key, encode_storage_key_to, encode_storage_key_with_info_to,
        IntoStorageKeys, StorageKeyEncodeError, StorageKeys,
    };

    type TypeIdOf<T> = <<T as InfoAndResolver>::Info as StorageTypeInfo>::TypeId;

    /// Decode a storage key in a modern runtime, returning information about it.
    ///
    /// This information can be used to identify and, where possible, decode the parts of the storage key.
    ///
    /// See [`decode_storage_key`] for a more complete example.
    ///
    /// # Example
    ///
    /// Here, we decode some storage keys from a block.
    ///
    /// ```rust
    /// use frame_decode::storage::decode_storage_key_current;
    /// use frame_decode::helpers::decode_with_visitor;
    /// use frame_metadata::RuntimeMetadata;
    /// use parity_scale_codec::Decode;
    /// use scale_value::scale::ValueVisitor;
    ///
    /// let metadata_bytes = std::fs::read("artifacts/metadata_10000000_9180.scale").unwrap();
    /// let RuntimeMetadata::V14(metadata) = RuntimeMetadata::decode(&mut &*metadata_bytes).unwrap() else { return };
    ///
    /// let storage_keyval_bytes = std::fs::read("artifacts/storage_10000000_9180_system_account.json").unwrap();
    /// let storage_keyval_hex: Vec<(String, String)> = serde_json::from_slice(&storage_keyval_bytes).unwrap();
    ///
    /// for (key, _val) in storage_keyval_hex {
    ///     let key_bytes = hex::decode(key.trim_start_matches("0x")).unwrap();
    ///
    ///     // Decode the storage key, returning information about it:
    ///     let storage_info = decode_storage_key_current(
    ///         "System",
    ///         "Account",
    ///         &mut &*key_bytes,
    ///         &metadata,
    ///     ).unwrap();
    ///
    ///     // See `decode_storage_key` for more.
    /// }
    /// ```
    pub fn decode_storage_key_current<T>(
        pallet_name: &str,
        storage_entry: &str,
        cursor: &mut &[u8],
        metadata: &T,
    ) -> Result<StorageKey<TypeIdOf<T>>, StorageKeyDecodeError<TypeIdOf<T>>>
    where
        T: InfoAndResolver,
        T::Info: StorageTypeInfo,
        TypeIdOf<T>: core::fmt::Debug + Clone,
        T::Resolver: TypeResolver<TypeId = TypeIdOf<T>>,
    {
        decode_storage_key(
            pallet_name,
            storage_entry,
            cursor,
            metadata.info(),
            metadata.resolver(),
        )
    }

    /// Decode a storage value in a modern (V14-metadata-or-later) runtime.
    ///
    /// # Example
    ///
    /// Here, we decode some storage values from a block.
    ///
    /// ```rust
    /// use frame_decode::storage::decode_storage_value_current;
    /// use frame_decode::helpers::decode_with_visitor;
    /// use frame_metadata::RuntimeMetadata;
    /// use parity_scale_codec::Decode;
    /// use scale_value::scale::ValueVisitor;
    ///
    /// let metadata_bytes = std::fs::read("artifacts/metadata_10000000_9180.scale").unwrap();
    /// let RuntimeMetadata::V14(metadata) = RuntimeMetadata::decode(&mut &*metadata_bytes).unwrap() else { return };
    ///
    /// let storage_keyval_bytes = std::fs::read("artifacts/storage_10000000_9180_system_account.json").unwrap();
    /// let storage_keyval_hex: Vec<(String, String)> = serde_json::from_slice(&storage_keyval_bytes).unwrap();
    ///
    /// for (_key, val) in storage_keyval_hex {
    ///     let value_bytes = hex::decode(val.trim_start_matches("0x")).unwrap();
    ///
    ///     // Decode the storage value, here into a scale_value::Value:
    ///     let account_value = decode_storage_value_current(
    ///         "System",
    ///         "Account",
    ///         &mut &*value_bytes,
    ///         &metadata,
    ///         ValueVisitor::new()
    ///     ).unwrap();
    /// }
    /// ```
    pub fn decode_storage_value_current<'scale, 'resolver, T, V>(
        pallet_name: &str,
        storage_entry: &str,
        cursor: &mut &'scale [u8],
        metadata: &'resolver T,
        visitor: V,
    ) -> Result<V::Value<'scale, 'resolver>, StorageValueDecodeError<TypeIdOf<T>>>
    where
        T: InfoAndResolver,
        T::Info: StorageTypeInfo,
        TypeIdOf<T>: core::fmt::Debug + Clone,
        T::Resolver: scale_type_resolver::TypeResolver<TypeId = TypeIdOf<T>>,
        V: Visitor<TypeResolver = T::Resolver>,
        V::Error: core::fmt::Debug,
    {
        decode_storage_value(
            pallet_name,
            storage_entry,
            cursor,
            metadata.info(),
            metadata.resolver(),
            visitor,
        )
    }
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
    //! - [`list_storage_entries`] returns an iterator over all of the storage entries available in
    //!   some metadata.
    //!

    pub use crate::utils::{decode_with_error_tracing, DecodeErrorTrace};
    pub use crate::utils::{list_storage_entries, list_storage_entries_any, StorageEntry};
    #[cfg(feature = "legacy")]
    pub use crate::utils::{type_registry_from_metadata, type_registry_from_metadata_any};

    /// An alias to [`scale_decode::visitor::decode_with_visitor`]. This can be used to decode the byte ranges
    /// given back from functions like [`crate::extrinsics::decode_extrinsic_current`] or
    /// [`crate::storage::decode_storage_key_current`].
    ///
    pub use scale_decode::visitor::decode_with_visitor;

    /// An alias to the underlying [`scale-decode`] crate.
    ///
    pub use scale_decode;
}

#[cfg(all(test, feature = "legacy"))]
mod test {
    use crate::methods::extrinsic_type_info::ExtrinsicTypeInfo;
    use crate::methods::storage_type_info::StorageTypeInfo;
    use crate::utils::{InfoAndResolver, ToStorageEntriesList, ToTypeRegistry};

    // This will panic if there is any issue decoding the legacy types we provide.
    #[cfg(all(test, feature = "legacy-types"))]
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
        impls_trait!(frame_metadata::v14::RuntimeMetadataV14, InfoAndResolver);
        impls_trait!(frame_metadata::v15::RuntimeMetadataV15, InfoAndResolver);
        impls_trait!(frame_metadata::v16::RuntimeMetadataV16, InfoAndResolver);

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

        impls_trait!(frame_metadata::v8::RuntimeMetadataV8, ToStorageEntriesList);
        impls_trait!(frame_metadata::v9::RuntimeMetadataV9, ToStorageEntriesList);
        impls_trait!(frame_metadata::v10::RuntimeMetadataV10, ToStorageEntriesList);
        impls_trait!(frame_metadata::v11::RuntimeMetadataV11, ToStorageEntriesList);
        impls_trait!(frame_metadata::v12::RuntimeMetadataV12, ToStorageEntriesList);
        impls_trait!(frame_metadata::v13::RuntimeMetadataV13, ToStorageEntriesList);
        impls_trait!(frame_metadata::v14::RuntimeMetadataV14, ToStorageEntriesList);
        impls_trait!(frame_metadata::v15::RuntimeMetadataV15, ToStorageEntriesList);
        impls_trait!(frame_metadata::v16::RuntimeMetadataV16, ToStorageEntriesList);

        // This is a legacy trait and so only legacy metadata versions implement it:
        impls_trait!(frame_metadata::v8::RuntimeMetadataV8, ToTypeRegistry);
        impls_trait!(frame_metadata::v9::RuntimeMetadataV9, ToTypeRegistry);
        impls_trait!(frame_metadata::v10::RuntimeMetadataV10, ToTypeRegistry);
        impls_trait!(frame_metadata::v11::RuntimeMetadataV11, ToTypeRegistry);
        impls_trait!(frame_metadata::v12::RuntimeMetadataV12, ToTypeRegistry);
        impls_trait!(frame_metadata::v13::RuntimeMetadataV13, ToTypeRegistry);
    };
}
