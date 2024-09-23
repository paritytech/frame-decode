use alloc::vec::Vec;
use alloc::vec;
use super::storage_type_info::{StorageTypeInfo, StorageHasher};
use crate::utils::{ decode_with_error_tracing, DecodeErrorTrace };
use scale_type_resolver::TypeResolver;
use crate::decoding::storage_type_info::StorageInfoError;
use core::ops::Range;

/// An error returned trying to decode storage bytes.
#[non_exhaustive]
#[derive(Clone, Debug)]
pub enum StorageKeyDecodeError<TypeId> {
    CannotGetInfo(StorageInfoError<'static>),
    PrefixMismatch,
    NotEnoughBytes { needed: usize, have: usize },
    CannotDecodeKey { ty: TypeId, reason: DecodeErrorTrace, decoded_so_far: StorageKey<TypeId> },
}

impl <TypeId> StorageKeyDecodeError<TypeId> {
    /// Map the storage key error type IDs to something else.
    pub fn map_type_id<NewTypeId, F>(self, mut f: F) -> StorageKeyDecodeError<NewTypeId> 
    where
        F: FnMut(TypeId) -> NewTypeId        
    {
        match self {
            StorageKeyDecodeError::CannotGetInfo(e) => {
                StorageKeyDecodeError::CannotGetInfo(e)
            }
            StorageKeyDecodeError::PrefixMismatch => {
                StorageKeyDecodeError::PrefixMismatch
            }
            StorageKeyDecodeError::NotEnoughBytes { needed, have } => {
                StorageKeyDecodeError::NotEnoughBytes { needed, have }
            }
            StorageKeyDecodeError::CannotDecodeKey { ty, reason, decoded_so_far } => {
                StorageKeyDecodeError::CannotDecodeKey { ty: f(ty), reason, decoded_so_far: decoded_so_far.map_type_id(f) }
            }
        }
    }
}

/// An error returned trying to decode storage bytes.
#[non_exhaustive]
#[derive(Clone, Debug)]
pub enum StorageValueDecodeError<TypeId> {
    CannotGetInfo(StorageInfoError<'static>),
    CannotDecodeValue { ty: TypeId, reason: DecodeErrorTrace },
}

impl <TypeId> StorageValueDecodeError<TypeId> {
    /// Map the storage value error type IDs to something else.
    pub fn map_type_id<NewTypeId, F>(self, mut f: F) -> StorageValueDecodeError<NewTypeId> 
    where
        F: FnMut(TypeId) -> NewTypeId        
    {
        match self { 
            StorageValueDecodeError::CannotGetInfo(e) => {
                StorageValueDecodeError::CannotGetInfo(e)
            }
            StorageValueDecodeError::CannotDecodeValue { ty, reason } => {
                StorageValueDecodeError::CannotDecodeValue { ty: f(ty), reason }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct StorageKey<TypeId> {
    parts: Vec<StorageKeyPart<TypeId>>
}

impl <TypeId> StorageKey<TypeId> {
    /// Iterate over the parts of this storage key.
    pub fn parts(&self) -> impl ExactSizeIterator<Item=&StorageKeyPart<TypeId>> {
        self.parts.iter()
    }

    /// Map the storage key type IDs to something else.
    pub fn map_type_id<NewTypeId, F>(self, mut f: F) -> StorageKey<NewTypeId> 
    where
        F: FnMut(TypeId) -> NewTypeId        
    {
        StorageKey {
            parts: self.parts
                .into_iter()
                .map(|p| p.map_type_id(&mut f))
                .collect()
        }
    }
}

/// The decoded representation of a storage key.
#[derive(Clone, Debug)]
pub struct StorageKeyPart<TypeId> {
    hash_range: Range<u32>,
    value: Option<StorageKeyPartValue<TypeId>>,
    hasher: StorageHasher,
}

impl <TypeId> StorageKeyPart<TypeId> {
    pub fn hash_range(&self) -> Range<usize> {
        Range { 
            start: self.hash_range.start as usize, 
            end: self.hash_range.end as usize 
        }
    }

    pub fn hasher(&self) -> StorageHasher {
        self.hasher
    }

    pub fn value(&self) -> Option<&StorageKeyPartValue<TypeId>> {
        self.value.as_ref()
    }

    /// Map the storage part type ID to something else.
    pub fn map_type_id<NewTypeId, F>(self, f: F) -> StorageKeyPart<NewTypeId> 
    where
        F: FnMut(TypeId) -> NewTypeId        
    {
        StorageKeyPart {
            hash_range: self.hash_range,
            value: self.value.map(|v| v.map_type_id(f)),
            hasher: self.hasher,
        }
    }
}

#[derive(Clone, Debug)]
pub struct StorageKeyPartValue<TypeId> {
    range: Range<u32>,
    ty: TypeId
}

impl <TypeId> StorageKeyPartValue<TypeId> {
    pub fn range(&self) -> Range<usize> {
        Range { 
            start: self.range.start as usize, 
            end: self.range.end as usize 
        }
    }

    pub fn ty(&self) -> &TypeId {
        &self.ty
    }

    /// Map the storage part type ID to something else.
    pub fn map_type_id<NewTypeId, F>(self, mut f: F) -> StorageKeyPartValue<NewTypeId> 
    where
        F: FnMut(TypeId) -> NewTypeId        
    {
        StorageKeyPartValue {
            range: self.range,
            ty: f(self.ty)
        }
    }
}

pub fn decode_storage_key<Info, Resolver>(
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
    let storage_info = info.get_storage_info(pallet_name, storage_entry)
        .map_err(|e| StorageKeyDecodeError::CannotGetInfo(e.into_owned()))?;

    let bytes = *cursor;
    let curr_idx = |cursor: &mut &[u8]| (bytes.len() - cursor.len()) as u32;

    let prefix = strip_bytes(cursor, 32)?;

    // Check that the storage key prefix is what we expect:
    let expected_prefix = {
        let mut v = Vec::<u8>::with_capacity(16);
        v.extend(&sp_crypto_hashing::twox_128(pallet_name.as_bytes()));
        v.extend(&sp_crypto_hashing::twox_128(storage_entry.as_bytes()));
        v
    };
    if prefix != expected_prefix {
        return Err(StorageKeyDecodeError::PrefixMismatch)
    }

    let mut parts = vec![];
    for key in storage_info.keys {
        let hasher = key.hasher;
        let start_idx = curr_idx(cursor);
        let part = match &hasher {
            StorageHasher::Blake2_128 |
            StorageHasher::Twox128 => {
                strip_bytes(cursor, 16)?;
                StorageKeyPart { 
                    hash_range: Range { start: start_idx, end: curr_idx(cursor) },
                    value: None,
                    hasher,
                } 
            },
            StorageHasher::Blake2_256 |
            StorageHasher::Twox256 => {
                strip_bytes(cursor, 32)?;
                StorageKeyPart { 
                    hash_range: Range { start: start_idx, end: curr_idx(cursor) },
                    value: None,
                    hasher,
                } 
            },
            StorageHasher::Blake2_128Concat => {
                strip_bytes(cursor, 16)?;
                let hash_end_idx = curr_idx(cursor);
                decode_with_error_tracing(
                    cursor, 
                    key.key_id.clone(),
                    type_resolver, 
                    scale_decode::visitor::IgnoreVisitor::new()
                ).map_err(|e| StorageKeyDecodeError::CannotDecodeKey { 
                    ty: key.key_id.clone(), 
                    reason: e, 
                    decoded_so_far: StorageKey { parts: parts.clone() }
                })?;
                StorageKeyPart { 
                    hash_range: Range { start: start_idx, end: hash_end_idx },
                    value: Some(StorageKeyPartValue {
                        range: Range { start: hash_end_idx, end: curr_idx(cursor) },
                        ty: key.key_id,
                    }),
                    hasher,
                }
            },
            StorageHasher::Twox64Concat => {
                strip_bytes(cursor, 8)?;
                let hash_end_idx = curr_idx(cursor);
                decode_with_error_tracing(
                    cursor, 
                    key.key_id.clone(),
                    type_resolver, 
                    scale_decode::visitor::IgnoreVisitor::new()
                ).map_err(|e| StorageKeyDecodeError::CannotDecodeKey { 
                    ty: key.key_id.clone(), 
                    reason: e, 
                    decoded_so_far: StorageKey { parts: parts.clone() }
                })?;
                StorageKeyPart { 
                    hash_range: Range { start: start_idx, end: hash_end_idx },
                    value: Some(StorageKeyPartValue {
                        range: Range { start: hash_end_idx, end: curr_idx(cursor) },
                        ty: key.key_id,
                    }),
                    hasher,
                }
            },
            StorageHasher::Identity => {
                decode_with_error_tracing(
                    cursor, 
                    key.key_id.clone(),
                    type_resolver, 
                    scale_decode::visitor::IgnoreVisitor::new()
                ).map_err(|e| StorageKeyDecodeError::CannotDecodeKey { 
                    ty: key.key_id.clone(), 
                    reason: e, 
                    decoded_so_far: StorageKey { parts: parts.clone() }
                })?;
                StorageKeyPart { 
                    hash_range: Range { start: start_idx, end: start_idx },
                    value: Some(StorageKeyPartValue {
                        range: Range { start: start_idx, end: curr_idx(cursor) },
                        ty: key.key_id,
                    }),
                    hasher,
                }
            },
        };
        parts.push(part)
    }

    Ok(StorageKey { parts })
}

pub fn decode_storage_value<'scale, 'resolver, Info, Resolver, V>(
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
    let storage_info = info.get_storage_info(pallet_name, storage_entry)
        .map_err(|e| StorageValueDecodeError::CannotGetInfo(e.into_owned()))?;

    let value_id = storage_info.value_id;

    let decoded = decode_with_error_tracing(cursor, value_id.clone(), type_resolver, visitor)
        .map_err(|e| StorageValueDecodeError::CannotDecodeValue { 
            ty: value_id, 
            reason: e
        })?;

    Ok(decoded)
}

fn strip_bytes<'a, T>(cursor: &mut &'a [u8], num: usize) -> Result<&'a [u8], StorageKeyDecodeError<T>> {
    let bytes = cursor
        .get(..num)
        .ok_or_else(|| StorageKeyDecodeError::NotEnoughBytes { needed: num, have: cursor.len() })?;

    *cursor = &cursor[num..];
    Ok(bytes)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_strip_bytes() {
        let v = vec![0,1,2,3,4,5,6,7,8];
        let cursor = &mut &*v;
        let stripped = strip_bytes::<()>(cursor, 4).unwrap();
        assert_eq!(stripped, &[0,1,2,3]);
        assert_eq!(cursor, &[4,5,6,7,8]);
    }
}