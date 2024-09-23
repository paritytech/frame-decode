//! Decode extrinsics and storage values from substrate based networks like Polkadot.
#![warn(missing_docs)]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod utils;
mod decoding;

pub use scale_decode::visitor::decode_with_visitor;

/// This module contains functions for decoding extrinsics.
pub mod extrinsics {
    use scale_type_resolver::TypeResolver;
    #[cfg(feature = "legacy")]
    use super::{AnyError, AnyTypeId};
    use crate::utils::InfoAndResolver;
    
    pub use crate::decoding::extrinsic_type_info::{
        ExtrinsicTypeInfo,
        ExtrinsicInfo,
        ExtrinsicSignatureInfo,
        ExtrinsicInfoArg,
        ExtrinsicInfoError,
    };
    pub use crate::decoding::extrinsic_decoder::{
        Extrinsic,
        ExtrinsicDecodeError,
        decode_extrinsic,
    };

    /// Decode an extrinsic by breaking it down into its constituent parts. Each part is denoted by
    /// a byte range and type ID, which can then be decoded into a value.
    #[cfg(feature = "legacy")]
    pub fn decode_extrinsic_any<'resolver, 'info: 'resolver, Resolver>(
        cursor: &mut &[u8], 
        metadata: &'info frame_metadata::RuntimeMetadata, 
        historic_types: &'resolver Resolver
    ) -> Result<Extrinsic<'resolver, AnyTypeId>, AnyError<ExtrinsicDecodeError>> 
    where
        Resolver: TypeResolver<TypeId = scale_info_legacy::LookupName>
    {
        use frame_metadata::RuntimeMetadata;
        match metadata {
            RuntimeMetadata::V8(m) => decode_extrinsic(cursor, m, historic_types)
                .map(|e| e.map_type_id(AnyTypeId::Legacy))
                .map_err(AnyError::DecodeError),
            RuntimeMetadata::V9(m) => decode_extrinsic(cursor, m, historic_types)
                .map(|e| e.map_type_id(AnyTypeId::Legacy))
                .map_err(AnyError::DecodeError),
            RuntimeMetadata::V10(m) => decode_extrinsic(cursor, m, historic_types)
                .map(|e| e.map_type_id(AnyTypeId::Legacy))
                .map_err(AnyError::DecodeError),
            RuntimeMetadata::V11(m) => decode_extrinsic(cursor, m, historic_types)
                .map(|e| e.map_type_id(AnyTypeId::Legacy))
                .map_err(AnyError::DecodeError),
            RuntimeMetadata::V12(m) => decode_extrinsic(cursor, m, historic_types)
                .map(|e| e.map_type_id(AnyTypeId::Legacy))
                .map_err(AnyError::DecodeError),
            RuntimeMetadata::V13(m) => decode_extrinsic(cursor, m, historic_types)
                .map(|e| e.map_type_id(AnyTypeId::Legacy))
                .map_err(AnyError::DecodeError),
            RuntimeMetadata::V14(m) => decode_extrinsic(cursor, m, &m.types)
                .map(|e| e.map_type_id(AnyTypeId::Current))
                .map_err(AnyError::DecodeError),
            RuntimeMetadata::V15(m) => decode_extrinsic(cursor, m, &m.types)
                .map(|e| e.map_type_id(AnyTypeId::Current))
                .map_err(AnyError::DecodeError),
            _ => Err(AnyError::MetadataNotSupported { metadata_version: metadata.version() })
        }
    }

    /// Decode an extrinsic in a modern runtime (ie one exposing V14+ metadata). Each part is denoted by
    /// a byte range and type ID, which can then be decoded into a value.
    pub fn decode_extrinsic_current<'info_and_resolver, T>(
        cursor: &mut &[u8], 
        metadata: &'info_and_resolver T, 
    ) -> Result<Extrinsic<'info_and_resolver, <T::Info as ExtrinsicTypeInfo>::TypeId>, ExtrinsicDecodeError> 
    where
        T: InfoAndResolver,
        T::Info: ExtrinsicTypeInfo,
        <T::Info as ExtrinsicTypeInfo>::TypeId: core::fmt::Debug + Clone,
        T::Resolver: scale_type_resolver::TypeResolver<TypeId = <T::Info as ExtrinsicTypeInfo>::TypeId>,
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
    use scale_decode::Visitor;
    use scale_type_resolver::TypeResolver;
    #[cfg(feature = "legacy")]
    use super::{AnyError,AnyTypeId};
    use crate::utils::InfoAndResolver;

    pub use crate::decoding::storage_type_info::{
        StorageTypeInfo,
        StorageInfo,
        StorageInfoError,
        StorageHasher,
        StorageKeyInfo,
    };
    pub use crate::decoding::storage_decoder::{
        StorageKey,
        StorageKeyPart,
        StorageKeyPartValue,
        StorageKeyDecodeError,
        StorageValueDecodeError,
        decode_storage_key,
        decode_storage_value,
    };

    /// Decode a storage key by breaking it down into its constituent parts. Each part is denoted by
    /// a hasher type, hasher byte range, and if possible, value information which can then be decoded 
    /// into a value.
    #[cfg(feature = "legacy")]
    pub fn decode_storage_key_any<'resolver, 'info: 'resolver, Resolver>(
        pallet_name: &str,
        entry_name: &str,
        cursor: &mut &[u8], 
        metadata: &'info frame_metadata::RuntimeMetadata,
        historic_types: &'resolver Resolver
    ) -> Result<StorageKey<AnyTypeId>, AnyError<StorageKeyDecodeError<AnyTypeId>>> 
    where
        Resolver: TypeResolver<TypeId = scale_info_legacy::LookupName>
    { 
        use frame_metadata::RuntimeMetadata;
        match metadata {
            RuntimeMetadata::V8(m) => decode_storage_key(pallet_name, entry_name, cursor, m, historic_types)
                .map(|e| e.map_type_id(AnyTypeId::Legacy))
                .map_err(|e| AnyError::DecodeError(e.map_type_id(AnyTypeId::Legacy))),
            RuntimeMetadata::V9(m) => decode_storage_key(pallet_name, entry_name, cursor, m, historic_types)
                .map(|e| e.map_type_id(AnyTypeId::Legacy))
                .map_err(|e| AnyError::DecodeError(e.map_type_id(AnyTypeId::Legacy))),
            RuntimeMetadata::V10(m) => decode_storage_key(pallet_name, entry_name, cursor, m, historic_types)
                .map(|e| e.map_type_id(AnyTypeId::Legacy))
                .map_err(|e| AnyError::DecodeError(e.map_type_id(AnyTypeId::Legacy))),
            RuntimeMetadata::V11(m) => decode_storage_key(pallet_name, entry_name, cursor, m, historic_types)
                .map(|e| e.map_type_id(AnyTypeId::Legacy))
                .map_err(|e| AnyError::DecodeError(e.map_type_id(AnyTypeId::Legacy))),
            RuntimeMetadata::V12(m) => decode_storage_key(pallet_name, entry_name, cursor, m, historic_types)
                .map(|e| e.map_type_id(AnyTypeId::Legacy))
                .map_err(|e| AnyError::DecodeError(e.map_type_id(AnyTypeId::Legacy))),
            RuntimeMetadata::V13(m) => decode_storage_key(pallet_name, entry_name, cursor, m, historic_types)
                .map(|e| e.map_type_id(AnyTypeId::Legacy))
                .map_err(|e| AnyError::DecodeError(e.map_type_id(AnyTypeId::Legacy))),
            RuntimeMetadata::V14(m) => decode_storage_key(pallet_name, entry_name, cursor, m, &m.types)
                .map(|e| e.map_type_id(AnyTypeId::Current))
                .map_err(|e| AnyError::DecodeError(e.map_type_id(AnyTypeId::Current))),
            RuntimeMetadata::V15(m) => decode_storage_key(pallet_name, entry_name, cursor, m, &m.types)
                .map(|e| e.map_type_id(AnyTypeId::Current))
                .map_err(|e| AnyError::DecodeError(e.map_type_id(AnyTypeId::Current))),
            _ => Err(AnyError::MetadataNotSupported { metadata_version: metadata.version() })
        }
    }

    /// Decode a storage key in a modern runtime (ie one exposing V14+ metadata) by breaking it down into its 
    /// constituent parts. Each part is denoted by a hasher type, hasher byte range, and if possible, value 
    /// information which can then be decoded into a value.
    pub fn decode_storage_key_current<T>(
        pallet_name: &str,
        storage_entry: &str,
        cursor: &mut &[u8], 
        metadata: &T, 
    ) -> Result<StorageKey<<T::Info as StorageTypeInfo>::TypeId>, StorageKeyDecodeError<<T::Info as StorageTypeInfo>::TypeId>> 
    where
        T: InfoAndResolver,
        T::Info: StorageTypeInfo,
        <T::Info as StorageTypeInfo>::TypeId: core::fmt::Debug + Clone,
        T::Resolver: TypeResolver<TypeId = <T::Info as StorageTypeInfo>::TypeId>,
    {
        decode_storage_key(pallet_name, storage_entry, cursor, metadata.info(), metadata.resolver())
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
        type_resolver: &Resolver
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
        visitor: V
    ) -> Result<V::Value<'scale, 'resolver>, StorageValueDecodeError<<T::Info as StorageTypeInfo>::TypeId>> 
    where
        T: InfoAndResolver,
        T::Info: StorageTypeInfo,
        <T::Info as StorageTypeInfo>::TypeId: core::fmt::Debug + Clone,
        T::Resolver: scale_type_resolver::TypeResolver<TypeId = <T::Info as StorageTypeInfo>::TypeId>,
        V: Visitor<TypeResolver = T::Resolver>,
        V::Error: core::fmt::Debug,
    {
        decode_storage_value(pallet_name, storage_entry, cursor, metadata.info(), metadata.resolver(), visitor)
    }

    /// Decode a storage value in a historic runtime (ie one prior to V14 metadata).
    #[cfg(feature = "legacy")]
    pub fn decode_storage_value_legacy<'scale, 'resolver, Info, Resolver, V>(
        pallet_name: &str, 
        storage_entry: &str, 
        cursor: &mut &'scale [u8], 
        info: &Info, 
        type_resolver: &'resolver Resolver, 
        visitor: V
    ) -> Result<V::Value<'scale, 'resolver>, StorageValueDecodeError<Info::TypeId>>
    where
        Info: StorageTypeInfo,
        Info::TypeId: Clone + core::fmt::Debug,
        Resolver: TypeResolver<TypeId = Info::TypeId>,
        V: scale_decode::Visitor<TypeResolver = Resolver>,
        V::Error: core::fmt::Debug
    {
        decode_storage_value(pallet_name, storage_entry, cursor, info, type_resolver, visitor)
    }
}

/// This is the type ID given back from calls like [`crate::extrinsics::decode_extrinsic_any`] and
/// [`crate::storage::decode_storage_key_any`], and represents either a modern or historic type ID,
/// depending on the age of the block that we are decoding from.
#[derive(Debug, Clone)]
pub enum AnyTypeId {
    Current(u32),
    #[cfg(feature = "legacy")]
    Legacy(scale_info_legacy::LookupName),
}

/// This is the error type given back from calls like [`crate::extrinsics::decode_extrinsic_any`] and
/// [`crate::storage::decode_storage_key_any`]. In the case that we pass a metadata version that the calls
/// don't currently support, this will return [`AnyError::MetadataNotSupported`].
#[derive(Debug, Clone)]
pub enum AnyError<Err> {
    MetadataNotSupported { metadata_version: u32 },
    DecodeError(Err)
}