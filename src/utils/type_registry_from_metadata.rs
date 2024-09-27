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
pub fn type_registry_from_metadata<Md: ToTypeRegistry>(
    metadata: &Md,
) -> Result<scale_info_legacy::TypeRegistry, scale_info_legacy::lookup_name::ParseError> {
    metadata.to_type_registry()
}

#[cfg(feature = "legacy")]
pub trait ToTypeRegistry {
    fn to_type_registry(
        &self,
    ) -> Result<scale_info_legacy::TypeRegistry, scale_info_legacy::lookup_name::ParseError>;
}

#[cfg(feature = "legacy")]
const _: () = {
    macro_rules! impl_for_v8_to_v13 {
        ($path:path $(, $builtin_index:ident)?) => {
            impl ToTypeRegistry for $path {
                fn to_type_registry(&self) -> Result<scale_info_legacy::TypeRegistry, scale_info_legacy::lookup_name::ParseError> {
                    use super::as_decoded;
                    use scale_info_legacy::type_shape::{Field, TypeShape, Variant, VariantDesc};
                    use scale_info_legacy::InsertName;
                    use scale_info_legacy::{LookupName, TypeRegistry};

                    let metadata = self;
                    let mut new_types = TypeRegistry::empty();
                    let modules = as_decoded(&metadata.modules);

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
                            new_types.insert(call_enum_insert_name, TypeShape::EnumOf(call_variants));

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
                            new_types.insert(event_enum_insert_name, TypeShape::EnumOf(event_variants));

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
                    new_types.insert(calls_enum_insert_name, TypeShape::EnumOf(call_module_variants));

                    // Store the module event variants in the types:
                    let events_enum_name_str = "builtin::Event";
                    let events_enum_insert_name = InsertName::parse(&events_enum_name_str).unwrap();
                    new_types.insert(events_enum_insert_name, TypeShape::EnumOf(event_module_variants));

                    Ok(new_types)
                }
            }
        }
    }

    impl_for_v8_to_v13!(frame_metadata::v8::RuntimeMetadataV8);
    impl_for_v8_to_v13!(frame_metadata::v9::RuntimeMetadataV9);
    impl_for_v8_to_v13!(frame_metadata::v10::RuntimeMetadataV10);
    impl_for_v8_to_v13!(frame_metadata::v11::RuntimeMetadataV11);
    impl_for_v8_to_v13!(frame_metadata::v12::RuntimeMetadataV12, use_builtin_index);
    impl_for_v8_to_v13!(frame_metadata::v13::RuntimeMetadataV13, use_builtin_index);
};
