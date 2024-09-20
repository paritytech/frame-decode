use scale_info_legacy::LookupName;
use alloc::borrow::Cow;
use alloc::vec::Vec;
use crate::utils::as_decoded;

/// This is implemented for all metadatas exposed from `frame_metadata` and is responsible for extracting the
/// type IDs that we need in order to decode extrinsics.
pub trait ExtrinsicTypeInfo {
    type TypeId;
    /// Get the information about a given extrinsic.
    fn get_extrinsic_info<'a>(&'a self, pallet_index: u8, call_index: u8) -> Result<ExtrinsicInfo<'a, Self::TypeId>, ExtrinsicInfoError<'a>>;
    /// Get the information needed to decode the extrinsic signature bytes.
    fn get_signature_info<'a>(&'a self) -> Result<ExtrinsicSignatureInfo<'a, Self::TypeId>, ExtrinsicInfoError<'a>>;
}
 
/// An error returned trying to access extrinsic type information.
#[non_exhaustive]
pub enum ExtrinsicInfoError<'a> {
    PalletNotFound { index: u8 },
    CallNotFound { index: u8, pallet_index: u8, pallet_name: Cow<'a, str> },
    CannotParseTypeName { name: Cow<'a, str>, reason: scale_info_legacy::lookup_name::ParseError },
    CallsTypeNotFound { id: u32, pallet_index: u8, pallet_name: Cow<'a, str> },
    CallsTypeShouldBeVariant { id: u32, pallet_index: u8, pallet_name: Cow<'a, str> },
    ExtrinsicTypeNotFound { id: u32 },
    ExtrinsicAddressTypeNotFound,
    ExtrinsicSignatureTypeNotFound,
}

impl <'a> ExtrinsicInfoError<'a> {
    pub fn into_owned(self) -> ExtrinsicInfoError<'static> {
        match self {
            ExtrinsicInfoError::PalletNotFound { index } => {
                ExtrinsicInfoError::PalletNotFound { index }
            },
            ExtrinsicInfoError::CallNotFound { index, pallet_index, pallet_name } => {
                ExtrinsicInfoError::CallNotFound { index, pallet_index, pallet_name: Cow::Owned(pallet_name.into_owned()) }
            },
            ExtrinsicInfoError::CannotParseTypeName { name, reason } => {
                ExtrinsicInfoError::CannotParseTypeName { name: Cow::Owned(name.into_owned()), reason }
            },
            ExtrinsicInfoError::CallsTypeNotFound { id, pallet_index, pallet_name } => {
                ExtrinsicInfoError::CallsTypeNotFound { id, pallet_index, pallet_name: Cow::Owned(pallet_name.into_owned()) }
            },
            ExtrinsicInfoError::CallsTypeShouldBeVariant { id, pallet_index, pallet_name } => {
                ExtrinsicInfoError::CallsTypeShouldBeVariant { id, pallet_index, pallet_name: Cow::Owned(pallet_name.into_owned()) }
            },
            ExtrinsicInfoError::ExtrinsicTypeNotFound { id } => {
                ExtrinsicInfoError::ExtrinsicTypeNotFound { id }
            },
            ExtrinsicInfoError::ExtrinsicAddressTypeNotFound => {
                ExtrinsicInfoError::ExtrinsicAddressTypeNotFound
            },
            ExtrinsicInfoError::ExtrinsicSignatureTypeNotFound => {
                ExtrinsicInfoError::ExtrinsicSignatureTypeNotFound
            }
        }
    }
}

#[derive(Debug)]
pub struct ExtrinsicInfoArg<'a, TypeId> {
    pub name: Cow<'a, str>,
    pub id: TypeId,
}

#[derive(Debug)]
pub struct ExtrinsicInfo<'a, TypeId> {
    pub pallet_name: Cow<'a, str>,
    pub call_name: Cow<'a, str>,
    pub args: Vec<ExtrinsicInfoArg<'a, TypeId>>
}

#[derive(Debug)]
pub struct ExtrinsicSignatureInfo<'a, TypeId> {
    pub address_id: TypeId,
    pub signature_id: TypeId,
    pub signed_extension_ids: Vec<ExtrinsicInfoArg<'a, TypeId>>
}

macro_rules! impl_extrinsic_info_body_for_v8_to_v11 {
    ($self:ident, $pallet_index:ident, $call_index:ident) => {{
        let modules = as_decoded(&$self.modules);

        let m = modules
            .iter()
            .filter(|m| m.calls.is_some())
            .nth($pallet_index as usize)
            .ok_or_else(|| ExtrinsicInfoError::PalletNotFound { index: $pallet_index })?;

        let m_name = as_decoded(&m.name);

        let calls = m.calls
            .as_ref()
            .ok_or_else(|| ExtrinsicInfoError::CallNotFound { 
                index: $call_index, 
                pallet_index: $pallet_index, 
                pallet_name: Cow::Borrowed(m_name),
            })?;

        let calls = as_decoded(calls);

        let call = calls
            .get($call_index as usize)
            .ok_or_else(|| ExtrinsicInfoError::CallNotFound { 
                index: $call_index, 
                pallet_index: $pallet_index, 
                pallet_name: Cow::Borrowed(m_name),
            })?;

        let c_name = as_decoded(&call.name);

        let args = as_decoded(&call.arguments);

        let args = args.iter().map(|a| {
            let ty = as_decoded(&a.ty);
            let id = parse_lookup_name(ty)?.in_pallet(m_name);
            let name = as_decoded(&a.name);
            Ok(ExtrinsicInfoArg { id, name: Cow::Borrowed(name) })
        }).collect::<Result<_, ExtrinsicInfoError>>()?;

        Ok(ExtrinsicInfo {
            pallet_name: Cow::Borrowed(m_name),
            call_name: Cow::Borrowed(c_name),
            args
        })
    }}
}

macro_rules! impl_for_v8_to_v10 {
    ($path:path) => {
        impl ExtrinsicTypeInfo for $path {
            type TypeId = LookupName;
            fn get_extrinsic_info(&self, pallet_index: u8, call_index: u8) -> Result<ExtrinsicInfo<Self::TypeId>, ExtrinsicInfoError<'_>> {
                impl_extrinsic_info_body_for_v8_to_v11!(self, pallet_index, call_index)
            }
            fn get_signature_info(&self) -> Result<ExtrinsicSignatureInfo<Self::TypeId>, ExtrinsicInfoError<'_>> {
                Ok(ExtrinsicSignatureInfo {
                    address_id: parse_lookup_name("hardcoded::ExtrinsicAddress")?,
                    signature_id: parse_lookup_name("hardcoded::ExtrinsicSignature")?,
                    signed_extension_ids: vec![
                        ExtrinsicInfoArg {
                            name: Cow::Borrowed("ExtrinsicSignedExtensions"),
                            id: parse_lookup_name("hardcoded::ExtrinsicSignedExtensions")?
                        }
                    ]
                })
            }
        }
    }
}

impl_for_v8_to_v10!(frame_metadata::v8::RuntimeMetadataV8);
impl_for_v8_to_v10!(frame_metadata::v9::RuntimeMetadataV9);
impl_for_v8_to_v10!(frame_metadata::v10::RuntimeMetadataV10);

impl ExtrinsicTypeInfo for frame_metadata::v11::RuntimeMetadataV11 {
    type TypeId = LookupName;
    fn get_extrinsic_info(&self, pallet_index: u8, call_index: u8) -> Result<ExtrinsicInfo<Self::TypeId>, ExtrinsicInfoError<'_>> {
        impl_extrinsic_info_body_for_v8_to_v11!(self, pallet_index, call_index)

    }
    fn get_signature_info(&self) -> Result<ExtrinsicSignatureInfo<Self::TypeId>, ExtrinsicInfoError<'_>> {
        // In V11 metadata we start exposing signed extension names, so we use those directly instead of
        // a hardcoded ExtrinsicSignedExtensions type that the user is expected to define.
        let signed_extension_ids = self.extrinsic.signed_extensions.iter().map(|e| {
            let signed_ext_name = as_decoded(e);
            let signed_ext_id = parse_lookup_name(signed_ext_name)?;

            Ok(ExtrinsicInfoArg { id: signed_ext_id, name: Cow::Borrowed(signed_ext_name) })
        }).collect::<Result<Vec<_>,ExtrinsicInfoError<'_>>>()?;

        Ok(ExtrinsicSignatureInfo {
            address_id: parse_lookup_name("hardcoded::ExtrinsicAddress")?,
            signature_id: parse_lookup_name("hardcoded::ExtrinsicSignature")?,
            signed_extension_ids
        })
    }
}

macro_rules! impl_for_v12_to_v13 {
    ($path:path) => {
        impl ExtrinsicTypeInfo for $path {
            type TypeId = LookupName;
            fn get_extrinsic_info(&self, pallet_index: u8, call_index: u8) -> Result<ExtrinsicInfo<Self::TypeId>, ExtrinsicInfoError<'_>> {
                let modules = as_decoded(&self.modules);

                let m = modules
                    .iter()
                    .find(|m| m.index == pallet_index)
                    .ok_or_else(|| ExtrinsicInfoError::PalletNotFound { index: pallet_index })?;
        
                let m_name = as_decoded(&m.name);
        
                let calls = m.calls
                    .as_ref()
                    .ok_or_else(|| ExtrinsicInfoError::CallNotFound { 
                        index: call_index, 
                        pallet_index: pallet_index, 
                        pallet_name: Cow::Borrowed(m_name),
                    })?;
        
                let calls = as_decoded(calls);
        
                let call = calls
                    .get(call_index as usize)
                    .ok_or_else(|| ExtrinsicInfoError::CallNotFound { 
                        index: call_index, 
                        pallet_index: pallet_index, 
                        pallet_name: Cow::Borrowed(m_name),
                    })?;

                let c_name = as_decoded(&call.name);

                let args = as_decoded(&call.arguments);

                let args = args.iter().map(|a| {
                    let ty = as_decoded(&a.ty);
                    let id = parse_lookup_name(ty)?.in_pallet(m_name);
                    let name = as_decoded(&a.name);
                    Ok(ExtrinsicInfoArg { id, name: Cow::Borrowed(name) })
                }).collect::<Result<_, ExtrinsicInfoError>>()?;

                Ok(ExtrinsicInfo {
                    pallet_name: Cow::Borrowed(m_name),
                    call_name: Cow::Borrowed(c_name),
                    args
                })
            }
            fn get_signature_info(&self) -> Result<ExtrinsicSignatureInfo<Self::TypeId>, ExtrinsicInfoError<'_>> {
                // In V12 metadata we are exposing signed extension names, so we use those directly instead of
                // a hardcoded ExtrinsicSignedExtensions type that the user is expected to define.
                let signed_extension_ids = self.extrinsic.signed_extensions.iter().map(|e| {
                    let signed_ext_name = as_decoded(e);
                    let signed_ext_id = parse_lookup_name(signed_ext_name)?;

                    Ok(ExtrinsicInfoArg { id: signed_ext_id, name: Cow::Borrowed(signed_ext_name) })
                }).collect::<Result<Vec<_>,_>>()?;

                Ok(ExtrinsicSignatureInfo {
                    address_id: parse_lookup_name("hardcoded::ExtrinsicAddress")?,
                    signature_id: parse_lookup_name("hardcoded::ExtrinsicSignature")?,
                    signed_extension_ids
                })
            }
        }
    }
}

impl_for_v12_to_v13!(frame_metadata::v12::RuntimeMetadataV12);
impl_for_v12_to_v13!(frame_metadata::v13::RuntimeMetadataV13);

macro_rules! impl_call_arg_ids_body_for_v14_to_v15 {
    ($self:ident, $pallet_index:ident, $call_index:ident) => {{
        let pallet = $self.pallets
            .iter()
            .find(|p| p.index == $pallet_index)
            .ok_or_else(|| ExtrinsicInfoError::PalletNotFound { index: $pallet_index })?;

        let pallet_name = &pallet.name;

        let calls_id = pallet.calls.as_ref()
            .ok_or_else(|| ExtrinsicInfoError::CallNotFound { 
                index: $call_index, 
                pallet_index: $pallet_index, 
                pallet_name: Cow::Borrowed(pallet_name),
            })?
            .ty.id;

        let calls_ty = $self.types.resolve(calls_id)
            .ok_or_else(|| ExtrinsicInfoError::CallsTypeNotFound {
                id: calls_id,
                pallet_index: $pallet_index,
                pallet_name: Cow::Borrowed(pallet_name)
            })?;

        let calls_enum = match &calls_ty.type_def {
            scale_info::TypeDef::Variant(v) => v,
            _ => return Err(ExtrinsicInfoError::CallsTypeShouldBeVariant {
                id: calls_id,
                pallet_index: $pallet_index,
                pallet_name: Cow::Borrowed(pallet_name)
            })
        };

        let call_variant = calls_enum.variants
            .iter()
            .find(|v| v.index == $call_index)
            .ok_or_else(|| ExtrinsicInfoError::CallNotFound { 
                index: $call_index, 
                pallet_index: $pallet_index, 
                pallet_name: Cow::Borrowed(pallet_name),
            })?;

        let args = call_variant
            .fields
            .iter()
            .map(|f| {
                let id = f.ty.id;
                let name = f.name.as_ref()
                    .map(|n| Cow::Borrowed(&**n))
                    .unwrap_or(Cow::Owned(String::new()));
                ExtrinsicInfoArg { id, name }
            })
            .collect();

        Ok(ExtrinsicInfo {
            pallet_name: Cow::Borrowed(pallet_name),
            call_name: Cow::Borrowed(&call_variant.name),
            args,
        })
    }}
}

impl ExtrinsicTypeInfo for frame_metadata::v14::RuntimeMetadataV14 {
    type TypeId = u32;
    fn get_extrinsic_info(&self, pallet_index: u8, call_index: u8) -> Result<ExtrinsicInfo<Self::TypeId>, ExtrinsicInfoError<'_>> {
        impl_call_arg_ids_body_for_v14_to_v15!(self, pallet_index, call_index)
    }
    fn get_signature_info(&self) -> Result<ExtrinsicSignatureInfo<Self::TypeId>, ExtrinsicInfoError<'_>> {
        let signed_extension_ids = self.extrinsic.signed_extensions.iter().map(|e| {
            ExtrinsicInfoArg { id: e.ty.id, name: Cow::Borrowed(&e.identifier) }
        }).collect();

        let extrincis_parts = get_v14_extrinsic_parts(self)?;

        Ok(ExtrinsicSignatureInfo {
            address_id: extrincis_parts.address,
            signature_id: extrincis_parts.signature,
            signed_extension_ids
        })
    }
}

impl ExtrinsicTypeInfo for frame_metadata::v15::RuntimeMetadataV15 {
    type TypeId = u32;
    fn get_extrinsic_info(&self, pallet_index: u8, call_index: u8) -> Result<ExtrinsicInfo<Self::TypeId>, ExtrinsicInfoError<'_>> {
        impl_call_arg_ids_body_for_v14_to_v15!(self, pallet_index, call_index)
    }
    fn get_signature_info(&self) -> Result<ExtrinsicSignatureInfo<Self::TypeId>, ExtrinsicInfoError<'_>> {
        let signed_extension_ids = self.extrinsic.signed_extensions.iter().map(|e| {
            ExtrinsicInfoArg { id: e.ty.id, name: Cow::Borrowed(&e.identifier) }
        }).collect();

        Ok(ExtrinsicSignatureInfo {
            address_id: self.extrinsic.address_ty.id,
            signature_id: self.extrinsic.signature_ty.id,
            signed_extension_ids
        })
    }
}

fn get_v14_extrinsic_parts(metadata: &frame_metadata::v14::RuntimeMetadataV14) -> Result<ExtrinsicParts, ExtrinsicInfoError<'_>> {
    const ADDRESS: &str = "Address";
    const SIGNATURE: &str = "Signature";

    let extrinsic_id = metadata.extrinsic.ty.id;
    let extrinsic_ty = metadata.types.resolve(extrinsic_id)
        .ok_or_else(|| ExtrinsicInfoError::ExtrinsicTypeNotFound { id: extrinsic_id })?;

    let address_ty = extrinsic_ty
        .type_params
        .iter()
        .find_map(|param| if param.name == ADDRESS { param.ty } else { None })
        .ok_or_else(|| ExtrinsicInfoError::ExtrinsicAddressTypeNotFound)?;

    let signature_ty = extrinsic_ty
        .type_params
        .iter()
        .find_map(|param| if param.name == SIGNATURE { param.ty } else { None })
        .ok_or_else(|| ExtrinsicInfoError::ExtrinsicSignatureTypeNotFound)?;

    Ok(ExtrinsicParts {
        address: address_ty.id,
        signature: signature_ty.id,
    })
}

struct ExtrinsicParts {
    address: u32,
    signature: u32,
}

fn parse_lookup_name(name: &str) -> Result<LookupName, ExtrinsicInfoError<'_>> {
    LookupName::parse(name).map_err(|e| ExtrinsicInfoError::CannotParseTypeName {
        name: Cow::Borrowed(name),
        reason: e
    })
}