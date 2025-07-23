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

use super::storage_type_info::{StorageHasher, StorageTypeInfo};
use crate::methods::storage_type_info::StorageInfoError;
use alloc::vec::Vec;
use scale_type_resolver::TypeResolver;

/// An error returned trying to encode storage keys.
#[non_exhaustive]
#[allow(missing_docs)]
#[derive(Debug, thiserror::Error)]
pub enum StorageKeyEncodeError {
    #[error("Cannot get storage info: {0}")]
    CannotGetInfo(StorageInfoError<'static>),
    #[error("Failed to encode storage key: {0}")]
    EncodeError(#[from] scale_encode::Error),
    #[error("Too many keys provided: expected at most {max_keys_expected}")]
    TooManyKeysProvided {
        /// The maximum number of keys that were expected.
        max_keys_expected: usize,
    },
}

/// Encode a storage key prefix from a pallet name and storage entry name. This prefix
/// is the first 32 bytes of any storage key which comes from a pallet, and is essentially
/// `twox_128(pallet_name) + twox_128(storage_entry_name)`.
pub fn encode_prefix(pallet_name: &str, storage_entry: &str) -> [u8; 32] {
    let mut prefix = [0u8; 32];

    let pallet_bytes = sp_crypto_hashing::twox_128(pallet_name.as_bytes());
    let entry_bytes = sp_crypto_hashing::twox_128(storage_entry.as_bytes());

    prefix[..16].copy_from_slice(&pallet_bytes);
    prefix[16..].copy_from_slice(&entry_bytes);

    prefix
}

/// Encode a complete storage key for a given pallet and storage entry and a set of keys that
/// are each able to be encoded via [`scale_encode::EncodeAsType`].
///
/// This is the same as [`encode_storage_key_to`], but returns the encoded key as a `Vec<u8>`, rather
/// than accepting a mutable output buffer.
///
/// # Example
///
/// ```rust
/// use frame_decode::storage::encode_storage_key;
/// use frame_metadata::RuntimeMetadata;
/// use parity_scale_codec::Decode;
///
/// let metadata_bytes = std::fs::read("artifacts/metadata_10000000_9180.scale").unwrap();
/// let RuntimeMetadata::V14(metadata) = RuntimeMetadata::decode(&mut &*metadata_bytes).unwrap() else { return };
///
/// let account_id = [0u8; 32];
///
/// // System.Account needs only one key to point at a specific value; the account ID.
/// // We just fake an account ID for this example  by providing 32 0 bytes, but anything
/// // which would `scale_encode::EncodeAsType` into 32 bytes would work.
/// let encoded_key = encode_storage_key(
///     "System",
///     "Account",
///     [account_id],
///     &metadata,
///     &metadata.types,
/// ).unwrap();
/// ```
pub fn encode_storage_key<Info, Resolver, Keys>(
    pallet_name: &str,
    storage_entry: &str,
    keys: Keys,
    info: &Info,
    type_resolver: &Resolver,
) -> Result<Vec<u8>, StorageKeyEncodeError>
where
    Keys: IntoStorageKeys,
    Info: StorageTypeInfo,
    Info::TypeId: Clone + core::fmt::Debug,
    Resolver: TypeResolver<TypeId = Info::TypeId>,
{
    // pre-allocate at least as many bytes as we need for the root/prefix.
    let mut out = Vec::with_capacity(32);
    encode_storage_key_to(
        pallet_name,
        storage_entry,
        keys,
        info,
        type_resolver,
        &mut out,
    )?;
    Ok(out)
}

/// Encode a complete storage key for a given pallet and storage entry and a set of keys that
/// are each able to be encoded via [`scale_encode::EncodeAsType`].
///
/// # Example
///
/// ```rust
/// use frame_decode::storage::encode_storage_key_to;
/// use frame_metadata::RuntimeMetadata;
/// use parity_scale_codec::Decode;
///
/// let metadata_bytes = std::fs::read("artifacts/metadata_10000000_9180.scale").unwrap();
/// let RuntimeMetadata::V14(metadata) = RuntimeMetadata::decode(&mut &*metadata_bytes).unwrap() else { return };
///
/// let account_id = [0u8; 32];
///
/// // We'll encode the key to this.
/// let mut encoded_key = Vec::new();
///
/// // System.Account needs only one key to point at a specific value; the account ID.
/// // We just fake an account ID for this example  by providing 32 0 bytes, but anything
/// // which would `scale_encode::EncodeAsType` into 32 bytes would work.
/// encode_storage_key_to(
///     "System",
///     "Account",
///     [account_id],
///     &metadata,
///     &metadata.types,
///     &mut encoded_key,
/// ).unwrap();
/// ```
pub fn encode_storage_key_to<Info, Resolver, Keys>(
    pallet_name: &str,
    storage_entry: &str,
    keys: Keys,
    info: &Info,
    type_resolver: &Resolver,
    out: &mut Vec<u8>,
) -> Result<(), StorageKeyEncodeError>
where
    Keys: IntoStorageKeys,
    Info: StorageTypeInfo,
    Info::TypeId: Clone + core::fmt::Debug,
    Resolver: TypeResolver<TypeId = Info::TypeId>,
{
    let storage_info = info
        .get_storage_info(pallet_name, storage_entry)
        .map_err(|e| StorageKeyEncodeError::CannotGetInfo(e.into_owned()))?;

    encode_storage_key_with_info_to(
        &pallet_name,
        &storage_entry,
        keys,
        &storage_info,
        type_resolver,
        out,
    )
}

/// Encode a complete storage key for a given pallet and storage entry and a set of keys that
/// are each able to be encoded via [`scale_encode::EncodeAsType`].
///
/// Unlike [`encode_storage_key`], which obtains the storage info internally given the pallet and storage entry names,
/// this function takes the storage info as an argument. This is useful if you already have the storage info available,
/// for example if you are encoding multiple keys for the same storage entry.
pub fn encode_storage_key_with_info_to<Resolver, Keys>(
    pallet_name: &str,
    storage_entry: &str,
    keys: Keys,
    storage_info: &crate::storage::StorageInfo<<Resolver as TypeResolver>::TypeId>,
    type_resolver: &Resolver,
    out: &mut Vec<u8>,
) -> Result<(), StorageKeyEncodeError>
where
    Keys: IntoStorageKeys,
    Resolver: TypeResolver,
    <Resolver as TypeResolver>::TypeId: Clone + core::fmt::Debug,
{
    // Encode the prefix:
    let prefix = encode_prefix(pallet_name, storage_entry);
    out.extend_from_slice(&prefix);

    // Encode the keys:
    let mut keys = keys.into_storage_keys();
    let mut temp = Vec::with_capacity(32);
    for key_info in &storage_info.keys {
        match keys.encode_next_key_to(key_info.key_id.clone(), type_resolver, &mut temp) {
            None => break, // No more keys to encode.
            Some(Err(e)) => return Err(StorageKeyEncodeError::EncodeError(e)),
            Some(Ok(())) => { /* All ok */ }
        };

        match key_info.hasher {
            StorageHasher::Blake2_128 => {
                let hash = sp_crypto_hashing::blake2_128(&temp);
                out.extend_from_slice(&hash);
            }
            StorageHasher::Blake2_256 => {
                let hash = sp_crypto_hashing::blake2_256(&temp);
                out.extend_from_slice(&hash);
            }
            StorageHasher::Blake2_128Concat => {
                let hash = sp_crypto_hashing::blake2_128(&temp);
                out.extend_from_slice(&hash);
                out.extend_from_slice(&temp);
            }
            StorageHasher::Twox128 => {
                let hash = sp_crypto_hashing::twox_128(&temp);
                out.extend_from_slice(&hash);
            }
            StorageHasher::Twox256 => {
                let hash = sp_crypto_hashing::twox_256(&temp);
                out.extend_from_slice(&hash);
            }
            StorageHasher::Twox64Concat => {
                let hash = sp_crypto_hashing::twox_64(&temp);
                out.extend_from_slice(&hash);
                out.extend_from_slice(&temp);
            }
            StorageHasher::Identity => {
                out.extend_from_slice(&temp);
            }
        }

        // Clear our temp space ready for the next key.
        temp.clear();
    }

    // If the user has provided more keys, ie we still have keys that
    // we're supposed to encode at this point, then return an error.
    if keys.has_next_key() {
        return Err(StorageKeyEncodeError::TooManyKeysProvided {
            max_keys_expected: storage_info.keys.len(),
        });
    }

    Ok(())
}

/// This can be implemented for anything that can be converted into something implementing [`StorageKeys`].
/// It is implemented by default for tuples up to length 10, vectors and arrays (where the values all implement
/// [`scale_encode::EncodeAsType`]).
pub trait IntoStorageKeys {
    /// An implementation of [`StorageKeys`] that can be used to iterate through the keys.
    type Keys: StorageKeys;
    /// Return an implementation of [`StorageKeys`] for this type.
    fn into_storage_keys(self) -> Self::Keys;
}

/// Since [`scale_encode::EncodeAsType`] is not dyn safe, this trait is used to iterate through and encode a set of keys.
pub trait StorageKeys {
    /// Is there another key still to encode?
    fn has_next_key(&self) -> bool;
    /// Encode the next key, if there is one, into the provided output buffer.
    fn encode_next_key_to<Resolver>(
        &mut self,
        type_id: Resolver::TypeId,
        types: &Resolver,
        out: &mut Vec<u8>,
    ) -> Option<Result<(), scale_encode::Error>>
    where
        Resolver: TypeResolver;
    /// Encode the next key, if there is one, and return the encoded bytes
    fn encode_next_key<Resolver>(
        &mut self,
        type_id: Resolver::TypeId,
        types: &Resolver,
    ) -> Option<Result<Vec<u8>, scale_encode::Error>>
    where
        Resolver: TypeResolver,
    {
        let mut out = Vec::new();
        self.encode_next_key_to(type_id, types, &mut out)
            .map(|res| res.map(|_| out))
    }
}

// Vecs of keys implement IntoStorageKeys.
impl<K: scale_encode::EncodeAsType> IntoStorageKeys for Vec<K> {
    type Keys = <Self as IntoIterator>::IntoIter;
    fn into_storage_keys(self) -> Self::Keys {
        self.into_iter()
    }
}

impl<K: scale_encode::EncodeAsType> StorageKeys for alloc::vec::IntoIter<K> {
    fn has_next_key(&self) -> bool {
        self.len() > 0
    }
    fn encode_next_key_to<Resolver>(
        &mut self,
        type_id: Resolver::TypeId,
        types: &Resolver,
        out: &mut Vec<u8>,
    ) -> Option<Result<(), scale_encode::Error>>
    where
        Resolver: TypeResolver,
    {
        let next_key = self.next()?;
        if let Err(e) = next_key.encode_as_type_to(type_id, types, out) {
            return Some(Err(e));
        }
        Some(Ok(()))
    }
}

// As do arrays of keys.
impl<K: scale_encode::EncodeAsType, const N: usize> IntoStorageKeys for [K; N] {
    type Keys = <Self as IntoIterator>::IntoIter;
    fn into_storage_keys(self) -> Self::Keys {
        self.into_iter()
    }
}

impl<K: scale_encode::EncodeAsType, const N: usize> StorageKeys for core::array::IntoIter<K, N> {
    fn has_next_key(&self) -> bool {
        self.len() > 0
    }
    fn encode_next_key_to<Resolver>(
        &mut self,
        type_id: Resolver::TypeId,
        types: &Resolver,
        out: &mut Vec<u8>,
    ) -> Option<Result<(), scale_encode::Error>>
    where
        Resolver: TypeResolver,
    {
        let next_key = self.next()?;
        if let Err(e) = next_key.encode_as_type_to(type_id, types, out) {
            return Some(Err(e));
        }
        Some(Ok(()))
    }
}

// Empty tuples can be used as a placeholder for no storage keys.
impl IntoStorageKeys for () {
    type Keys = ();
    fn into_storage_keys(self) -> Self::Keys {
        ()
    }
}

impl StorageKeys for () {
    fn has_next_key(&self) -> bool {
        false
    }
    fn encode_next_key_to<Resolver>(
        &mut self,
        _type_id: Resolver::TypeId,
        _types: &Resolver,
        _out: &mut Vec<u8>,
    ) -> Option<Result<(), scale_encode::Error>>
    where
        Resolver: TypeResolver,
    {
        None
    }
}

// Tuples of different lengths can be used as storage keys, too.
macro_rules! impl_tuple_storage_keys {
    ($($ty:ident $number:tt),*) => {
        const _: () = {
            const TUPLE_LEN: usize = 0 $(+ $number - $number + 1)*;

            impl <$($ty: scale_encode::EncodeAsType),*> IntoStorageKeys for ($($ty,)*) {
                type Keys = TupleIter<$($ty),*>;
                fn into_storage_keys(self) -> Self::Keys {
                    TupleIter {
                        idx: 0,
                        items: self,
                    }
                }
            }

            pub struct TupleIter<$($ty),*> {
                idx: usize,
                items: ($($ty,)*)
            }

            impl <$($ty: scale_encode::EncodeAsType),*> StorageKeys for TupleIter<$($ty),*> {
                fn has_next_key(&self) -> bool {
                    self.idx < TUPLE_LEN
                }
                fn encode_next_key_to<Resolver>(&mut self, type_id: Resolver::TypeId, types: &Resolver, out: &mut Vec<u8>) -> Option<Result<(), scale_encode::Error>>
                where
                    Resolver: TypeResolver,
                {
                    $(
                        if self.idx == $number {
                            let item = &self.items.$number;
                            if let Err(e) = item.encode_as_type_to(type_id, types, out) {
                                return Some(Err(e));
                            }
                            self.idx += 1;
                            return Some(Ok(()));
                        }
                    )*
                    None
                }
            }
        };
    };
}

impl_tuple_storage_keys!(A 0);
impl_tuple_storage_keys!(A 0, B 1);
impl_tuple_storage_keys!(A 0, B 1, C 2);
impl_tuple_storage_keys!(A 0, B 1, C 2, D 3);
impl_tuple_storage_keys!(A 0, B 1, C 2, D 3, E 4);
impl_tuple_storage_keys!(A 0, B 1, C 2, D 3, E 4, F 5);
impl_tuple_storage_keys!(A 0, B 1, C 2, D 3, E 4, F 5, G 6);
impl_tuple_storage_keys!(A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7);
impl_tuple_storage_keys!(A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8);
impl_tuple_storage_keys!(A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8, J 9);

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_tuple_storage_keys() {
        use parity_scale_codec::Encode;
        use scale_info_legacy::LookupName;

        // We just need some basic types to test with.
        let types = crate::legacy_types::polkadot::relay_chain();
        let types = types.for_spec_version(0);

        let keys = (123u16, true, "hello");
        let mut storage_keys = keys.into_storage_keys();

        assert_eq!(storage_keys.has_next_key(), true);
        let val = storage_keys
            .encode_next_key(LookupName::parse("u64").unwrap(), &types)
            .unwrap()
            .unwrap();
        assert_eq!(val, 123u64.encode());

        assert_eq!(storage_keys.has_next_key(), true);
        let val = storage_keys
            .encode_next_key(LookupName::parse("bool").unwrap(), &types)
            .unwrap()
            .unwrap();
        assert_eq!(val, true.encode());

        assert_eq!(storage_keys.has_next_key(), true);
        let val = storage_keys
            .encode_next_key(LookupName::parse("String").unwrap(), &types)
            .unwrap()
            .unwrap();
        assert_eq!(val, "hello".encode());

        assert_eq!(storage_keys.has_next_key(), false);
        assert!(storage_keys
            .encode_next_key(LookupName::parse("foo").unwrap(), &types)
            .is_none());
        assert!(storage_keys
            .encode_next_key(LookupName::parse("foo").unwrap(), &types)
            .is_none());
    }

    #[test]
    fn test_vec_storage_keys() {
        use parity_scale_codec::Encode;
        use scale_info_legacy::LookupName;

        // We just need some basic types to test with.
        let types = crate::legacy_types::polkadot::relay_chain();
        let types = types.for_spec_version(0);

        let keys = vec![123u16, 456u16, 789u16];
        let mut storage_keys = keys.into_storage_keys();

        assert_eq!(storage_keys.has_next_key(), true);
        let val = storage_keys
            .encode_next_key(LookupName::parse("u64").unwrap(), &types)
            .unwrap()
            .unwrap();
        assert_eq!(val, 123u64.encode());

        assert_eq!(storage_keys.has_next_key(), true);
        let val = storage_keys
            .encode_next_key(LookupName::parse("u16").unwrap(), &types)
            .unwrap()
            .unwrap();
        assert_eq!(val, 456u16.encode());

        assert_eq!(storage_keys.has_next_key(), true);
        let val = storage_keys
            .encode_next_key(LookupName::parse("u32").unwrap(), &types)
            .unwrap()
            .unwrap();
        assert_eq!(val, 789u32.encode());

        assert_eq!(storage_keys.has_next_key(), false);
        assert!(storage_keys
            .encode_next_key(LookupName::parse("foo").unwrap(), &types)
            .is_none());
        assert!(storage_keys
            .encode_next_key(LookupName::parse("foo").unwrap(), &types)
            .is_none());
    }

    #[test]
    fn test_array_storage_keys() {
        use parity_scale_codec::Encode;
        use scale_info_legacy::LookupName;

        // We just need some basic types to test with.
        let types = crate::legacy_types::polkadot::relay_chain();
        let types = types.for_spec_version(0);

        let keys: [u16; 3] = [123, 456, 789];
        let mut storage_keys = keys.into_storage_keys();

        assert_eq!(storage_keys.has_next_key(), true);
        let val = storage_keys
            .encode_next_key(LookupName::parse("u64").unwrap(), &types)
            .unwrap()
            .unwrap();
        assert_eq!(val, 123u64.encode());

        assert_eq!(storage_keys.has_next_key(), true);
        let val = storage_keys
            .encode_next_key(LookupName::parse("u16").unwrap(), &types)
            .unwrap()
            .unwrap();
        assert_eq!(val, 456u16.encode());

        assert_eq!(storage_keys.has_next_key(), true);
        let val = storage_keys
            .encode_next_key(LookupName::parse("u32").unwrap(), &types)
            .unwrap()
            .unwrap();
        assert_eq!(val, 789u32.encode());

        assert_eq!(storage_keys.has_next_key(), false);
        assert!(storage_keys
            .encode_next_key(LookupName::parse("foo").unwrap(), &types)
            .is_none());
        assert!(storage_keys
            .encode_next_key(LookupName::parse("foo").unwrap(), &types)
            .is_none());
    }

    #[test]
    fn test_encode_storage_key() {
        use crate::storage::encode_storage_key;
        use frame_metadata::RuntimeMetadata;
        use parity_scale_codec::Decode;

        let metadata_bytes = std::fs::read("artifacts/metadata_10000000_9180.scale").unwrap();
        let RuntimeMetadata::V14(metadata) =
            RuntimeMetadata::decode(&mut &*metadata_bytes).unwrap()
        else {
            return;
        };

        let account_id = [0u8; 32];

        // System.Account needs only one key to point at a specific value; the account ID.
        // We just fake an account ID for this example by providing 32 0 bytes, but anything
        // which would `scale_encode::EncodeAsType` into 32 bytes should work.
        encode_storage_key(
            "System",
            "Account",
            [account_id],
            &metadata,
            &metadata.types,
        )
        .expect("Encoding should work");
        encode_storage_key(
            "System",
            "Account",
            (account_id,),
            &metadata,
            &metadata.types,
        )
        .expect("Encoding should work");

        // We provide no additional keys, so we should get the prefix only.
        let out = encode_storage_key("System", "Account", (), &metadata, &metadata.types)
            .expect("Encoding should work");
        assert_eq!(&out, &encode_prefix("System", "Account"));

        // We provide too many additional keys so should get an error.
        let err = encode_storage_key(
            "System",
            "Account",
            (account_id, 123u16),
            &metadata,
            &metadata.types,
        );
        assert!(matches!(
            err,
            Err(StorageKeyEncodeError::TooManyKeysProvided {
                max_keys_expected: 1
            })
        ));
    }
}
