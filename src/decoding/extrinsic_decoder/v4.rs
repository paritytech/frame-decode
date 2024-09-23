use alloc::borrow::Cow;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::vec;
use crate::decoding::extrinsic_type_info::ExtrinsicTypeInfo;
use crate::decoding::extrinsic_type_info::ExtrinsicInfoError;
use crate::utils::{ decode_with_error_tracing, DecodeErrorTrace };
use scale_type_resolver::TypeResolver;
use parity_scale_codec::Decode;
use core::ops::Range;

/// An error decoding a version 4 extrinsic.
#[derive(Debug, Clone)]
#[non_exhaustive]
#[allow(missing_docs)]
pub enum ExtrinsicDecodeError {
    CannotGetInfo(ExtrinsicInfoError<'static>),
    CannotDecodeSignature(DecodeErrorTrace),
    CannotDecodePalletIndex(parity_scale_codec::Error),
    CannotDecodeCallIndex(parity_scale_codec::Error),
    CannotDecodeCallData {
        pallet_name: String,
        call_name: String,
        argument_name: String,
        reason: DecodeErrorTrace,
    },
}

impl core::error::Error for ExtrinsicDecodeError {}

impl core::fmt::Display for ExtrinsicDecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ExtrinsicDecodeError::CannotGetInfo(extrinsic_info_error) => {
                write!(f, "Cannot get extrinsic info:\n\n{extrinsic_info_error}")
            },
            ExtrinsicDecodeError::CannotDecodeSignature(decode_error_trace) => {
                write!(f, "Cannot decode signature:\n\n{decode_error_trace}")
            },
            ExtrinsicDecodeError::CannotDecodePalletIndex(error) => {
                write!(f, "Cannot decode pallet index:\n\n{error}")
            },
            ExtrinsicDecodeError::CannotDecodeCallIndex(error) => {
                write!(f, "Cannot decode call index:\n\n{error}")
            },
            ExtrinsicDecodeError::CannotDecodeCallData { pallet_name, call_name, argument_name, reason } => {
                write!(f, "Cannot decode call data for argument {argument_name} in {pallet_name}.{call_name}:\n\n{reason}")
            },
        }
    }
}

/// An owned variant of an Extrinsic (note: this may still contain
/// references if the visitor used to decode the extrinsic contents holds
/// onto any)
pub type ExtrinsicOwned<TypeId> = Extrinsic<'static, TypeId>;

/// Information about a version 4 extrinsic.
#[derive(Clone, Debug)]
pub struct Extrinsic<'info, TypeId> {
    byte_len: u32,
    signature: Option<ExtrinsicSignature<'info, TypeId>>,
    call_name: Cow<'info, str>,
    pallet_name: Cow<'info, str>,
    call_data: Vec<NamedArg<'info, TypeId>>
}

impl <'info, TypeId> Extrinsic<'info, TypeId> {
    /// Take ownership of the extrinsic, so that it no longer references
    /// the extrinsic info or bytes.
    pub fn into_owned(self) -> ExtrinsicOwned<TypeId> {
        Extrinsic {
            byte_len: self.byte_len,
            signature: self.signature.map(|s| s.into_owned()),
            call_name: Cow::Owned(self.call_name.into_owned()),
            pallet_name: Cow::Owned(self.pallet_name.into_owned()),
            call_data: self.call_data
                .into_iter()
                .map(|e| e.into_owned())
                .collect()
        }
    }

    /// Does the extrinsic have a signature.
    pub fn is_signed(&self) -> bool {
        self.signature.is_some()
    }

    /// Return the extrinsic signature payload, if present.
    pub fn signature_payload(&self) -> Option<&ExtrinsicSignature<'info, TypeId>> {
        self.signature.as_ref()
    }

    /// Return a range denoting the signature payload bytes.
    pub fn signature_payload_range(&self) -> Range<usize> {
        self.signature.as_ref().map(|s| {
            Range { start: 1, end: s.signed_exts_end_idx() }
        }).unwrap_or(Range { start: 1, end: 1 })
    }

    /// Iterate over the call data argument names and types.
    pub fn call_data(&self) -> impl Iterator<Item=&NamedArg<'info, TypeId>> {
        self.call_data.iter()
    }

    /// Return a range denoting the call data bytes.
    pub fn call_data_range(&self) -> Range<usize> {
        self.signature.as_ref().map(|s| {
            Range { 
                start: s.signed_exts_end_idx(), 
                end: self.byte_len as usize 
            }
        }).unwrap_or(Range { start: 1, end: self.byte_len as usize })
    }

    /// Map the signature type IDs to something else.
    pub fn map_type_id<NewTypeId, F>(self, mut f: F) -> Extrinsic<'info, NewTypeId> 
    where
        F: FnMut(TypeId) -> NewTypeId        
    {
        Extrinsic {
            byte_len: self.byte_len,
            signature: self.signature.map(|s| s.map_type_id(&mut f)),
            call_name: self.call_name,
            pallet_name: self.pallet_name,
            call_data: self.call_data
                .into_iter()
                .map(|s| s.map_type_id(&mut f))
                .collect()
        }
    }
}

/// Information about the extrinsic signature.
#[derive(Clone, Debug)]
pub struct ExtrinsicSignature<'info, TypeId> {
    // Store byte offsets so people can ask for raw 
    // bytes to do their own thing.
    address_start_idx: u32,
    address_end_idx: u32,
    signature_end_idx: u32,
    // Also decode and store actual values. We could
    // do this more "on demand" but it complicates
    // everything. Ultimately just a couple of vec allocs
    // that we could perhaps optimise into SmallVecs or
    // something if desired.
    address_ty: TypeId,
    signature_ty: TypeId,
    signed_extensions: Vec<NamedArg<'info, TypeId>>,
}

impl <'info, TypeId> ExtrinsicSignature<'info, TypeId> {
    /// Take ownership of the signature.
    pub fn into_owned(self) -> ExtrinsicSignature<'static, TypeId> {
        ExtrinsicSignature {
            address_start_idx: self.address_start_idx,
            address_end_idx: self.address_end_idx,
            signature_end_idx: self.signature_end_idx,
            address_ty: self.address_ty,
            signature_ty: self.signature_ty,
            signed_extensions: self.signed_extensions
                .into_iter()
                .map(|e| e.into_owned())
                .collect()
        }
    }

    /// Return a range denoting the address bytes.
    pub fn address_range(&self) -> Range<usize> {
        Range { start: self.address_start_idx as usize, end: self.address_end_idx as usize }
    }

    /// The decoded address.
    pub fn address_type(&self) -> &TypeId {
        &self.address_ty
    }

    /// Return a range denoting the signature bytes.
    pub fn signature_range(&self) -> Range<usize> {
        Range { start: self.address_end_idx as usize, end: self.signature_end_idx as usize }
    }

    /// The decoded signature.
    pub fn signature_type(&self) -> &TypeId {
        &self.signature_ty
    }

    /// Iterate over the signed extension argument names and types.
    pub fn signed_extensions(&self) -> impl Iterator<Item=&NamedArg<'info, TypeId>> {
        self.signed_extensions.iter()
    }

    /// Map the signature type IDs to something else.
    pub fn map_type_id<NewTypeId, F>(self, mut f: F) -> ExtrinsicSignature<'info, NewTypeId> 
    where
        F: FnMut(TypeId) -> NewTypeId
    {
        ExtrinsicSignature { 
            address_start_idx: self.address_start_idx, 
            address_end_idx: self.address_end_idx, 
            signature_end_idx: self.signature_end_idx, 
            address_ty: f(self.address_ty), 
            signature_ty: f(self.signature_ty), 
            signed_extensions: self.signed_extensions
                .into_iter()
                .map(|s| s.map_type_id(&mut f))
                .collect()
        }
    }

    fn signed_exts_end_idx(&self) -> usize {
        self.signed_extensions
            .last()
            .map(|e| e.range.end)
            .unwrap_or(self.signature_end_idx) as usize
    }
}

/// A single named argument.
#[derive(Clone, Debug)]
pub struct NamedArg<'info, TypeId> {
    name: Cow<'info, str>,
    range: Range<u32>,
    ty: TypeId
}

impl <'info, TypeId> NamedArg<'info, TypeId> {
    /// Map the type ID to something else.
    pub fn map_type_id<NewTypeId, F>(self, mut f: F) -> NamedArg<'info, NewTypeId> 
    where
        F: FnMut(TypeId) -> NewTypeId 
    {
        NamedArg { 
            name: self.name, 
            range: self.range, 
            ty: f(self.ty)
        }
    }
}

impl <'info, TypeId> NamedArg<'info, TypeId> {
    /// Take ownership of this named argument.
    pub fn into_owned(self) -> NamedArg<'static, TypeId> {
        NamedArg {
            name: Cow::Owned(self.name.into_owned()),
            range: self.range,
            ty: self.ty
        }
    }

    /// The name of this argument.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return a range denoting the bytes associated with this argument value.
    pub fn range(&self) -> Range<usize> {
        Range { 
            start: self.range.start as usize,
            end: self.range.end as usize
        }
    }

    /// The type ID associated with this argument value.
    pub fn ty(&self) -> &TypeId {
        &self.ty
    }
}

pub fn decode_extrinsic<'scale, 'info, 'resolver, Info, Resolver>(
    offset: usize,
    cursor: &mut &'scale [u8], 
    info: &'info Info, 
    type_resolver: &'resolver Resolver,
) -> Result<Extrinsic<'info, Info::TypeId>, ExtrinsicDecodeError> 
where
    Info: ExtrinsicTypeInfo,
    Info::TypeId: Clone + core::fmt::Debug,
    Resolver: TypeResolver<TypeId = Info::TypeId>,
{
    let bytes = *cursor;
    let curr_idx = |cursor: &mut &[u8]| (bytes.len() - cursor.len() + offset) as u32;
    let is_signed = bytes[0] & 0b1000_0000 != 0;
    *cursor = &cursor[1..];

    // Signature part
    let signature = is_signed.then(|| {
        let signature_info = info
            .get_signature_info()
            .map_err(|e| ExtrinsicDecodeError::CannotGetInfo(e.into_owned()))?;

        let address_start_idx = curr_idx(cursor);
        decode_with_error_tracing(
            cursor,
            signature_info.address_id.clone(),
            type_resolver,
            scale_decode::visitor::IgnoreVisitor::new()
        ).map_err(|e| ExtrinsicDecodeError::CannotDecodeSignature(e.into()))?;
        let address_end_idx = curr_idx(cursor);

        decode_with_error_tracing(
            cursor,
            signature_info.signature_id.clone(),
            type_resolver,
            scale_decode::visitor::IgnoreVisitor::new(),
        ).map_err(|e| ExtrinsicDecodeError::CannotDecodeSignature(e.into()))?;
        let signature_end_idx = curr_idx(cursor);

        let mut signed_extensions = vec![];
        for ext in signature_info.signed_extension_ids {
            let start_idx = curr_idx(cursor);
            decode_with_error_tracing(
                cursor,
                ext.id.clone(),
                type_resolver,
                scale_decode::visitor::IgnoreVisitor::new(),
            ).map_err(|e| ExtrinsicDecodeError::CannotDecodeSignature(e.into()))?;
            let end_idx = curr_idx(cursor);

            signed_extensions.push(NamedArg {
                name: ext.name,
                range: Range { start: start_idx, end: end_idx },
                ty: ext.id
            });
        }

        Ok::<_, ExtrinsicDecodeError>(ExtrinsicSignature {
            address_start_idx,
            address_end_idx,
            signature_end_idx,
            address_ty: signature_info.address_id,
            signature_ty: signature_info.signature_id,
            signed_extensions
        })
    }).transpose()?;

    // Call data part
    let pallet_index: u8 = Decode::decode(cursor)
        .map_err(|e| ExtrinsicDecodeError::CannotDecodePalletIndex(e.into()))?;
    let call_index: u8 = Decode::decode(cursor)
        .map_err(|e| ExtrinsicDecodeError::CannotDecodeCallIndex(e.into()))?;
    let extrinsic_info = info.get_extrinsic_info(pallet_index, call_index)
        .map_err(|e| ExtrinsicDecodeError::CannotGetInfo(e.into_owned()))?;

    let mut call_data = vec![];
    for arg in extrinsic_info.args {
        let start_idx = curr_idx(cursor);
        decode_with_error_tracing(
            cursor,
            arg.id.clone(),
            type_resolver,
            scale_decode::visitor::IgnoreVisitor::new(),
        ).map_err(|e| ExtrinsicDecodeError::CannotDecodeCallData {
            pallet_name: extrinsic_info.pallet_name.to_string(),
            call_name: extrinsic_info.call_name.to_string(),
            argument_name: arg.name.to_string(),
            reason: e.into()
        })?;
        let end_idx = curr_idx(cursor);

        call_data.push(NamedArg { name: arg.name, range: Range { start: start_idx, end: end_idx }, ty: arg.id })
    }

    let ext = Extrinsic { 
        byte_len: bytes.len() as u32,
        signature,
        call_name: extrinsic_info.call_name,
        pallet_name: extrinsic_info.pallet_name,
        call_data
    };

    Ok(ext)
}
