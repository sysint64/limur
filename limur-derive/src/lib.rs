use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, LitStr, parse_macro_input};

#[proc_macro_derive(Identifiable, attributes(id))]
pub fn derive_identifiable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => panic!("Identifiable only supports structs with named fields"),
        },
        _ => panic!("Identifiable only supports structs"),
    };

    // First try to find field with #[id] attribute
    let id_field = fields
        .iter()
        .find(|f| f.attrs.iter().any(|attr| attr.path().is_ident("id")));

    // If not found, try to find field named "id"
    let id_field = id_field.or_else(|| {
        fields
            .iter()
            .find(|f| f.ident.as_ref().map(|i| i == "id").unwrap_or(false))
    });

    let id_field = id_field.expect("No field marked with #[id] or named 'id' found");

    let id_field_name = id_field.ident.as_ref().unwrap();
    let id_field_type = &id_field.ty;

    let expanded = quote! {
        #[allow(clippy::misnamed_getters)]
        impl #impl_generics ::limur::prelude::Identifiable for #name #ty_generics #where_clause {
            type Id = #id_field_type;

            fn id(&self) -> Self::Id {
                self.#id_field_name
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_derive(WidgetState)]
pub fn derive_widget_state(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let expanded = quote! {
        impl #impl_generics ::limur::prelude::WidgetState for #name #ty_generics #where_clause {
            #[inline]
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }

            #[inline]
            fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
                self
            }

            #[inline]
            fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
                self
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_derive(WidgetBuilder)]
pub fn widget_builder_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let expanded = quote! {
        impl #impl_generics ::limur::prelude::WidgetBuilder for #name #ty_generics #where_clause {
            fn frame_mut(&mut self) -> &mut ::limur::FrameBuilder {
                &mut self.frame
            }
        }
    };

    TokenStream::from(expanded)
}

/// Derive macro for ShortcutScopeId
#[proc_macro_derive(ShortcutScopeId, attributes(scope_prefix))]
pub fn derive_shortcut_scope_id(input: TokenStream) -> TokenStream {
    derive_id_impl(input, "ShortcutScopeId")
}

/// Derive macro for ShortcutModifierId
#[proc_macro_derive(ShortcutModifierId, attributes(modifier_prefix))]
pub fn derive_shortcut_modifier_id(input: TokenStream) -> TokenStream {
    derive_id_impl(input, "ShortcutModifierId")
}

/// Derive macro for ShortcutId
#[proc_macro_derive(ShortcutId, attributes(shortcut_prefix))]
pub fn derive_shortcut_id(input: TokenStream) -> TokenStream {
    derive_id_impl(input, "ShortcutId")
}

fn derive_id_impl(input: TokenStream, wrapper_type: &str) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let name_str = LitStr::new(&name.to_string(), name.span());
    let wrapper_ident = syn::Ident::new(wrapper_type, name.span());

    match &input.data {
        Data::Enum(data_enum) => {
            // Generate Into implementation for enum variants
            let variants: Vec<_> = data_enum
                .variants
                .iter()
                .map(|variant| {
                    let variant_name = &variant.ident;
                    let variant_str = LitStr::new(&variant_name.to_string(), variant_name.span());
                    let full_id = quote! {
                        concat!(
                            module_path!(), "::",
                            #name_str, "::",
                            #variant_str
                        )
                    };

                    quote! {
                        #name::#variant_name => ::limur::#wrapper_ident(#full_id)
                    }
                })
                .collect();

            let expanded = quote! {
                impl From<#name> for ::limur::#wrapper_ident {
                    fn from(value: #name) -> Self {
                        match value {
                            #(#variants),*
                        }
                    }
                }
            };

            TokenStream::from(expanded)
        }
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Unit => {
                let full_id = quote! {
                    concat!(module_path!(), "::", #name_str)
                };

                let expanded = quote! {
                    impl From<#name> for ::limur::#wrapper_ident {
                        fn from(_: #name) -> Self {
                            ::limur::#wrapper_ident(#full_id)
                        }
                    }
                };

                TokenStream::from(expanded)
            }
            _ => {
                panic!("Only unit structs are supported for {}", wrapper_type);
            }
        },
        _ => panic!(
            "{} can only be derived for enums and unit structs",
            wrapper_type
        ),
    }
}
