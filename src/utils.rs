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

impl core::error::Error for DecodeErrorTrace {}

impl core::fmt::Display for DecodeErrorTrace {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let DecodeErrorTrace { original_error, tracing_error } = self;

        write!(f, "{original_error}")?;
        if !tracing_error.is_empty() {
            write!(f, ":\n\n{tracing_error}")?;
        }
        Ok(())
    }
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

/// [`frame_metadata::RuntimeMetadata`] contains information about runtime calls and events. This
/// function adds this information into a [`scale_info_legacy::TypeRegistry`] which can then be referenced
/// by other types. The main types you'll be able to reference from this set are:
/// 
/// - `builtin::Call` - A variant which describes the shape of a `RuntimeCall`, ie any call.
/// - `builtin::Event` - A variant which describes the shape of a `RuntimeEvent`, ie any event.
/// 
/// These are composed from:
/// 
/// - `builtin::module::event::$PALLET` - A variant containing the events in a specific pallet.
/// - `builtin::module::call::$PALLET` - A variant containing the calls in a specific pallet.
#[cfg(feature = "legacy")]
pub fn type_registry_from_metadata(metadata: &frame_metadata::RuntimeMetadata) -> Result<scale_info_legacy::TypeRegistry, scale_info_legacy::lookup_name::ParseError> {
    use scale_info_legacy::{TypeRegistry, LookupName};
    use scale_info_legacy::type_shape::{TypeShape,Field,Variant,VariantDesc};
    use scale_info_legacy::InsertName;
    
    macro_rules! impl_for_v8_to_v13 {
        ($new_types:ident, $metadata:ident $(, $builtin_index:ident)?) => {{
            let modules = as_decoded(&$metadata.modules);

            let mut call_module_variants: Vec<Variant> = vec![];
            let mut event_module_variants: Vec<Variant> = vec![];

            let mut calls_index = 0u8;
            let mut events_index = 0u8;

            for module in modules {

                // In older metadatas, calls and event enums can have different indexes
                // in a given pallet. Pallets without calls or events don't increment
                // the respective index for them.
                let (calls_index, events_index) = {
                    let out = (calls_index, events_index);
                    if module.calls.is_some() {
                        calls_index += 1;
                    }
                    if module.event.is_some() {
                        events_index += 1;
                    }
                    out
                };

                // For v12 and v13 metadata, there is a builtin index for everything in a pallet.
                // If we pass an ident as second arg to this macro, we'll trigger
                // using this builtin index instead.
                $(
                    let $builtin_index = true;
                    let (calls_index, events_index) = if $builtin_index {
                        (module.index, module.index)
                    } else {
                        (calls_index, events_index)
                    };
                )?

                let module_name = as_decoded(&module.name);

                //// 1. Add calls to the type registry
                if let Some(calls) = &module.calls.as_ref() {
                    let calls = as_decoded(calls);

                    // Iterate over each call in the module and turn into variants:
                    let mut call_variants: Vec<Variant> = vec![];
                    for (c_idx, call) in calls.iter().enumerate() {
                        let call_name = as_decoded(&call.name);
                        let args = as_decoded(&call.arguments)
                            .iter()
                            .map(|arg| {
                                Ok(Field {
                                    name: as_decoded(&arg.name).to_owned(),
                                    value: LookupName::parse(&as_decoded(&arg.ty))?.in_pallet(module_name),
                                })
                            })
                            .collect::<Result<_,_>>()?;

                        call_variants.push(Variant {
                            index: c_idx as u8,
                            name: call_name.clone(),
                            fields: VariantDesc::StructOf(args)
                        });
                    }

                    // Store these call variants in the types:
                    let call_enum_name_str = format!("builtin::module::call::{module_name}");
                    let call_enum_insert_name = InsertName::parse(&call_enum_name_str).unwrap();
                    $new_types.insert(call_enum_insert_name, TypeShape::EnumOf(call_variants));

                    // Reference it in the modules enum we're building:
                    let call_enum_lookup_name = LookupName::parse(&call_enum_name_str).unwrap();
                    call_module_variants.push(Variant {
                        index: calls_index,
                        name: module_name.clone(),
                        fields: VariantDesc::TupleOf(vec![call_enum_lookup_name])
                    });
                }

                //// 2. Add events to the type registry
                if let Some(events) = &module.event.as_ref() {
                    let events = as_decoded(events);

                    let mut event_variants: Vec<Variant> = vec![];
                    for (e_idx, event)in events.iter().enumerate() {
                        let event_name = as_decoded(&event.name);
                        let args = as_decoded(&event.arguments)
                            .iter()
                            .map(|arg| {
                                Ok(LookupName::parse(&arg)?.in_pallet(module_name))
                            })
                            .collect::<Result<_,_>>()?;

                        event_variants.push(Variant {
                            index: e_idx as u8,
                            name: event_name.clone(),
                            fields: VariantDesc::TupleOf(args)
                        });
                    }

                    // Store event variants in the types:
                    let event_enum_name_str = format!("builtin::module::event::{module_name}");
                    let event_enum_insert_name = InsertName::parse(&event_enum_name_str).unwrap();
                    $new_types.insert(event_enum_insert_name, TypeShape::EnumOf(event_variants));

                    // Reference it in the modules enum we're building:
                    let event_enum_lookup_name = LookupName::parse(&event_enum_name_str).unwrap();
                    event_module_variants.push(Variant {
                        index: events_index,
                        name: module_name.clone(),
                        fields: VariantDesc::TupleOf(vec![event_enum_lookup_name])
                    });
                }
            }

            // Store the module call variants in the types:
            let calls_enum_name_str = "builtin::Call";
            let calls_enum_insert_name = InsertName::parse(&calls_enum_name_str).unwrap();
            $new_types.insert(calls_enum_insert_name, TypeShape::EnumOf(call_module_variants));

            // Store the module event variants in the types:
            let events_enum_name_str = "builtin::Event";
            let events_enum_insert_name = InsertName::parse(&events_enum_name_str).unwrap();
            $new_types.insert(events_enum_insert_name, TypeShape::EnumOf(event_module_variants));
        }}
    }

    let mut new_types = TypeRegistry::empty();

    match metadata {
        frame_metadata::RuntimeMetadata::V8(m) => impl_for_v8_to_v13!(new_types, m),
        frame_metadata::RuntimeMetadata::V9(m) => impl_for_v8_to_v13!(new_types, m),
        frame_metadata::RuntimeMetadata::V10(m) => impl_for_v8_to_v13!(new_types, m),
        frame_metadata::RuntimeMetadata::V11(m) => impl_for_v8_to_v13!(new_types, m),
        frame_metadata::RuntimeMetadata::V12(m) => impl_for_v8_to_v13!(new_types, m, use_builtin_index),
        frame_metadata::RuntimeMetadata::V13(m) => impl_for_v8_to_v13!(new_types, m, use_builtin_index),
        _ => {/* do nothing if metadata too old or new */}
    };

    Ok(new_types)
}