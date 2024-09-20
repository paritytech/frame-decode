//! Decode extrinsics and storage values from substrate based networks like Polkadot.
#![warn(missing_docs)]

extern crate alloc;

mod utils;
mod decoding;

// TODO:
// - map_type_id on storage errors, too.
// - Feature flags and test in subxt?

/// This module contains functions for decoding extrinsics.
pub mod extrinsics {
    use scale_info_legacy::{LookupName, TypeRegistrySet};
    use frame_metadata::RuntimeMetadata;
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

    pub fn decode_extrinsic_any<'resolver, 'info: 'resolver>(
        cursor: &mut &[u8], 
        metadata: &'info frame_metadata::RuntimeMetadata, 
        historic_types: &'resolver TypeRegistrySet
    ) -> Result<Extrinsic<'resolver, AnyTypeId>, AnyError<ExtrinsicDecodeError>> {
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

    pub fn decode_extrinsic_legacy<'info, 'resolver, Info: ExtrinsicTypeInfo<TypeId = LookupName>>(
        cursor: &mut &[u8], 
        metadata: &'info Info, 
        historic_types: &'resolver TypeRegistrySet
    ) -> Result<Extrinsic<'info, LookupName>, ExtrinsicDecodeError> {
        decode_extrinsic(cursor, metadata, historic_types)
    }

}

/// This module contains functions for decoding storage keys and values.
pub mod storage {
    use scale_info_legacy::{LookupName, TypeRegistrySet};
    use frame_metadata::RuntimeMetadata;
    use super::{AnyError,AnyTypeId};

    pub use crate::decoding::storage_decoder::{
        StorageKey,
        StorageKeyPart,
        StorageKeyPartValue,
        StorageKeyDecodeError,
        StorageValue,
        StorageValueDecodeError,
        decode_storage_key,
        decode_storage_value,
    };

    // pub fn decode_storage_key_any<'resolver, 'info: 'resolver>(
    //     pallet_name: &str,
    //     entry_name: &str,
    //     cursor: &mut &[u8], 
    //     metadata: &'info frame_metadata::RuntimeMetadata, 
    //     historic_types: &'resolver TypeRegistrySet
    // ) -> Result<StorageKey<AnyTypeId>, AnyError<StorageKeyDecodeError<AnyTypeId>>> {
    //     match metadata {
    //         RuntimeMetadata::V8(m) => decode_storage_key(pallet_name, entry_name, cursor, m, historic_types)
    //             .map(|e| e.map_type_id(AnyTypeId::Legacy))
    //             .map_err(AnyError::DecodeError),
    //         RuntimeMetadata::V9(m) => decode_storage_key(pallet_name, entry_name, cursor, m, historic_types)
    //             .map(|e| e.map_type_id(AnyTypeId::Legacy))
    //             .map_err(AnyError::DecodeError),
    //         RuntimeMetadata::V10(m) => decode_storage_key(pallet_name, entry_name, cursor, m, historic_types)
    //             .map(|e| e.map_type_id(AnyTypeId::Legacy))
    //             .map_err(AnyError::DecodeError),
    //         RuntimeMetadata::V11(m) => decode_storage_key(pallet_name, entry_name, cursor, m, historic_types)
    //             .map(|e| e.map_type_id(AnyTypeId::Legacy))
    //             .map_err(AnyError::DecodeError),
    //         RuntimeMetadata::V12(m) => decode_storage_key(pallet_name, entry_name, cursor, m, historic_types)
    //             .map(|e| e.map_type_id(AnyTypeId::Legacy))
    //             .map_err(AnyError::DecodeError),
    //         RuntimeMetadata::V13(m) => decode_storage_key(pallet_name, entry_name, cursor, m, historic_types)
    //             .map(|e| e.map_type_id(AnyTypeId::Legacy))
    //             .map_err(AnyError::DecodeError),
    //         RuntimeMetadata::V14(m) => decode_storage_key(pallet_name, entry_name, cursor, m, &m.types)
    //             .map(|e| e.map_type_id(AnyTypeId::Current))
    //             .map_err(AnyError::DecodeError),
    //         RuntimeMetadata::V15(m) => decode_storage_key(pallet_name, entry_name, cursor, m, &m.types)
    //             .map(|e| e.map_type_id(AnyTypeId::Current))
    //             .map_err(AnyError::DecodeError),
    //         _ => Err(AnyError::MetadataNotSupported { metadata_version: metadata.version() })
    //     }
    // }
}

#[derive(Debug, Clone)]
pub enum AnyTypeId {
    Legacy(scale_info_legacy::LookupName),
    Current(u32)
}

#[derive(Debug, Clone)]
pub enum AnyError<Err> {
    MetadataNotSupported { metadata_version: u32 },
    DecodeError(Err)
}