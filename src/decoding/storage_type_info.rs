use alloc::vec::Vec;
use alloc::borrow::ToOwned;
use alloc::borrow::Cow;

/// This is implemented for all metadatas exposed from `frame_metadata` and is responsible for extracting the
/// type IDs and related info needed to decode storage entries.
pub trait StorageTypeInfo {
    /// The type of type IDs that we are using to obtain type information.
    type TypeId;
    /// Get the information needed to decode a specific storage entry key/value.
    fn get_storage_info(&self, pallet_name: &str, storage_entry: &str) -> Result<StorageInfo<Self::TypeId>, StorageInfoError<'_>>;
}

/// An error returned trying to access storage type information.
#[non_exhaustive]
#[allow(missing_docs)]
#[derive(Clone, Debug)]
pub enum StorageInfoError<'a> {
    PalletNotFound { name: Cow<'a, str> },
    StorageNotFound { name: Cow<'a, str>, pallet_name: Cow<'a, str> },
    #[cfg(feature = "legacy")]
    CannotParseTypeName { name: Cow<'a, str>, reason: scale_info_legacy::lookup_name::ParseError },
    HasherKeyMismatch { entry_name: Cow<'a, str>, pallet_name: Cow<'a, str>, num_hashers: usize, num_keys: usize },
    StorageTypeNotFound { entry_name: Cow<'a, str>, pallet_name: Cow<'a, str>, id: u32 },
}

impl <'a> core::error::Error for StorageInfoError<'a> {}

impl <'a> core::fmt::Display for StorageInfoError<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            StorageInfoError::PalletNotFound { name } => {
                write!(f, "Pallet '{name}' not found.")
            },
            StorageInfoError::StorageNotFound { name, pallet_name } => {
                write!(f, "Storage item '{name}' not found in pallet '{pallet_name}'.")
            },
            #[cfg(feature = "legacy")]
            StorageInfoError::CannotParseTypeName { name, reason } => {
                write!(f, "Cannot parse type name {name}:\n\n{reason}.")
            },
            StorageInfoError::HasherKeyMismatch { entry_name, pallet_name, num_hashers, num_keys } => {
                write!(f, "Number of hashers and keys does not line up for {pallet_name}.{entry_name}; we have {num_hashers} hashers and {num_keys} keys.")
            },
            StorageInfoError::StorageTypeNotFound { entry_name, pallet_name, id } => {
                write!(f, "Cannot find type ID {id} for {pallet_name}.{entry_name}.")
            },
        }
    }
}

impl <'a> StorageInfoError<'a> {
    /// Take ownership of this error, turning any lifetimes to `'static`.
    pub fn into_owned(self) -> StorageInfoError<'static> {
        match self {
            StorageInfoError::PalletNotFound { name } => {
                StorageInfoError::PalletNotFound { name: Cow::Owned(name.into_owned()) }
            }
            StorageInfoError::StorageNotFound { name, pallet_name } => {
                StorageInfoError::StorageNotFound { name: Cow::Owned(name.into_owned()), pallet_name: Cow::Owned(pallet_name.into_owned()) }
            }
            #[cfg(feature = "legacy")]
            StorageInfoError::CannotParseTypeName { name, reason } => {
                StorageInfoError::CannotParseTypeName { name: Cow::Owned(name.into_owned()), reason }
            }
            StorageInfoError::HasherKeyMismatch { entry_name, pallet_name, num_hashers, num_keys } => {
                StorageInfoError::HasherKeyMismatch { entry_name: Cow::Owned(entry_name.into_owned()), pallet_name: Cow::Owned(pallet_name.into_owned()), num_hashers, num_keys }
            }
            StorageInfoError::StorageTypeNotFound { entry_name, pallet_name, id } => {
                StorageInfoError::StorageTypeNotFound { entry_name: Cow::Owned(entry_name.into_owned()), pallet_name: Cow::Owned(pallet_name.into_owned()), id }
            }
        }
    }
}

/// Information about a storage entry.
#[derive(Debug)]
pub struct StorageInfo<TypeId> {
    /// No entries if a plain storage entry, or N entries for N maps.
    pub keys: Vec<StorageKeyInfo<TypeId>>,
    /// The type of the values.
    pub value_id: TypeId,
}

/// Information about a single key within a storage entry.
#[derive(Debug)]
pub struct StorageKeyInfo<TypeId> {
    /// How is this key hashed?
    pub hasher: StorageHasher,
    /// The type of the key.
    pub key_id: TypeId,
}

/// Hasher used by storage maps
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum StorageHasher {
	/// 128-bit Blake2 hash.
	Blake2_128,
	/// 256-bit Blake2 hash.
	Blake2_256,
    /// Multiple 128-bit Blake2 hashes concatenated.
	Blake2_128Concat,
	/// 128-bit XX hash.
	Twox128,
	/// 256-bit XX hash.
	Twox256,
	/// 64-bit XX hashes concatentation.
	Twox64Concat,
	/// Identity hashing (no hashing).
	Identity,
}

macro_rules! impl_storage_type_info_for_v14_to_v15 {
    ($path:path, $name:ident, $to_storage_hasher:ident) => {
        const _: () = {
            use $path as path;
            impl StorageTypeInfo for path :: $name {
                type TypeId = u32;
                fn get_storage_info(&self, pallet_name: &str, storage_entry: &str) -> Result<StorageInfo<Self::TypeId>, StorageInfoError<'_>> {
                    let pallet = self.pallets
                        .iter()
                        .find(|p| p.name.as_ref() as &str == pallet_name)
                        .ok_or_else(|| StorageInfoError::PalletNotFound { name: Cow::Owned(pallet_name.to_owned()) })?;
            
                    let storages = pallet.storage
                        .as_ref()
                        .ok_or_else(|| StorageInfoError::StorageNotFound {
                            name: Cow::Owned(storage_entry.to_owned()),
                            pallet_name: Cow::Borrowed(&pallet.name)
                        })?;

                    let storage = storages.entries
                        .iter()
                        .find(|e| e.name.as_ref() as &str == storage_entry)
                        .ok_or_else(|| StorageInfoError::StorageNotFound {
                            name: Cow::Owned(storage_entry.to_owned()),
                            pallet_name: Cow::Borrowed(&pallet.name)
                        })?;
                                    
                    match &storage.ty {
                        path::StorageEntryType::Plain(value) => {
                            Ok(StorageInfo { 
                                keys: Vec::new(), 
                                value_id: value.id 
                            })
                        },
                        path::StorageEntryType::Map { hashers, key, value } => {
                            let value_id = value.id;
                            let key_id = key.id;
                            let key_ty = self.types
                                .resolve(key_id)
                                .ok_or_else(|| StorageInfoError::StorageTypeNotFound {
                                    pallet_name: Cow::Borrowed(&pallet.name),
                                    entry_name: Cow::Borrowed(&storage.name),
                                    id: key_id,
                                })?;
            
                            if let scale_info::TypeDef::Tuple(tuple) = &key_ty.type_def {
                                if hashers.len() == 1 {
                                    // Multiple keys but one hasher; use same hasher for every key
                                    let hasher = $to_storage_hasher(&hashers[0]);
                                    Ok(StorageInfo { 
                                        keys: tuple.fields.iter().map(|f| StorageKeyInfo { hasher, key_id: f.id }).collect(), 
                                        value_id
                                    })
                                } else if hashers.len() == tuple.fields.len() {
                                    // One hasher per key
                                    let keys = tuple.fields.iter().zip(hashers).map(|(field, hasher)| {
                                        StorageKeyInfo {
                                            hasher: $to_storage_hasher(hasher),
                                            key_id: field.id
                                        }
                                    }).collect();
                                    Ok(StorageInfo { 
                                        keys, 
                                        value_id
                                    })
                                } else {
                                    // Hasher and key mismatch
                                    Err(StorageInfoError::HasherKeyMismatch { 
                                        pallet_name: Cow::Borrowed(&pallet.name),
                                        entry_name: Cow::Borrowed(&storage.name),
                                        num_hashers: hashers.len(), 
                                        num_keys: tuple.fields.len() 
                                    })
                                }
                            } else if hashers.len() == 1 {
                                // One key, one hasher.
                                Ok(StorageInfo { 
                                    keys: Vec::from_iter([
                                        StorageKeyInfo { hasher: $to_storage_hasher(&hashers[0]), key_id }
                                    ]), 
                                    value_id
                                })
                            } else {
                                // Multiple hashers but only one key; error.
                                Err(StorageInfoError::HasherKeyMismatch { 
                                    pallet_name: Cow::Borrowed(&pallet.name),
                                    entry_name: Cow::Borrowed(&storage.name),
                                    num_hashers: hashers.len(), 
                                    num_keys: 1
                                })                            
                            }
                        },
                    }
                }
            }
        };
    }
}

impl_storage_type_info_for_v14_to_v15!(frame_metadata::v14, RuntimeMetadataV14, to_storage_hasher_v14);
impl_storage_type_info_for_v14_to_v15!(frame_metadata::v15, RuntimeMetadataV15, to_storage_hasher_v15);

macro_rules! to_latest_storage_hasher {
    ($ident:ident, $path:path) => {
        fn $ident(hasher: &$path) -> StorageHasher {
            match hasher {
                <$path>::Blake2_128 => StorageHasher::Blake2_128,
                <$path>::Blake2_128Concat => StorageHasher::Blake2_128Concat,
                <$path>::Blake2_256 => StorageHasher::Blake2_256,
                <$path>::Twox128 => StorageHasher::Twox128,
                <$path>::Twox256 => StorageHasher::Twox256,
                <$path>::Twox64Concat => StorageHasher::Twox64Concat,
                <$path>::Identity => StorageHasher::Identity,
            }
        }
    }
}

to_latest_storage_hasher!(to_storage_hasher_v14, frame_metadata::v14::StorageHasher);
to_latest_storage_hasher!(to_storage_hasher_v15, frame_metadata::v15::StorageHasher);

#[cfg(feature = "legacy")]
mod legacy {
    use super::*;
    use scale_info_legacy::LookupName;
    use frame_metadata::decode_different::DecodeDifferent;
    use crate::utils::as_decoded;

    macro_rules! impl_storage_type_info_for_v8_to_v12 {
        ($path:path, $name:ident, $to_storage_hasher:ident) => {
            const _: () = {
                use $path as path;
                impl StorageTypeInfo for path :: $name {
                    type TypeId = LookupName;
                
                    fn get_storage_info(&self, pallet_name: &str, storage_entry: &str) -> Result<StorageInfo<Self::TypeId>, StorageInfoError<'_>> {
                        let modules = as_decoded(&self.modules);
                
                        let m = modules
                            .iter()
                            .find(|m| as_decoded(&m.name).as_ref() as &str == pallet_name)
                            .ok_or_else(|| StorageInfoError::PalletNotFound { name: Cow::Owned(pallet_name.to_owned()) })?;
                
                        let pallet_name = as_decoded(&m.name);
    
                        let storages = m.storage
                            .as_ref()
                            .map(|s| as_decoded(s))
                            .ok_or_else(|| StorageInfoError::StorageNotFound {
                                name: Cow::Owned(storage_entry.to_owned()),
                                pallet_name: Cow::Borrowed(pallet_name)
                            })?;
                
                        let storage = as_decoded(&storages.entries)
                            .iter()
                            .find(|s| as_decoded(&s.name).as_ref() as &str == storage_entry)
                            .ok_or_else(|| StorageInfoError::StorageNotFound {
                                name: Cow::Owned(storage_entry.to_owned()),
                                pallet_name: Cow::Borrowed(pallet_name)
                            })?;
    
                        match &storage.ty {
                            path::StorageEntryType::Plain(ty) => {
                                let value_id = decode_lookup_name_or_err(ty, pallet_name)?;
                                Ok(StorageInfo {
                                    keys: vec![],
                                    value_id
                                })
                            },
                            path::StorageEntryType::Map { hasher, key, value, .. } => {
                                let key_id = decode_lookup_name_or_err(key, pallet_name)?;
                                let hasher = $to_storage_hasher(hasher);
                                let value_id = decode_lookup_name_or_err(value, pallet_name)?;
                                Ok(StorageInfo { 
                                    keys: vec![StorageKeyInfo { hasher, key_id }],
                                    value_id
                                })
                            },
                            path::StorageEntryType::DoubleMap { hasher, key1, key2, value, key2_hasher } => {
                                let key1_id = decode_lookup_name_or_err(key1, pallet_name)?;
                                let key1_hasher = $to_storage_hasher(hasher);
                                let key2_id = decode_lookup_name_or_err(key2, pallet_name)?;
                                let key2_hasher = $to_storage_hasher(key2_hasher);
                                let value_id = decode_lookup_name_or_err(value, pallet_name)?;
                                Ok(StorageInfo { 
                                    keys: vec![
                                        StorageKeyInfo { hasher: key1_hasher, key_id: key1_id },
                                        StorageKeyInfo { hasher: key2_hasher, key_id: key2_id },
                                    ],
                                    value_id
                                })
                            }
                        }
                    }
                }
            };
        }
    }
    
    impl_storage_type_info_for_v8_to_v12!(frame_metadata::v8, RuntimeMetadataV8, to_storage_hasher_v8);
    impl_storage_type_info_for_v8_to_v12!(frame_metadata::v9, RuntimeMetadataV9, to_storage_hasher_v9);
    impl_storage_type_info_for_v8_to_v12!(frame_metadata::v10, RuntimeMetadataV10, to_storage_hasher_v10);
    impl_storage_type_info_for_v8_to_v12!(frame_metadata::v11, RuntimeMetadataV11, to_storage_hasher_v11);
    impl_storage_type_info_for_v8_to_v12!(frame_metadata::v12, RuntimeMetadataV12, to_storage_hasher_v12);
    
    impl StorageTypeInfo for frame_metadata::v13::RuntimeMetadataV13 {
        type TypeId = LookupName;
    
        fn get_storage_info(&self, pallet_name: &str, storage_entry: &str) -> Result<StorageInfo<Self::TypeId>, StorageInfoError<'_>> {
            let modules = as_decoded(&self.modules);
                
            let m = modules
                .iter()
                .find(|m| as_decoded(&m.name).as_ref() as &str == pallet_name)
                .ok_or_else(|| StorageInfoError::PalletNotFound { name: Cow::Owned(pallet_name.to_owned()) })?;
    
            let pallet_name = as_decoded(&m.name);
    
            let storages = m.storage
                .as_ref()
                .map(|s| as_decoded(s))
                .ok_or_else(|| StorageInfoError::StorageNotFound {
                    name: Cow::Owned(storage_entry.to_owned()),
                    pallet_name: Cow::Borrowed(pallet_name)
                })?;
    
            let storage = as_decoded(&storages.entries)
                .iter()
                .find(|s| as_decoded(&s.name).as_ref() as &str == storage_entry)
                .ok_or_else(|| StorageInfoError::StorageNotFound {
                    name: Cow::Owned(storage_entry.to_owned()),
                    pallet_name: Cow::Borrowed(pallet_name)
                })?;
    
            let storage_name = as_decoded(&storage.name);
    
            match &storage.ty {
                frame_metadata::v13::StorageEntryType::Plain(ty) => {
                    let value_id = decode_lookup_name_or_err(ty, pallet_name)?;
                    Ok(StorageInfo {
                        keys: vec![],
                        value_id
                    })
                },
                frame_metadata::v13::StorageEntryType::Map { hasher, key, value, .. } => {
                    let key_id = decode_lookup_name_or_err(key, pallet_name)?;
                    let hasher = to_storage_hasher_v13(hasher);
                    let value_id = decode_lookup_name_or_err(value, pallet_name)?;
                    Ok(StorageInfo { 
                        keys: vec![StorageKeyInfo { hasher, key_id }],
                        value_id
                    })
                },
                frame_metadata::v13::StorageEntryType::DoubleMap { hasher, key1, key2, value, key2_hasher } => {
                    let key1_id = decode_lookup_name_or_err(key1, pallet_name)?;
                    let key1_hasher = to_storage_hasher_v13(hasher);
                    let key2_id = decode_lookup_name_or_err(key2, pallet_name)?;
                    let key2_hasher = to_storage_hasher_v13(key2_hasher);
                    let value_id = decode_lookup_name_or_err(value, pallet_name)?;
                    Ok(StorageInfo { 
                        keys: vec![
                            StorageKeyInfo { hasher: key1_hasher, key_id: key1_id },
                            StorageKeyInfo { hasher: key2_hasher, key_id: key2_id },
                        ],
                        value_id
                    })
                },
                frame_metadata::v13::StorageEntryType::NMap { keys, hashers, value } => {
                    let keys = as_decoded(keys);
                    let hashers = as_decoded(hashers);
                    let value_id = decode_lookup_name_or_err(value, pallet_name)?;
    
                    // If one hasher and lots of keys then hash each key the same.
                    // If one hasher per key then unique hasher per key.
                    // Else, there's some error.
                    let keys: Result<Vec<_>, StorageInfoError<'_>> = if hashers.len() == 1 {
                        let hasher = to_storage_hasher_v13(&hashers[0]);
                        keys.iter()
                            .map(|key| {
                                let key_id = lookup_name_or_err(key, pallet_name)?;
                                Ok(StorageKeyInfo { hasher, key_id })
                            })
                            .collect()
                    } else if hashers.len() == keys.len() {
                        keys.iter()
                            .zip(hashers)
                            .map(|(key, hasher)| {
                                let hasher = to_storage_hasher_v13(hasher);
                                let key_id = lookup_name_or_err(key, pallet_name)?;
                                Ok(StorageKeyInfo { hasher, key_id })
                            })
                            .collect()
                    } else {
                        Err(StorageInfoError::HasherKeyMismatch { 
                            pallet_name: Cow::Borrowed(pallet_name),
                            entry_name: Cow::Borrowed(storage_name),
                            num_hashers: hashers.len(), 
                            num_keys: keys.len() 
                        })
                    };
    
                    Ok(StorageInfo { 
                        keys: keys?,
                        value_id
                    })
                }
            }
        }
    }

    fn to_storage_hasher_v8(hasher: &frame_metadata::v8::StorageHasher) -> StorageHasher {
        match hasher {
            frame_metadata::v8::StorageHasher::Blake2_128 => StorageHasher::Blake2_128,
            frame_metadata::v8::StorageHasher::Blake2_256 => StorageHasher::Blake2_256,
            frame_metadata::v8::StorageHasher::Twox128 => StorageHasher::Twox128,
            frame_metadata::v8::StorageHasher::Twox256 => StorageHasher::Twox256,
            frame_metadata::v8::StorageHasher::Twox64Concat => StorageHasher::Twox64Concat,
        }
    }
    fn to_storage_hasher_v9(hasher: &frame_metadata::v9::StorageHasher) -> StorageHasher {
        match hasher {
            frame_metadata::v9::StorageHasher::Blake2_128 => StorageHasher::Blake2_128,
            frame_metadata::v9::StorageHasher::Blake2_128Concat => StorageHasher::Blake2_128Concat,
            frame_metadata::v9::StorageHasher::Blake2_256 => StorageHasher::Blake2_256,
            frame_metadata::v9::StorageHasher::Twox128 => StorageHasher::Twox128,
            frame_metadata::v9::StorageHasher::Twox256 => StorageHasher::Twox256,
            frame_metadata::v9::StorageHasher::Twox64Concat => StorageHasher::Twox64Concat,
        }
    }
    fn to_storage_hasher_v10(hasher: &frame_metadata::v10::StorageHasher) -> StorageHasher {
        match hasher {
            frame_metadata::v10::StorageHasher::Blake2_128 => StorageHasher::Blake2_128,
            frame_metadata::v10::StorageHasher::Blake2_128Concat => StorageHasher::Blake2_128Concat,
            frame_metadata::v10::StorageHasher::Blake2_256 => StorageHasher::Blake2_256,
            frame_metadata::v10::StorageHasher::Twox128 => StorageHasher::Twox128,
            frame_metadata::v10::StorageHasher::Twox256 => StorageHasher::Twox256,
            frame_metadata::v10::StorageHasher::Twox64Concat => StorageHasher::Twox64Concat,
        }
    }

    to_latest_storage_hasher!(to_storage_hasher_v11, frame_metadata::v11::StorageHasher);
    to_latest_storage_hasher!(to_storage_hasher_v12, frame_metadata::v12::StorageHasher);
    to_latest_storage_hasher!(to_storage_hasher_v13, frame_metadata::v13::StorageHasher);

    fn decode_lookup_name_or_err<S: AsRef<str>>(s: &DecodeDifferent<&str, S>, pallet_name: &str) -> Result<LookupName, StorageInfoError<'static>> {
        let ty = sanitize_type_name(as_decoded(s).as_ref());
        lookup_name_or_err(&ty, pallet_name)
    }
    
    fn lookup_name_or_err<'a>(ty: &'a str, pallet_name: &str) -> Result<LookupName, StorageInfoError<'static>> {
        let id = LookupName::parse(ty)
            .map_err(|e| StorageInfoError::CannotParseTypeName { 
                name: Cow::Owned(ty.to_owned()), 
                reason: e 
            })?
            .in_pallet(pallet_name);
        Ok(id)
    }
    
    fn sanitize_type_name(name: &str) -> std::borrow::Cow<'_, str> {
        if name.contains('\n') {
            std::borrow::Cow::Owned(name.replace('\n', ""))
        } else {
            std::borrow::Cow::Borrowed(name)
        }
    }
}