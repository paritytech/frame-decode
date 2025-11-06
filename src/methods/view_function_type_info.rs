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

use super::Entry;
use crate::utils::Either;
use alloc::borrow::Cow;
use alloc::borrow::ToOwned;
use alloc::string::String;

/// This is implemented for anything capable of providing information about view functions
/// (primarily metadata V16 and onwards).
pub trait ViewFunctionTypeInfo {
    /// The type of type IDs that we are using to obtain type information.
    type TypeId: Clone;
    /// Get the information needed to decode a specific View Function.
    fn view_function_info(
        &self,
        pallet_name: &str,
        function_name: &str,
    ) -> Result<ViewFunctionInfo<'_, Self::TypeId>, ViewFunctionInfoError<'_>>;
    /// Iterate over all of the available View Functions.
    fn view_functions(&self) -> impl Iterator<Item = Entry<'_>>;
    /// Iterate over all of the available View Functions in a given pallet.
    fn view_functions_in_pallet(&self, pallet: &str) -> impl Iterator<Item = Cow<'_, str>> {
        Entry::entries_in(self.view_functions(), pallet)
    }
}

/// An error returned trying to access View Function type information.
#[non_exhaustive]
#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ViewFunctionInfoError<'info> {
    #[error("Pallet `{pallet_name}` not found")]
    PalletNotFound { pallet_name: String },
    #[error("View function `{function_name}` not found in pallet `{pallet_name}`")]
    FunctionNotFound {
        pallet_name: Cow<'info, str>,
        function_name: String,
    },
}

impl<'info> ViewFunctionInfoError<'info> {
    /// Take ownership of this error, turning any lifetimes to `'static`.
    pub fn into_owned(self) -> ViewFunctionInfoError<'static> {
        match self {
            ViewFunctionInfoError::PalletNotFound { pallet_name } => {
                ViewFunctionInfoError::PalletNotFound { pallet_name }
            }
            ViewFunctionInfoError::FunctionNotFound {
                pallet_name,
                function_name,
            } => ViewFunctionInfoError::FunctionNotFound {
                pallet_name: Cow::Owned(pallet_name.into_owned()),
                function_name,
            },
        }
    }
}

/// Information about a View Function.
#[derive(Debug, Clone)]
pub struct ViewFunctionInfo<'info, TypeId: Clone> {
    /// The query Id to use to call the view function.
    pub query_id: [u8; 32],
    /// Inputs to the runtime API.
    pub inputs: Cow<'info, [ViewFunctionInput<'info, TypeId>]>,
    /// The output type returned from the runtime API.
    pub output_id: TypeId,
}

impl<'info, TypeId: Clone> ViewFunctionInfo<'info, TypeId> {
    /// Take ownership of this info, turning any lifetimes to `'static`.
    pub fn into_owned(self) -> ViewFunctionInfo<'static, TypeId> {
        let inputs = self
            .inputs
            .iter()
            .map(|input| input.clone().into_owned())
            .collect();

        ViewFunctionInfo {
            query_id: self.query_id,
            inputs,
            output_id: self.output_id,
        }
    }
}

/// Information about a specific input value to a View Function.
#[derive(Debug, Clone)]
pub struct ViewFunctionInput<'info, TypeId> {
    /// Name of the input.
    pub name: Cow<'info, str>,
    /// Type of the input.
    pub id: TypeId,
}

impl<'info, TypeId: Clone> ViewFunctionInput<'info, TypeId> {
    /// Take ownership of this input, turning any lifetimes to `'static`.
    pub fn into_owned(self) -> ViewFunctionInput<'static, TypeId> {
        ViewFunctionInput {
            name: Cow::Owned(self.name.into_owned()),
            id: self.id,
        }
    }
}

impl ViewFunctionTypeInfo for frame_metadata::v16::RuntimeMetadataV16 {
    type TypeId = u32;

    fn view_function_info(
        &self,
        pallet_name: &str,
        function_name: &str,
    ) -> Result<ViewFunctionInfo<'_, Self::TypeId>, ViewFunctionInfoError<'_>> {
        let pallet = self
            .pallets
            .iter()
            .find(|p| p.name == pallet_name)
            .ok_or_else(|| ViewFunctionInfoError::PalletNotFound {
                pallet_name: pallet_name.to_owned(),
            })?;

        let view_fn = pallet
            .view_functions
            .iter()
            .find(|vf| vf.name == function_name)
            .ok_or_else(|| ViewFunctionInfoError::FunctionNotFound {
                pallet_name: Cow::Borrowed(&pallet.name),
                function_name: function_name.to_owned(),
            })?;

        let inputs = view_fn
            .inputs
            .iter()
            .map(|input| ViewFunctionInput {
                name: Cow::Borrowed(&input.name),
                id: input.ty.id,
            })
            .collect();

        Ok(ViewFunctionInfo {
            query_id: view_fn.id,
            inputs,
            output_id: view_fn.output.id,
        })
    }

    fn view_functions(&self) -> impl Iterator<Item = Entry<'_>> {
        self.pallets.iter().flat_map(|pallet| {
            core::iter::once(Entry::In(Cow::Borrowed(&pallet.name))).chain(
                pallet
                    .view_functions
                    .iter()
                    .map(|vf| Entry::Name(Cow::Borrowed(&vf.name))),
            )
        })
    }

    fn view_functions_in_pallet(&self, pallet_name: &str) -> impl Iterator<Item = Cow<'_, str>> {
        let pallet = self.pallets.iter().find(|p| p.name == pallet_name);

        let Some(pallet) = pallet else {
            return Either::Left(core::iter::empty());
        };

        let pallet_vfs = pallet
            .view_functions
            .iter()
            .map(|v| Cow::Borrowed(&*v.name));

        Either::Right(pallet_vfs)
    }
}
