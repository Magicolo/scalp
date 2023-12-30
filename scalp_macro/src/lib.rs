use proc_macro::TokenStream;
use quote::{__private::Span, quote};
use syn::{parse_macro_input, punctuated::Punctuated, DeriveInput, Ident, Path};

#[proc_macro_derive(Parse)]
pub fn parse(input: TokenStream) -> TokenStream {
    let DeriveInput {
        ident,
        generics,
        attrs,
        data,
        vis,
    } = parse_macro_input!(input as DeriveInput);
    let parse_path = path(["scalp", "Parse"]);
    let context_path = path(["scalp", "Context"]);
    let error_path = path(["scalp", "Error"]);
    let write_path = path(["std", "io", "Write"]);

    quote!(impl #parse_path for #ident #generics {
        fn parse(#context_path { arguments, environment }: &mut #context_path) -> Result<Self, #error_path> {
            fn help(write: &mut dyn #write_path) { }
            fn version(write: &mut dyn #write_path) { }
            unimplemented!()
        }
    }).into()
}

fn path<'a>(segments: impl IntoIterator<Item = &'a str>) -> Path {
    let mut separated = Punctuated::new();
    for segment in segments {
        separated.push(Ident::new(segment, Span::call_site()).into());
    }
    Path {
        segments: separated,
        leading_colon: None,
    }
}