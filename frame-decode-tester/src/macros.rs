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

//! Internal macros for reducing metadata version boilerplate.

/// Dispatch on `RuntimeMetadata` version with a uniform expression for all supported versions.
///
/// # Example
/// ```ignore
/// let items = with_metadata_uniform!(metadata, |m| {
///     m.storage_tuples().map(|(p, s)| (p.into_owned(), s.into_owned())).collect()
/// })?;
/// ```
#[macro_export]
macro_rules! with_metadata_uniform {
    ($metadata:expr, |$m:ident| $body:expr) => {{
        use frame_metadata::RuntimeMetadata as RM;
        match $metadata {
            RM::V8($m) => Ok($body),
            RM::V9($m) => Ok($body),
            RM::V10($m) => Ok($body),
            RM::V11($m) => Ok($body),
            RM::V12($m) => Ok($body),
            RM::V13($m) => Ok($body),
            RM::V14($m) => Ok($body),
            RM::V15($m) => Ok($body),
            RM::V16($m) => Ok($body),
            _ => Err("Unsupported metadata version"),
        }
    }};
}

/// Dispatch on `RuntimeMetadata` version with different type resolvers for legacy (V8-V13)
/// vs modern (V14+) metadata.
///
/// - For V8-V13: uses `$legacy_resolver` as the type resolver
/// - For V14-V16: uses `&m.types` (the embedded PortableRegistry)
///
/// # Example
/// ```ignore
/// let result = with_metadata_versioned!(
///     metadata,
///     legacy_types,  // resolver for V8-V13
///     |m, resolver| decode_inner(bytes, m, resolver)
/// );
/// ```
#[macro_export]
macro_rules! with_metadata_versioned {
    ($metadata:expr, $legacy_resolver:expr, |$m:ident, $resolver:ident| $body:expr) => {{
        use frame_metadata::RuntimeMetadata as RM;
        match $metadata {
            RM::V8($m) => {
                let $resolver = $legacy_resolver;
                $body
            }
            RM::V9($m) => {
                let $resolver = $legacy_resolver;
                $body
            }
            RM::V10($m) => {
                let $resolver = $legacy_resolver;
                $body
            }
            RM::V11($m) => {
                let $resolver = $legacy_resolver;
                $body
            }
            RM::V12($m) => {
                let $resolver = $legacy_resolver;
                $body
            }
            RM::V13($m) => {
                let $resolver = $legacy_resolver;
                $body
            }
            RM::V14($m) => {
                let $resolver = &$m.types;
                $body
            }
            RM::V15($m) => {
                let $resolver = &$m.types;
                $body
            }
            RM::V16($m) => {
                let $resolver = &$m.types;
                $body
            }
            _ => Err("Unsupported metadata version".to_string()),
        }
    }};
}
