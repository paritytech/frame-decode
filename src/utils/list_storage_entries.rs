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

use alloc::borrow::Cow;
use alloc::boxed::Box;
use frame_metadata::RuntimeMetadata;

/// Returns an iterator listing the available storage entries in some metadata.
pub fn list_storage_entries(metadata: &RuntimeMetadata) -> impl Iterator<Item = StorageEntry<'_>> {
    match metadata {
        RuntimeMetadata::V0(_deprecated_metadata)
        | RuntimeMetadata::V1(_deprecated_metadata)
        | RuntimeMetadata::V2(_deprecated_metadata)
        | RuntimeMetadata::V3(_deprecated_metadata)
        | RuntimeMetadata::V4(_deprecated_metadata)
        | RuntimeMetadata::V5(_deprecated_metadata)
        | RuntimeMetadata::V6(_deprecated_metadata)
        | RuntimeMetadata::V7(_deprecated_metadata) => {
            Box::new(core::iter::empty()) as Box<dyn Iterator<Item = StorageEntry<'_>>>
        }
        #[cfg(feature = "legacy")]
        RuntimeMetadata::V8(m) => Box::new(m.storage_entries_list()),
        #[cfg(not(feature = "legacy"))]
        RuntimeMetadata::V8(_opaque) => Box::new(core::iter::empty()),
        #[cfg(feature = "legacy")]
        RuntimeMetadata::V9(m) => Box::new(m.storage_entries_list()),
        #[cfg(not(feature = "legacy"))]
        RuntimeMetadata::V9(_opaque) => Box::new(core::iter::empty()),
        #[cfg(feature = "legacy")]
        RuntimeMetadata::V10(m) => Box::new(m.storage_entries_list()),
        #[cfg(not(feature = "legacy"))]
        RuntimeMetadata::V10(_opaque) => Box::new(core::iter::empty()),
        #[cfg(feature = "legacy")]
        RuntimeMetadata::V11(m) => Box::new(m.storage_entries_list()),
        #[cfg(not(feature = "legacy"))]
        RuntimeMetadata::V11(_opaque) => Box::new(core::iter::empty()),
        #[cfg(feature = "legacy")]
        RuntimeMetadata::V12(m) => Box::new(m.storage_entries_list()),
        #[cfg(not(feature = "legacy"))]
        RuntimeMetadata::V12(_opaque) => Box::new(core::iter::empty()),
        #[cfg(feature = "legacy")]
        RuntimeMetadata::V13(m) => Box::new(m.storage_entries_list()),
        #[cfg(not(feature = "legacy"))]
        RuntimeMetadata::V13(_opaque) => Box::new(core::iter::empty()),
        RuntimeMetadata::V14(m) => Box::new(m.storage_entries_list()),
        RuntimeMetadata::V15(m) => Box::new(m.storage_entries_list()),
    }
}

/// Details about a single storage entry.
#[derive(Debug, Clone)]
pub struct StorageEntry<'a> {
    pallet: Cow<'a, str>,
    entry: Cow<'a, str>,
}

impl<'a> StorageEntry<'a> {
    /// Take ownership of this storage entry, converting lifetimes to `'static`
    pub fn into_owned(self) -> StorageEntry<'static> {
        StorageEntry {
            pallet: Cow::Owned(self.pallet.into_owned()),
            entry: Cow::Owned(self.entry.into_owned()),
        }
    }

    /// Name of the pallet containing the storage entry.
    pub fn pallet(&self) -> &str {
        &self.pallet
    }

    /// Name of the storage entry.
    pub fn entry(&self) -> &str {
        &self.entry
    }
}

trait StorageEntriesList {
    /// List all of the storage entries available in some metadata.
    fn storage_entries_list(&self) -> impl Iterator<Item = StorageEntry<'_>>;
}

#[cfg(feature = "legacy")]
const _: () = {
    macro_rules! impl_storage_entries_list_for_v8_to_v13 {
        ($path:path) => {
            impl StorageEntriesList for $path {
                fn storage_entries_list(&self) -> impl Iterator<Item = StorageEntry<'_>> {
                    use crate::utils::as_decoded;
                    as_decoded(&self.modules).iter().flat_map(|module| {
                        let Some(storage) = &module.storage else {
                            return Either::Left(core::iter::empty());
                        };
                        let pallet = as_decoded(&module.name);
                        let storage = as_decoded(storage);
                        let entries = as_decoded(&storage.entries);

                        Either::Right(entries.iter().map(|entry_meta| {
                            let entry = as_decoded(&entry_meta.name);
                            StorageEntry {
                                pallet: Cow::Borrowed(pallet.as_str()),
                                entry: Cow::Borrowed(entry.as_str()),
                            }
                        }))
                    })
                }
            }
        };
    }

    impl_storage_entries_list_for_v8_to_v13!(frame_metadata::v8::RuntimeMetadataV8);
    impl_storage_entries_list_for_v8_to_v13!(frame_metadata::v9::RuntimeMetadataV9);
    impl_storage_entries_list_for_v8_to_v13!(frame_metadata::v10::RuntimeMetadataV10);
    impl_storage_entries_list_for_v8_to_v13!(frame_metadata::v11::RuntimeMetadataV11);
    impl_storage_entries_list_for_v8_to_v13!(frame_metadata::v12::RuntimeMetadataV12);
    impl_storage_entries_list_for_v8_to_v13!(frame_metadata::v13::RuntimeMetadataV13);
};

macro_rules! impl_storage_entries_list_for_v14_to_v15 {
    ($path:path) => {
        impl StorageEntriesList for $path {
            fn storage_entries_list(&self) -> impl Iterator<Item = StorageEntry<'_>> {
                self.pallets.iter().flat_map(|pallet| {
                    let Some(storage) = &pallet.storage else {
                        return Either::Left(core::iter::empty());
                    };

                    Either::Right(storage.entries.iter().map(|entry_meta| {
                        let entry = &entry_meta.name;
                        StorageEntry {
                            pallet: Cow::Borrowed(pallet.name.as_ref()),
                            entry: Cow::Borrowed(entry.as_ref()),
                        }
                    }))
                })
            }
        }
    };
}

impl_storage_entries_list_for_v14_to_v15!(frame_metadata::v14::RuntimeMetadataV14);
impl_storage_entries_list_for_v14_to_v15!(frame_metadata::v15::RuntimeMetadataV15);

enum Either<L, R> {
    Left(L),
    Right(R),
}

impl<L, R> Iterator for Either<L, R>
where
    L: Iterator,
    R: Iterator<Item = L::Item>,
{
    type Item = L::Item;
    fn next(&mut self) -> Option<L::Item> {
        match self {
            Either::Left(l) => l.next(),
            Either::Right(r) => r.next(),
        }
    }
}
