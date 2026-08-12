use std::ops::Range;

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{Index, Result};

pub fn expand(range: Range<usize>) -> Result<TokenStream> {
    let mut ts = TokenStream::new();
    for n in range {
        let generic_params_argument = (1..=n).map(|n| format_ident!("A{}", n));
        let matches_types_argument = generic_params_argument
            .clone()
            .enumerate()
            .map(|(n, gp)| quote!(<#gp>::matches(tys[#n])));
        let invokables = (0..n).map(Index::from).map(|n| quote!(self.#n.invokable()));

        let generic_params_parameter = (1..=n).map(|n| format_ident!("P{}", n));
        let matches_types_parameter = generic_params_parameter
            .clone()
            .enumerate()
            .map(|(n, gp)| quote!(<#gp>::matches(types[#n])));

        // log params
        let log_parameters = generic_params_parameter.clone().enumerate().map(|(n, gp)| {
            quote!({
                #[cfg(feature = "trace")]
                crate::debug!("\tChecking parameter {} {:?} vs method param {:?}",
                    #n,
                    stringify!(#gp),
                    types.get(#n).map(|&ty| (ty, <#gp>::matches(ty)))
                );
            })
        });

        let generic_params_argument_tuple = generic_params_argument.clone();
        let generic_params_argument_where = generic_params_argument.clone();
        let generic_params_argument_type = generic_params_argument.clone();
        let generic_params_argument_classes = generic_params_argument.clone();

        let generic_params_parameter_tuple = generic_params_parameter.clone();
        let generic_params_parameter_where = generic_params_parameter.clone();
        let generic_params_parameter_classes = generic_params_parameter.clone();

        let impl_ts = quote! {
            unsafe impl<#(#generic_params_argument),*> Arguments<#n> for (#(#generic_params_argument_tuple,)*)
            where
                #(#generic_params_argument_where: Argument),*
            {
                type Type = (#(#generic_params_argument_type::Type,)*);

                fn matches(tys: &[&Il2CppType]) -> bool {
                    tys.len() == #n && #(#matches_types_argument) && *
                }

                fn classes() -> [&'static Il2CppClass; #n] {
                    [#(#generic_params_argument_classes::class()),*]
                }

                fn invokable(&mut self) -> [*mut c_void; #n] {
                    [#(#invokables),*]
                }
            }

            unsafe impl<#(#generic_params_parameter),*> Parameters for (#(#generic_params_parameter_tuple,)*)
            where
                #(#generic_params_parameter_where: Parameter),*
            {
                const COUNT: usize = #n;

                fn matches(types: &[&Il2CppType]) -> bool {
                    #(#log_parameters)*
                    types.len() == #n && #(#matches_types_parameter) && *
                }

                fn classes() -> Vec<&'static Il2CppClass> {
                    vec![#(#generic_params_parameter_classes::class()),*]
                }
            }
        };
        ts.extend(TokenStream::from(impl_ts));
    }

    Ok(ts)
}
