use proc_macro::TokenStream;
use scalp_core::case::Case;

#[proc_macro]
pub fn to_string(input: TokenStream) -> TokenStream {
    use proc_macro::{Literal, TokenTree};

    let mut output = TokenStream::new();
    output.extend(input.into_iter().map(|tree| match tree {
        TokenTree::Group(group) => TokenTree::Literal(Literal::string(group.to_string().as_str())),
        TokenTree::Punct(punctuation) => {
            TokenTree::Literal(Literal::string(punctuation.to_string().as_str()))
        }
        TokenTree::Ident(identifier) => {
            TokenTree::Literal(Literal::string(identifier.to_string().as_str()))
        }
        TokenTree::Literal(literal) => {
            TokenTree::Literal(Literal::string(literal.to_string().trim_matches('"')))
        }
    }));
    output
}

macro_rules! case {
    ($case: ident) => {
        #[proc_macro]
        pub fn $case(input: TokenStream) -> TokenStream {
            use proc_macro::{Group, Ident, Literal, TokenTree};

            let mut output = TokenStream::new();
            output.extend(input.into_iter().map(|tree| match tree {
                TokenTree::Group(group) => {
                    TokenTree::Group(Group::new(group.delimiter(), $case(group.stream())))
                }
                TokenTree::Ident(identifier) => {
                    let name = Case::$case(identifier.to_string().as_str());
                    TokenTree::Ident(Ident::new(&name, identifier.span()))
                }
                TokenTree::Punct(punctuation) => TokenTree::Punct(punctuation),
                TokenTree::Literal(literal) => {
                    let value = Case::$case(literal.to_string().trim_matches('"'));
                    TokenTree::Literal(Literal::string(&value))
                }
            }));
            output
        }
    };
}

case!(pascal);
case!(camel);
case!(snake);
case!(kebab);
case!(upper);
case!(lower);
case!(upper_snake);
case!(upper_kebab);
