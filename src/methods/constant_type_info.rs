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

use alloc::borrow::Cow;
use alloc::string::String;

/// This can be implemented for anything capable of providing Constant information.
pub trait ConstantTypeInfo {
    /// The type of type IDs that we are using to obtain type information.
    type TypeId: Clone;
    /// Get information about a constant
    fn constant_info(
        &self,
        pallet_name: &str,
        constant_name: &str,
    ) -> Result<ConstantInfo<'_, Self::TypeId>, ConstantInfoError<'_>>;
    /// Iterate over all of the available Constants.
    fn constants(&self) -> impl Iterator<Item = Constant<'_>>;
}

/// An error returned trying to access Constant information.
#[non_exhaustive]
#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ConstantInfoError<'info> {
    #[error("Pallet `{pallet_name}` not found")]
    PalletNotFound { pallet_name: String },
    #[error("Constant `{constant_name}` not found in pallet `{pallet_name}`")]
    ConstantNotFound {
        pallet_name: Cow<'info, str>,
        constant_name: String,
    },
    #[cfg(feature = "legacy")]
    #[error("Cannot parse type name {name}:\n\n{reason}.")]
    CannotParseTypeName {
        name: Cow<'info, str>,
        reason: scale_info_legacy::lookup_name::ParseError,
    },
}

impl<'info> ConstantInfoError<'info> {
    /// Convert this error into an owned version.
    pub fn into_owned(self) -> ConstantInfoError<'static> {
        match self {
            ConstantInfoError::PalletNotFound { pallet_name } => {
                ConstantInfoError::PalletNotFound { pallet_name }
            }
            ConstantInfoError::ConstantNotFound {
                pallet_name,
                constant_name,
            } => ConstantInfoError::ConstantNotFound {
                pallet_name: Cow::Owned(pallet_name.into_owned()),
                constant_name,
            },
            #[cfg(feature = "legacy")]
            ConstantInfoError::CannotParseTypeName { name, reason } => {
                ConstantInfoError::CannotParseTypeName {
                    name: Cow::Owned(name.into()),
                    reason,
                }
            }
        }
    }
}

/// Information about a Constant.
pub struct ConstantInfo<'info, TypeId: Clone> {
    /// The bytes representing this constant.
    ///
    /// It is expected that these bytes are in the metadata
    /// and can be borrowed here.
    pub bytes: &'info [u8],
    /// The type of this constant.
    pub type_id: TypeId,
}

/// The identifier for a single Constant.
#[derive(Debug, Clone)]
pub struct Constant<'info> {
    /// The trait containing this Constant.
    pub pallet_name: Cow<'info, str>,
    /// The method name for this Constant.
    pub constant_name: Cow<'info, str>,
}

macro_rules! impl_constant_type_info_for_v14_to_v16 {
    ($path:path, $name:ident) => {
        const _: () = {
            use $path as path;
            impl ConstantTypeInfo for path::$name {
                type TypeId = u32;

                fn constant_info(
                    &self,
                    pallet_name: &str,
                    constant_name: &str,
                ) -> Result<ConstantInfo<'_, Self::TypeId>, ConstantInfoError<'_>> {
                    let pallet = self
                        .pallets
                        .iter()
                        .find(|p| p.name == pallet_name)
                        .ok_or_else(|| ConstantInfoError::PalletNotFound {
                            pallet_name: pallet_name.into(),
                        })?;

                    let pallet_name = &pallet.name;

                    let constant = pallet
                        .constants
                        .iter()
                        .find(|c| c.name == constant_name)
                        .ok_or_else(move || ConstantInfoError::ConstantNotFound {
                            pallet_name: Cow::Borrowed(pallet_name),
                            constant_name: constant_name.into(),
                        })?;

                    Ok(ConstantInfo {
                        bytes: &constant.value,
                        type_id: constant.ty.id,
                    })
                }

                fn constants(&self) -> impl Iterator<Item = Constant<'_>> {
                    self.pallets.iter().flat_map(|p| {
                        p.constants.iter().map(|c| Constant {
                            pallet_name: Cow::Borrowed(&p.name),
                            constant_name: Cow::Borrowed(&c.name),
                        })
                    })
                }
            }
        };
    };
}

impl_constant_type_info_for_v14_to_v16!(frame_metadata::v14, RuntimeMetadataV14);
impl_constant_type_info_for_v14_to_v16!(frame_metadata::v15, RuntimeMetadataV15);
impl_constant_type_info_for_v14_to_v16!(frame_metadata::v16, RuntimeMetadataV16);

#[cfg(feature = "legacy")]
mod legacy {
    use super::*;
    use crate::utils::as_decoded;
    use frame_metadata::decode_different::DecodeDifferent;
    use scale_info_legacy::LookupName;

    macro_rules! impl_constant_type_info_for_v8_to_v13 {
        ($path:path, $name:ident) => {
            const _: () = {
                use $path as path;
                impl ConstantTypeInfo for path::$name {
                    type TypeId = LookupName;

                    fn constant_info(
                        &self,
                        pallet_name: &str,
                        constant_name: &str,
                    ) -> Result<ConstantInfo<'_, Self::TypeId>, ConstantInfoError<'_>> {
                        let modules = as_decoded(&self.modules);

                        let m = modules
                            .iter()
                            .find(|m| as_decoded(&m.name).as_ref() as &str == pallet_name)
                            .ok_or_else(|| ConstantInfoError::PalletNotFound {
                                pallet_name: pallet_name.into(),
                            })?;

                        let pallet_name = as_decoded(&m.name);
                        let constants = as_decoded(&m.constants);

                        let constant = constants
                            .iter()
                            .find(|c| as_decoded(&c.name).as_ref() as &str == constant_name)
                            .ok_or_else(|| ConstantInfoError::ConstantNotFound {
                                pallet_name: Cow::Borrowed(pallet_name),
                                constant_name: constant_name.into(),
                            })?;

                        let type_id = decode_lookup_name_or_err(&constant.ty, pallet_name)?;
                        let data = as_decoded(&constant.value);

                        Ok(ConstantInfo {
                            bytes: &**data,
                            type_id,
                        })
                    }

                    fn constants(&self) -> impl Iterator<Item = Constant<'_>> {
                        as_decoded(&self.modules).iter().flat_map(|module| {
                            let pallet_name = as_decoded(&module.name);
                            let constants = as_decoded(&module.constants);

                            constants.iter().map(|constant| {
                                let constant_name = as_decoded(&constant.name);
                                Constant {
                                    pallet_name: Cow::Borrowed(pallet_name),
                                    constant_name: Cow::Borrowed(constant_name),
                                }
                            })
                        })
                    }
                }
            };
        };
    }

    impl_constant_type_info_for_v8_to_v13!(frame_metadata::v8, RuntimeMetadataV8);
    impl_constant_type_info_for_v8_to_v13!(frame_metadata::v9, RuntimeMetadataV9);
    impl_constant_type_info_for_v8_to_v13!(frame_metadata::v10, RuntimeMetadataV10);
    impl_constant_type_info_for_v8_to_v13!(frame_metadata::v11, RuntimeMetadataV11);
    impl_constant_type_info_for_v8_to_v13!(frame_metadata::v12, RuntimeMetadataV12);
    impl_constant_type_info_for_v8_to_v13!(frame_metadata::v13, RuntimeMetadataV13);

    fn decode_lookup_name_or_err<S: AsRef<str>>(
        s: &DecodeDifferent<&str, S>,
        pallet_name: &str,
    ) -> Result<LookupName, ConstantInfoError<'static>> {
        let ty = sanitize_type_name(as_decoded(s).as_ref());
        lookup_name_or_err(&ty, pallet_name)
    }

    fn lookup_name_or_err(
        ty: &str,
        pallet_name: &str,
    ) -> Result<LookupName, ConstantInfoError<'static>> {
        let id = LookupName::parse(ty)
            .map_err(|e| ConstantInfoError::CannotParseTypeName {
                name: Cow::Owned(ty.into()),
                reason: e,
            })?
            .in_pallet(pallet_name);
        Ok(id)
    }

    fn sanitize_type_name(name: &str) -> Cow<'_, str> {
        if name.contains('\n') {
            Cow::Owned(name.replace('\n', ""))
        } else {
            Cow::Borrowed(name)
        }
    }
}
