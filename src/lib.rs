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

//! Decode extrinsics and storage values from substrate based networks like Polkadot.
#![deny(missing_docs)]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod decoding;
mod utils;

/// This module contains functions for decoding extrinsics.
pub mod extrinsics {
    use crate::utils::InfoAndResolver;
    use scale_type_resolver::TypeResolver;

    pub use crate::decoding::extrinsic_decoder::{
        decode_extrinsic, Extrinsic, ExtrinsicDecodeError, ExtrinsicExtensions, ExtrinsicOwned,
        ExtrinsicSignature, ExtrinsicType,
    };
    pub use crate::decoding::extrinsic_type_info::{
        ExtrinsicInfo, ExtrinsicInfoArg, ExtrinsicInfoError, ExtrinsicSignatureInfo,
        ExtrinsicTypeInfo,
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

    /// Decode an extrinsic in a historic runtime (ie one prior to V14 metadata). This is basically
    /// just an alias for [`decode_extrinsic`].
    ///
    /// To understand more about the historic types required to decode old blocks, see [`scale_info_legacy`].
    ///
    /// # Example
    ///
    /// ```rust
    /// use frame_decode::extrinsics::decode_extrinsic_legacy;
    /// use frame_metadata::RuntimeMetadata;
    /// use parity_scale_codec::Decode;
    /// use scale_info_legacy::ChainTypeRegistry;
    ///
    /// let metadata_bytes = std::fs::read("artifacts/metadata_5000000_30.scale").unwrap();
    /// let RuntimeMetadata::V12(metadata) = RuntimeMetadata::decode(&mut &*metadata_bytes).unwrap() else { panic!() };
    ///
    /// let extrinsics_bytes = std::fs::read("artifacts/exts_5000000_30.json").unwrap();
    /// let extrinsics_hex: Vec<String> = serde_json::from_slice(&extrinsics_bytes).unwrap();
    ///
    /// // For historic types, we also need to provide type definitions, since they aren't in the
    /// // metadata. We use scale-info-legacy to do this, and have already defined types for the
    /// // Polkadot relay chain, so let's load those in:
    /// let historic_type_bytes = std::fs::read("types/polkadot_types.yaml").unwrap();
    /// let historic_types: ChainTypeRegistry = serde_yaml::from_slice(&historic_type_bytes).unwrap();
    ///
    /// // We configure the loaded types for the spec version of the extrinsics we want to decode,
    /// // because types can vary between different spec versions.
    /// let mut historic_types_for_spec = historic_types.for_spec_version(30);
    ///
    /// // We also want to embelish these types with information from the metadata itself. This avoids
    /// // needing to hardcode a load of type definitions that we can already construct from the metadata.
    /// let types_from_metadata = frame_decode::helpers::type_registry_from_metadata(&metadata).unwrap();
    /// historic_types_for_spec.prepend(types_from_metadata);
    ///
    /// for ext_hex in extrinsics_hex {
    ///     let ext_bytes = hex::decode(ext_hex.trim_start_matches("0x")).unwrap();
    ///
    ///     // Decode the extrinsic, returning information about it:
    ///     let ext_info = decode_extrinsic_legacy(&mut &*ext_bytes, &metadata, &historic_types_for_spec).unwrap();
    ///
    ///     // Now we can use this information to inspect the extrinsic and decode the
    ///     // different values inside it (see the `decode_extrinsic` docs).
    /// }
    /// ```
    pub fn decode_extrinsic_legacy<'info, Info, Resolver>(
        cursor: &mut &[u8],
        info: &'info Info,
        type_resolver: &Resolver,
    ) -> Result<Extrinsic<'info, Info::TypeId>, ExtrinsicDecodeError>
    where
        Info: ExtrinsicTypeInfo,
        Info::TypeId: core::fmt::Debug + Clone,
        Resolver: TypeResolver<TypeId = Info::TypeId>,
    {
        decode_extrinsic(cursor, info, type_resolver)
    }
}

/// This module contains functions for decoding storage keys and values.
pub mod storage {
    use crate::utils::InfoAndResolver;
    use scale_decode::Visitor;
    use scale_type_resolver::TypeResolver;

    pub use crate::decoding::storage_type_info::{
        StorageHasher, StorageInfo, StorageInfoError, StorageKeyInfo, StorageTypeInfo,
    };

    pub use crate::decoding::storage_decoder::{
        decode_storage_key, decode_storage_value, StorageKey, StorageKeyDecodeError,
        StorageKeyPart, StorageKeyPartValue, StorageValueDecodeError,
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

    /// Decode a storage key in a historic (pre-V14-metadata) runtime, returning information about it.
    ///
    /// This information can be used to identify and, where possible, decode the parts of the storage key.
    ///
    /// This is basically just an alias for [`decode_storage_key`]. See that for a more complete example.
    ///
    /// # Example
    ///
    /// Here, we decode some storage keys from a block.
    ///
    /// ```rust
    /// use frame_decode::storage::decode_storage_key_legacy;
    /// use frame_metadata::RuntimeMetadata;
    /// use parity_scale_codec::Decode;
    /// use scale_info_legacy::ChainTypeRegistry;
    ///
    /// let metadata_bytes = std::fs::read("artifacts/metadata_5000000_30.scale").unwrap();
    /// let RuntimeMetadata::V12(metadata) = RuntimeMetadata::decode(&mut &*metadata_bytes).unwrap() else { panic!() };
    ///  
    /// let storage_keyval_bytes = std::fs::read("artifacts/storage_5000000_30_staking_validators.json").unwrap();
    /// let storage_keyval_hex: Vec<(String, String)> = serde_json::from_slice(&storage_keyval_bytes).unwrap();
    ///
    /// // For historic types, we also need to provide type definitions, since they aren't in the
    /// // metadata. We use scale-info-legacy to do this, and have already defined types for the
    /// // Polkadot relay chain, so let's load those in:
    /// let historic_type_bytes = std::fs::read("types/polkadot_types.yaml").unwrap();
    /// let historic_types: ChainTypeRegistry = serde_yaml::from_slice(&historic_type_bytes).unwrap();
    ///
    /// // We configure the loaded types for the spec version of the extrinsics we want to decode,
    /// // because types can vary between different spec versions.
    /// let mut historic_types_for_spec = historic_types.for_spec_version(30);
    ///
    /// // We also want to embelish these types with information from the metadata itself. This avoids
    /// // needing to hardcode a load of type definitions that we can already construct from the metadata.
    /// let types_from_metadata = frame_decode::helpers::type_registry_from_metadata(&metadata).unwrap();
    /// historic_types_for_spec.prepend(types_from_metadata);
    ///
    /// for (key, _val) in storage_keyval_hex {
    ///     let key_bytes = hex::decode(key.trim_start_matches("0x")).unwrap();
    ///
    ///     // Decode the storage key, returning information about it:
    ///     let storage_info = decode_storage_key_legacy(
    ///         "Staking",
    ///         "Validators",
    ///         &mut &*key_bytes,
    ///         &metadata,
    ///         &historic_types_for_spec
    ///     ).unwrap();
    ///
    ///     // See `decode_storage_key` for more.
    /// }
    /// ```
    #[cfg(feature = "legacy")]
    pub fn decode_storage_key_legacy<Info, Resolver>(
        pallet_name: &str,
        storage_entry: &str,
        cursor: &mut &[u8],
        info: &Info,
        type_resolver: &Resolver,
    ) -> Result<StorageKey<Info::TypeId>, StorageKeyDecodeError<Info::TypeId>>
    where
        Info: StorageTypeInfo,
        Info::TypeId: Clone + core::fmt::Debug,
        Resolver: TypeResolver<TypeId = Info::TypeId>,
    {
        decode_storage_key(pallet_name, storage_entry, cursor, info, type_resolver)
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

    /// Decode a storage value in a historic (pre-V14-metadata) runtime. This is basically
    /// just an alias for [`decode_storage_value`].
    ///
    /// # Example
    ///
    /// Here, we decode some storage values from a block.
    ///
    /// ```rust
    /// use frame_decode::storage::decode_storage_value_legacy;
    /// use frame_metadata::RuntimeMetadata;
    /// use parity_scale_codec::Decode;
    /// use scale_info_legacy::ChainTypeRegistry;
    /// use scale_value::scale::ValueVisitor;
    ///
    /// let metadata_bytes = std::fs::read("artifacts/metadata_5000000_30.scale").unwrap();
    /// let RuntimeMetadata::V12(metadata) = RuntimeMetadata::decode(&mut &*metadata_bytes).unwrap() else { panic!() };
    ///  
    /// let storage_keyval_bytes = std::fs::read("artifacts/storage_5000000_30_staking_validators.json").unwrap();
    /// let storage_keyval_hex: Vec<(String, String)> = serde_json::from_slice(&storage_keyval_bytes).unwrap();
    ///
    /// // For historic types, we also need to provide type definitions, since they aren't in the
    /// // metadata. We use scale-info-legacy to do this, and have already defined types for the
    /// // Polkadot relay chain, so let's load those in:
    /// let historic_type_bytes = std::fs::read("types/polkadot_types.yaml").unwrap();
    /// let historic_types: ChainTypeRegistry = serde_yaml::from_slice(&historic_type_bytes).unwrap();
    ///
    /// // We configure the loaded types for the spec version of the extrinsics we want to decode,
    /// // because types can vary between different spec versions.
    /// let mut historic_types_for_spec = historic_types.for_spec_version(30);
    ///
    /// // We also want to embelish these types with information from the metadata itself. This avoids
    /// // needing to hardcode a load of type definitions that we can already construct from the metadata.
    /// let types_from_metadata = frame_decode::helpers::type_registry_from_metadata(&metadata).unwrap();
    /// historic_types_for_spec.prepend(types_from_metadata);
    ///
    /// for (_key, val) in storage_keyval_hex {
    ///     let value_bytes = hex::decode(val.trim_start_matches("0x")).unwrap();
    ///
    ///     // Decode the storage value, here into a scale_value::Value:
    ///     let account_value = decode_storage_value_legacy(
    ///         "Staking",
    ///         "Validators",
    ///         &mut &*value_bytes,
    ///         &metadata,
    ///         &historic_types_for_spec,
    ///         ValueVisitor::new()
    ///     ).unwrap();
    /// }
    /// ```    
    #[cfg(feature = "legacy")]
    pub fn decode_storage_value_legacy<'scale, 'resolver, Info, Resolver, V>(
        pallet_name: &str,
        storage_entry: &str,
        cursor: &mut &'scale [u8],
        info: &Info,
        type_resolver: &'resolver Resolver,
        visitor: V,
    ) -> Result<V::Value<'scale, 'resolver>, StorageValueDecodeError<Info::TypeId>>
    where
        Info: StorageTypeInfo,
        Info::TypeId: Clone + core::fmt::Debug,
        Resolver: TypeResolver<TypeId = Info::TypeId>,
        V: scale_decode::Visitor<TypeResolver = Resolver>,
        V::Error: core::fmt::Debug,
    {
        decode_storage_value(
            pallet_name,
            storage_entry,
            cursor,
            info,
            type_resolver,
            visitor,
        )
    }
}

/// Helper functions and types to assist with decoding.
pub mod helpers {
    #[cfg(feature = "legacy")]
    pub use crate::utils::type_registry_from_metadata;
    pub use crate::utils::{decode_with_error_tracing, DecodeErrorTrace};
    pub use crate::utils::{list_storage_entries, StorageEntry};

    /// An alias to [`scale_decode::visitor::decode_with_visitor`]. This can be used to decode the byte ranges
    /// given back from functions like [`crate::extrinsics::decode_extrinsic_current`] or
    /// [`crate::storage::decode_storage_key_current`].
    ///
    pub use scale_decode::visitor::decode_with_visitor;

    /// An alias to the underlying [`scale-decode`] crate.
    ///
    pub use scale_decode;
}
