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
use alloc::borrow::ToOwned;
use alloc::vec::Vec;

/// This can be implemented for anything capable of providing Runtime API type information.
/// It is implemented for newer versions of frame-metadata (V15 and above).
pub trait RuntimeApiTypeInfo {
    /// The type of type IDs that we are using to obtain type information.
    type TypeId;
    /// Get the information needed to encode/decode a specific Runtime API call
    fn get_runtime_api_info(
        &self,
        trait_name: &str,
        method_name: &str,
    ) -> Result<RuntimeApiInfo<'_, Self::TypeId>, RuntimeApiInfoError<'_>>;
    /// Iterate over all of the available Runtime APIs.
    fn runtime_apis(&self) -> impl Iterator<Item = RuntimeApi<'_>>;
}

/// An error returned trying to access Runtime API type information.
#[non_exhaustive]
#[allow(missing_docs)]
#[derive(Clone, Debug, thiserror::Error)]
pub enum RuntimeApiInfoError<'info> {
    #[error("Runtime API trait `{trait_name}` not found")]
    TraitNotFound { trait_name: String },
    #[error("Runtime API method `{method_name}` not found in trait `{trait_name}`")]
    MethodNotFound {
        trait_name: Cow<'info, str>,
        method_name: String,
    },
}

impl <'info> RuntimeApiInfoError<'info> {
    /// Take ownership of this error, turning any lifetimes to `'static`.
    pub fn into_owned(self) -> RuntimeApiInfoError<'static> {
        match self {
            RuntimeApiInfoError::TraitNotFound { trait_name } => {
                RuntimeApiInfoError::TraitNotFound {
                    trait_name,
                }
            }
            RuntimeApiInfoError::MethodNotFound { trait_name, method_name } => {
                RuntimeApiInfoError::MethodNotFound {
                    trait_name: Cow::Owned(trait_name.into_owned()),
                    method_name,
                }
            }
        }
    }
}

/// Information about a Runtime API.
pub struct RuntimeApiInfo<'a, TypeId> {
    /// Inputs to the runtime API.
    pub inputs: Vec<RuntimeApiInput<'a, TypeId>>,
    /// The output type returned from the runtime API.
    pub output_id: TypeId,
}

/// Information about a specific input value to a Runtime API.
pub struct RuntimeApiInput<'a, TypeId> {
    /// Name of the input.
    pub name: Cow<'a, str>,
    /// Type of the input.
    pub id: TypeId,
}

/// Details about a single storage entry.
#[derive(Debug, Clone)]
pub struct RuntimeApi<'a> {
    /// The trait containing this Runtime API.
    pub trait_name: Cow<'a, str>,
    /// The method name for this Runtime API.
    pub method_name: Cow<'a, str>,
}

macro_rules! impl_runtime_api_info_for_v15_to_v16 {
    ($path:path, $name:ident) => {
        const _: () = {
            use $path as path;
            impl RuntimeApiTypeInfo for path::$name {
                type TypeId = u32;

                fn get_runtime_api_info(
                    &self,
                    trait_name: &str,
                    method_name: &str,
                ) -> Result<RuntimeApiInfo<'_, Self::TypeId>, RuntimeApiInfoError<'_>> {
                    let api = self.apis.iter().find(|api| api.name == trait_name).ok_or(
                        RuntimeApiInfoError::TraitNotFound {
                            trait_name: trait_name.to_owned(),
                        },
                    )?;

                    let method = api
                        .methods
                        .iter()
                        .find(|method| method.name == method_name)
                        .ok_or(RuntimeApiInfoError::MethodNotFound {
                            trait_name: Cow::Borrowed(&api.name),
                            method_name: method_name.to_owned(),
                        })?;

                    let inputs = method
                        .inputs
                        .iter()
                        .map(|arg| RuntimeApiInput {
                            name: Cow::Borrowed(&arg.name),
                            id: arg.ty.id,
                        })
                        .collect();

                    Ok(RuntimeApiInfo {
                        inputs,
                        output_id: method.output.id,
                    })
                }

                fn runtime_apis(&self) -> impl Iterator<Item = RuntimeApi<'_>> {
                    self.apis.iter().flat_map(|api| {
                        api.methods.iter().map(|method| RuntimeApi {
                            trait_name: Cow::Borrowed(&api.name),
                            method_name: Cow::Borrowed(&method.name),
                        })
                    })
                }
            }
        };
    };
}

impl_runtime_api_info_for_v15_to_v16!(frame_metadata::v15, RuntimeMetadataV15);
impl_runtime_api_info_for_v15_to_v16!(frame_metadata::v16, RuntimeMetadataV16);

#[cfg(feature = "legacy")]
mod legacy {
    use super::*;
    use scale_info_legacy::{lookup_name, TypeRegistry, TypeRegistrySet};

    impl RuntimeApiTypeInfo for TypeRegistry {
        type TypeId = lookup_name::LookupName;

        fn get_runtime_api_info(
            &self,
            trait_name: &str,
            method_name: &str,
        ) -> Result<RuntimeApiInfo<'_, Self::TypeId>, RuntimeApiInfoError<'_>> {
            let api = self.runtime_api(trait_name, method_name).ok_or_else(|| {
                RuntimeApiInfoError::MethodNotFound {
                    trait_name: Cow::Owned(trait_name.to_owned()),
                    method_name: method_name.to_owned(),
                }
            })?;

            let inputs = api
                .inputs
                .iter()
                .map(|input| RuntimeApiInput {
                    name: Cow::Borrowed(&*input.name),
                    id: input.id.clone(),
                })
                .collect();

            Ok(RuntimeApiInfo {
                inputs,
                output_id: api.output.clone(),
            })
        }

        fn runtime_apis(&self) -> impl Iterator<Item = RuntimeApi<'_>> {
            self.runtime_apis()
                .map(|(trait_name, method_name)| RuntimeApi {
                    trait_name: Cow::Borrowed(trait_name),
                    method_name: Cow::Borrowed(method_name),
                })
        }
    }

    impl<'a> RuntimeApiTypeInfo for TypeRegistrySet<'a> {
        type TypeId = lookup_name::LookupName;

        fn get_runtime_api_info(
            &self,
            trait_name: &str,
            method_name: &str,
        ) -> Result<RuntimeApiInfo<'_, Self::TypeId>, RuntimeApiInfoError<'_>> {
            let api = self.runtime_api(trait_name, method_name).ok_or_else(|| {
                RuntimeApiInfoError::MethodNotFound {
                    trait_name: Cow::Owned(trait_name.to_owned()),
                    method_name: method_name.to_owned(),
                }
            })?;

            let inputs = api
                .inputs
                .iter()
                .map(|input| RuntimeApiInput {
                    name: Cow::Borrowed(&*input.name),
                    id: input.id.clone(),
                })
                .collect();

            Ok(RuntimeApiInfo {
                inputs,
                output_id: api.output.clone(),
            })
        }

        fn runtime_apis(&self) -> impl Iterator<Item = RuntimeApi<'_>> {
            self.runtime_apis()
                .map(|(trait_name, method_name)| RuntimeApi {
                    trait_name: Cow::Borrowed(trait_name),
                    method_name: Cow::Borrowed(method_name),
                })
        }
    }
}
