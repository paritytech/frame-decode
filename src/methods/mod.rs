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

pub mod constant_decoder;
pub mod constant_type_info;
pub mod custom_value_decoder;
pub mod custom_value_type_info;
pub mod extrinsic_decoder;
pub mod extrinsic_type_info;
pub mod runtime_api_decoder;
pub mod runtime_api_encoder;
pub mod runtime_api_type_info;
pub mod storage_decoder;
pub mod storage_encoder;
pub mod storage_type_info;
pub mod view_function_decoder;
pub mod view_function_encoder;
pub mod view_function_type_info;

use alloc::borrow::Cow;

/// This represents either an entry name, or the name of the thing that the entry is
/// in (for instance the name of a pallet or of a Runtime API trait).
///
/// Iterators returning this will iterate containers in order, first returning
/// [`Entry::In`] to communicate which container (eg pallet or Runtime API trait) is being
/// iterated over next, and then a [`Entry::Name`]s for the name of each of the entries in
/// the given container.
///
/// A container name will not be handed back more than once.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Entry<'info> {
    /// The name of the thing that the following entries are in/under.
    In(Cow<'info, str>),
    /// The name of the entry in/under the last given [`Entry::In`].
    Name(Cow<'info, str>),
}

impl<'info> Entry<'info> {
    /// Take ownership of the entry, converting any lifetimes to `'static`.
    pub fn into_owned(self) -> Entry<'static> {
        match self {
            Entry::In(name) => Entry::In(Cow::Owned(name.into_owned())),
            Entry::Name(name) => Entry::Name(Cow::Owned(name.into_owned())),
        }
    }

    /// Iterate over all of the entries in a specific container (ie all of the entries
    /// which follow a specific [`Entry::In`]).
    pub fn entries_in<'a>(
        entries: impl Iterator<Item = Entry<'a>>,
        container: &str,
    ) -> impl Iterator<Item = Cow<'a, str>> {
        entries
            .skip_while(|c| c != &Entry::In(Cow::Borrowed(container)))
            .skip(1)
            .take_while(|c| matches!(c, Entry::Name(_)))
            .filter_map(|c| match c {
                Entry::In(_) => None,
                Entry::Name(name) => Some(name),
            })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_entries_in() {
        fn entries() -> impl Iterator<Item = Entry<'static>> {
            [
                Entry::In("Baz".into()),
                Entry::In("Foo".into()),
                Entry::Name("foo_a".into()),
                Entry::Name("foo_b".into()),
                Entry::Name("foo_c".into()),
                Entry::In("Bar".into()),
                Entry::Name("bar_a".into()),
                Entry::In("Wibble".into()),
            ]
            .into_iter()
        }

        assert!(Entry::entries_in(entries(), "Baz").next().is_none());
        assert!(Entry::entries_in(entries(), "Wibble").next().is_none());

        let foos: Vec<String> = Entry::entries_in(entries(), "Foo")
            .map(|s| s.into_owned())
            .collect();
        assert_eq!(
            foos,
            Vec::from_iter(["foo_a".to_owned(), "foo_b".to_owned(), "foo_c".to_owned(),])
        );

        let bars: Vec<String> = Entry::entries_in(entries(), "Bar")
            .map(|s| s.into_owned())
            .collect();
        assert_eq!(bars, Vec::from_iter(["bar_a".to_owned(),]));
    }
}
