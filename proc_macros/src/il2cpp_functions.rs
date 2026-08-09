use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::Token;
use syn::{Error, FnArg, ForeignItemFn, LitByteStr, PatType, Signature};

pub struct Input(syn::Visibility, syn::Expr, Vec<ForeignItemFn>);

impl Parse for Input {
    fn parse(input: ParseStream<'_>) -> Result<Self, Error> {
        let vis = input.parse::<syn::Visibility>()?;
        let il2cpp_binary = input.parse::<syn::Expr>()?;

        // Require a `=>` token following the expression so the macro must be
        // invoked like `il2cpp_functions! { IL2CPP_BINARY => ... }`.
        let _arrow: Token![=>] = input.parse()?;

        let mut fns = Vec::new();
        while !input.is_empty() {
            fns.push(input.parse::<ForeignItemFn>()?);
        }
        Ok(Self(vis, il2cpp_binary, fns))
    }
}

pub fn expand(input: &Input) -> Result<TokenStream, Error> {
    let vis = &input.0;
    let il2cpp_lib = &input.1;
    let mut ts = quote! {
        #vis static LIBIL2CPP: LazyLock<Library> =
            LazyLock::new(|| unsafe { Library::new(#il2cpp_lib) }.unwrap());
    };

    for ForeignItemFn {
        attrs,
        vis,
        sig:
            Signature {
                ident,
                inputs,
                output,
                ..
            },
        ..
    } in input.2.iter()
    {
        let name = LitByteStr::new(format!("il2cpp_{}", ident).as_bytes(), ident.span());

        let inputs = inputs
            .iter()
            .map(|i| match i {
                FnArg::Receiver(_) => {
                    Err(Error::new_spanned(i, "il2cpp functions cannot take `self`"))
                }
                FnArg::Typed(p) => Ok(p),
            })
            .collect::<Result<Vec<&PatType>, Error>>()?;
        let inputs = inputs.as_slice();

        let input_pats = inputs.iter().map(|i| &i.pat);
        let input_tys = inputs.iter().map(|i| &i.ty);

        let wrapper = quote! {
            #(#attrs) *
            #vis unsafe fn #ident(#(#inputs),*) #output {
                static FN: OnceLock<Symbol<'static, unsafe extern "C" fn(#(#input_tys),*) #output>> =
                    OnceLock::new();
                let fun = FN.get_or_init(|| unsafe { LIBIL2CPP.get(#name) }.unwrap());
                (**fun)(#(#input_pats),*)
            }
        };
        ts.extend_one(wrapper);
    }

    Ok(ts.into())
}
