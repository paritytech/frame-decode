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

use crate::methods::storage_type_info::StorageTypeInfo;
use alloc::boxed::Box;
use frame_metadata::RuntimeMetadata;

pub use crate::methods::Entry;

/// Returns an iterator listing the available storage entries in some metadata.
///
/// This basically calls [`StorageTypeInfo::storage_entries()`] for each metadata version,
/// returning an empty iterator where applicable (ie when passing legacy metadata and the
/// `legacy` features flag is not enabled).
pub fn list_storage_entries_any(metadata: &RuntimeMetadata) -> impl Iterator<Item = Entry<'_>> {
    match metadata {
        RuntimeMetadata::V0(_deprecated_metadata)
        | RuntimeMetadata::V1(_deprecated_metadata)
        | RuntimeMetadata::V2(_deprecated_metadata)
        | RuntimeMetadata::V3(_deprecated_metadata)
        | RuntimeMetadata::V4(_deprecated_metadata)
        | RuntimeMetadata::V5(_deprecated_metadata)
        | RuntimeMetadata::V6(_deprecated_metadata)
        | RuntimeMetadata::V7(_deprecated_metadata) => {
            Box::new(core::iter::empty()) as Box<dyn Iterator<Item = Entry<'_>>>
        }
        #[cfg(feature = "legacy")]
        RuntimeMetadata::V8(m) => Box::new(m.storage_entries()),
        #[cfg(not(feature = "legacy"))]
        RuntimeMetadata::V8(_opaque) => Box::new(core::iter::empty()),
        #[cfg(feature = "legacy")]
        RuntimeMetadata::V9(m) => Box::new(m.storage_entries()),
        #[cfg(not(feature = "legacy"))]
        RuntimeMetadata::V9(_opaque) => Box::new(core::iter::empty()),
        #[cfg(feature = "legacy")]
        RuntimeMetadata::V10(m) => Box::new(m.storage_entries()),
        #[cfg(not(feature = "legacy"))]
        RuntimeMetadata::V10(_opaque) => Box::new(core::iter::empty()),
        #[cfg(feature = "legacy")]
        RuntimeMetadata::V11(m) => Box::new(m.storage_entries()),
        #[cfg(not(feature = "legacy"))]
        RuntimeMetadata::V11(_opaque) => Box::new(core::iter::empty()),
        #[cfg(feature = "legacy")]
        RuntimeMetadata::V12(m) => Box::new(m.storage_entries()),
        #[cfg(not(feature = "legacy"))]
        RuntimeMetadata::V12(_opaque) => Box::new(core::iter::empty()),
        #[cfg(feature = "legacy")]
        RuntimeMetadata::V13(m) => Box::new(m.storage_entries()),
        #[cfg(not(feature = "legacy"))]
        RuntimeMetadata::V13(_opaque) => Box::new(core::iter::empty()),
        RuntimeMetadata::V14(m) => Box::new(m.storage_entries()),
        RuntimeMetadata::V15(m) => Box::new(m.storage_entries()),
        RuntimeMetadata::V16(m) => Box::new(m.storage_entries()),
    }
}
