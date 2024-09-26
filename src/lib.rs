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
        decode_extrinsic, Extrinsic, ExtrinsicDecodeError, ExtrinsicOwned, ExtrinsicSignature,
    };
    pub use crate::decoding::extrinsic_type_info::{
        ExtrinsicInfo, ExtrinsicInfoArg, ExtrinsicInfoError, ExtrinsicSignatureInfo,
        ExtrinsicTypeInfo,
    };

    /// Decode an extrinsic in a modern runtime (ie one exposing V14+ metadata). Each part is denoted by
    /// a byte range and type ID, which can then be decoded into a value.
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

    /// Decode an extrinsic in a historic runtime (ie one prior to V14 metadata). Each part is denoted by
    /// a byte range and type ID, which can then be decoded into a value.
    pub fn decode_extrinsic_legacy<'scale, 'info, 'resolver, Info, Resolver>(
        cursor: &mut &'scale [u8],
        info: &'info Info,
        type_resolver: &'resolver Resolver,
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

    /// Decode a storage key in a modern runtime (ie one exposing V14+ metadata) by breaking it down into its
    /// constituent parts. Each part is denoted by a hasher type, hasher byte range, and if possible, value
    /// information which can then be decoded into a value.
    pub fn decode_storage_key_current<T>(
        pallet_name: &str,
        storage_entry: &str,
        cursor: &mut &[u8],
        metadata: &T,
    ) -> Result<
        StorageKey<<T::Info as StorageTypeInfo>::TypeId>,
        StorageKeyDecodeError<<T::Info as StorageTypeInfo>::TypeId>,
    >
    where
        T: InfoAndResolver,
        T::Info: StorageTypeInfo,
        <T::Info as StorageTypeInfo>::TypeId: core::fmt::Debug + Clone,
        T::Resolver: TypeResolver<TypeId = <T::Info as StorageTypeInfo>::TypeId>,
    {
        decode_storage_key(
            pallet_name,
            storage_entry,
            cursor,
            metadata.info(),
            metadata.resolver(),
        )
    }

    /// Decode a storage key in a historic runtime (ie one prior to V14 metadata) by breaking it down into its
    /// constituent parts. Each part is denoted by a hasher type, hasher byte range, and if possible, value
    /// information which can then be decoded into a value.
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

    /// Decode a storage value in a modern runtime (ie one exposing V14+ metadata).
    pub fn decode_storage_value_current<'scale, 'resolver, T, V>(
        pallet_name: &str,
        storage_entry: &str,
        cursor: &mut &'scale [u8],
        metadata: &'resolver T,
        visitor: V,
    ) -> Result<
        V::Value<'scale, 'resolver>,
        StorageValueDecodeError<<T::Info as StorageTypeInfo>::TypeId>,
    >
    where
        T: InfoAndResolver,
        T::Info: StorageTypeInfo,
        <T::Info as StorageTypeInfo>::TypeId: core::fmt::Debug + Clone,
        T::Resolver:
            scale_type_resolver::TypeResolver<TypeId = <T::Info as StorageTypeInfo>::TypeId>,
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

    /// Decode a storage value in a historic runtime (ie one prior to V14 metadata).
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
