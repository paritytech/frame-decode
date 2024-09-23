use alloc::format;
use alloc::string::String;

/// A utility function to unwrap the [`DecodeDifferent`] enum found in earlier metadata versions.
#[cfg(feature = "legacy")]
pub fn as_decoded<A, B>(item: &frame_metadata::decode_different::DecodeDifferent<A, B>) -> &B {
    match item {
        frame_metadata::decode_different::DecodeDifferent::Encode(_a) => panic!("Expecting decoded data"),
        frame_metadata::decode_different::DecodeDifferent::Decoded(b) => b,
    }
}

/// Decode some bytes, skipping over them. If decode fails, we try to decode again using
/// a tracing visitor in order to return a more detailed error message.
pub fn decode_with_error_tracing<'scale, 'resolver, Resolver, Id, V>(
    cursor: &mut &'scale [u8], 
    type_id: Id, 
    types: &'resolver Resolver, 
    visitor: V
) -> Result<V::Value<'scale, 'resolver>, DecodeErrorTrace>
where
    Resolver: scale_type_resolver::TypeResolver<TypeId = Id>,
    Id: core::fmt::Debug + Clone,
    V: scale_decode::Visitor<TypeResolver = Resolver>,
    V::Error: core::fmt::Debug
{
    let initial = *cursor;
    match scale_decode::visitor::decode_with_visitor(cursor, type_id.clone(), types, visitor) {
        Ok(value) => Ok(value),
        // Don't use scale-value; return the error as-is.
        #[cfg(not(feature = "error-tracing"))]
        Err(e) => {
            *cursor = initial;

            Err(DecodeErrorTrace {
                original_error: format!("{e:?}"),
                tracing_error: String::new()
            })
        }
        // Use scale-value tracing visitor to return a better error
        #[cfg(feature = "error-tracing")]
        Err(e) => {
            // Reset cursor incase it's been consumed by the above call, and decode using the
            // tracing visitor to hopefully return a better error.
            *cursor = initial;
            let res = scale_value::scale::tracing::decode_as_type(cursor, type_id.clone(), types)
                .map(|v| v.map_context(|id| format!("{id:?}")))
                .map_err(|te| DecodeErrorTrace { 
                    original_error: format!("{e:?}"),
                    tracing_error: te.to_string()
                })?;

            // If the above succeeds (we're expecting it to fail), then print the value out here.
            use core::fmt::Write;
            let mut res_string = String::new();
            write!(&mut res_string, "Failed to decode value with custom visitor (but tracing decoded it):\n\n").unwrap();
            scale_value::stringify::to_writer_custom()
                .pretty()
                .format_context(|type_id, w: &mut &mut String| write!(w, "{type_id}"))
                .add_custom_formatter(|v, w| scale_value::stringify::custom_formatters::format_hex(v,w))
                .add_custom_formatter(|v, w| {
                    // don't space unnamed composites over multiple lines if lots of primitive values.
                    if let scale_value::ValueDef::Composite(scale_value::Composite::Unnamed(vals)) = &v.value {
                        let are_primitive = vals.iter().all(|val| matches!(val.value, scale_value::ValueDef::Primitive(_)));
                        if are_primitive {
                            return Some(write!(w, "{v}"))
                        }
                    }
                    None
                })
                .write(&res, &mut res_string)
                .expect("writing to string should always succeed");

            Err(DecodeErrorTrace {
                original_error: format!("{e:?}"),
                tracing_error: res_string
            })
        }
    }
}

#[derive(Clone, Debug)]
pub struct DecodeErrorTrace {
    pub original_error: String,
    pub tracing_error: String,
}

pub trait InfoAndResolver {
    type Info;
    type Resolver;

    fn info(&self) -> &Self::Info;
    fn resolver(&self) -> &Self::Resolver;
}

impl InfoAndResolver for frame_metadata::v14::RuntimeMetadataV14 {
    type Info = scale_info::PortableRegistry;
    type Resolver = frame_metadata::v14::RuntimeMetadataV14;

    fn info(&self) -> &Self::Info { &self.types }
    fn resolver(&self) -> &Self::Resolver { self }
}

impl InfoAndResolver for frame_metadata::v15::RuntimeMetadataV15 {
    type Info = scale_info::PortableRegistry;
    type Resolver = frame_metadata::v15::RuntimeMetadataV15;

    fn info(&self) -> &Self::Info { &self.types }
    fn resolver(&self) -> &Self::Resolver { self }
}