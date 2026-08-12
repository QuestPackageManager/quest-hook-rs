use std::ops::Range;

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::Result;

pub fn expand(range: Range<usize>) -> Result<TokenStream> {
    let mut ts = TokenStream::new();
    for n in range {
        let generics = (1..=n).map(|n| format_ident!("T{}", n));

        let generics_impl = generics.clone();
        let generics_ty = generics.clone();
        let generics_where = generics.clone();
        let generics_classes = generics.clone();

        let impl_ts = quote! {
            impl<#(#generics_impl),*> Generics for (#(#generics_ty,)*)
            where
                #(#generics_where: Type),*
            {
                const COUNT: usize = #n;

                fn classes() -> Vec<&'static Il2CppClass> {
                    vec![#(#generics_classes::class()),*]
                }
            }
        };
        ts.extend(TokenStream::from(impl_ts));
    }

    Ok(ts)
}
