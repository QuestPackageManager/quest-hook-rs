use heck::{ToLowerCamelCase, ToSnakeCase};
use proc_macro::TokenStream;
use proc_macro2::{Group, TokenStream as TokenStream2, TokenTree as TokenTree2};
use quote::{format_ident, quote, quote_spanned, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{
    Abi, Attribute, Error, FnArg, GenericParam, Ident, ItemFn, LitStr, Pat, PatType, ReturnType,
    Token, Type, TypeTuple,
};

/// The parsed argument list of a `#[hook(...)]` attribute: the three
/// required target identifiers, followed by any number of optional
/// `key = "value"` arguments.
pub struct HookArgs {
    namespace: LitStr,
    class: LitStr,
    method: LitStr,
    extra: Vec<HookArg>,
}

impl Parse for HookArgs {
    fn parse(input: ParseStream<'_>) -> Result<Self, Error> {
        let namespace: LitStr = input.parse()?;
        input.parse::<Token![,]>()?;
        let class: LitStr = input.parse()?;
        input.parse::<Token![,]>()?;
        let method: LitStr = input.parse()?;

        let mut extra = Vec::new();
        while input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            if input.is_empty() {
                break;
            }
            extra.push(input.parse()?);
        }

        Ok(Self {
            namespace,
            class,
            method,
            extra,
        })
    }
}

pub fn expand(args: &HookArgs, input: ItemFn) -> Result<TokenStream, Error> {
    let metadata = Metadata::new(args, input)?;
    metadata.validate()?;

    let outer_fn = metadata.outer_fn();
    let struct_def = metadata.struct_def();
    let struct_impl = metadata.struct_impl();
    let static_def = metadata.static_def();
    let trait_impl = metadata.trait_impl();

    let ts = quote! {
        #outer_fn
        #struct_def
        #struct_impl
        #static_def
        #trait_impl
    };
    Ok(ts.into())
}

/// A single optional `key = "value"` argument in a `#[hook(...)]`
/// attribute, following the three required target identifiers.
enum HookArg {
    /// Overrides the hook's own namespace
    Namespace(LitStr),
    /// A `Priority::before` filter
    Before(LitStr),
    /// A `Priority::after` filter
    After(LitStr),
}

impl Parse for HookArg {
    fn parse(input: ParseStream<'_>) -> Result<Self, Error> {
        let key: Ident = input.parse()?;
        input.parse::<Token![=]>()?;
        let value: LitStr = input.parse()?;

        match key.to_string().as_str() {
            "namespace" => Ok(Self::Namespace(value)),
            "before" => Ok(Self::Before(value)),
            "after" => Ok(Self::After(value)),
            other => Err(Error::new_spanned(
                key,
                format!("unknown hook argument `{other}`"),
            )),
        }
    }
}

/// A parsed `before`/`after` filter value, mirroring
/// `hook_backend::HookFilter` but with both fields resolved up front instead
/// of carried around as raw strings.
#[cfg_attr(test, derive(Debug))]
enum HookFilter {
    /// Matches a hook with the given name in any namespace
    Name(String),
    /// Matches any hook in the given namespace
    Namespace(String),
    /// Matches a hook with the given name in the given namespace
    Both { namespace: String, name: String },
}

impl HookFilter {
    /// Parses a `before`/`after` filter value in one of the forms `"name"`,
    /// `"namespace::"`, or `"namespace::name"`.
    fn parse(lit: &LitStr) -> Result<Self, Error> {
        let value = lit.value();
        let Some((namespace, name)) = value.split_once("::") else {
            return Ok(Self::Name(value));
        };
        let namespace = (!namespace.is_empty()).then(|| namespace.to_string());
        let name = (!name.is_empty()).then(|| name.to_string());

        match (namespace, name) {
            // `"namespace::name"`
            (Some(namespace), Some(name)) => Ok(Self::Both { namespace, name }),
            // `"namespace::"`
            (Some(namespace), None) => Ok(Self::Namespace(namespace)),
            // `"::"`: neither half names anything to match
            (None, _) => Err(Error::new_spanned(
                lit,
                "filter must specify a namespace, a name, or both",
            )),
        }
    }

    /// Returns a `TokenStream2` that constructs a `hook_backend::HookFilter`
    fn expr(&self) -> TokenStream2 {
        let (namespace, name) = match self {
            Self::Name(name) => (
                quote!(::std::option::Option::None),
                quote!(::std::option::Option::Some(#name)),
            ),
            Self::Namespace(namespace) => (
                quote!(::std::option::Option::Some(#namespace)),
                quote!(::std::option::Option::None),
            ),
            Self::Both { namespace, name } => (
                quote!(::std::option::Option::Some(#namespace)),
                quote!(::std::option::Option::Some(#name)),
            ),
        };
        quote! {
            ::quest_hook::hook_backend::HookFilter {
                namespace: #namespace,
                name: #name,
            }
        }
    }
}

pub struct Metadata {
    namespace: String,
    class: String,
    method: String,
    /// Overrides the hook's own namespace; defaults to the crate name
    hook_namespace: Option<String>,
    /// `Priority::before` filters
    before: Vec<HookFilter>,
    /// `Priority::after` filters
    after: Vec<HookFilter>,
    input: ItemFn,
}

impl Metadata {
    fn new(args: &HookArgs, input: ItemFn) -> Result<Self, Error> {
        let namespace = args.namespace.value();
        let class = args.class.value();
        let method = args.method.value();

        let mut hook_namespace = None;
        let mut before = Vec::new();
        let mut after = Vec::new();

        for arg in &args.extra {
            match arg {
                HookArg::Namespace(value) => {
                    if hook_namespace.is_some() {
                        return Err(Error::new_spanned(value, "duplicate hook argument"));
                    }
                    hook_namespace = Some(value.value());
                }
                HookArg::Before(value) => before.push(HookFilter::parse(value)?),
                HookArg::After(value) => after.push(HookFilter::parse(value)?),
            }
        }

        Ok(Self {
            namespace,
            class,
            method,
            hook_namespace,
            before,
            after,
            input,
        })
    }

    fn hook_namespace_expr(&self) -> TokenStream2 {
        match &self.hook_namespace {
            Some(ns) => quote!(#ns),
            None => quote!(::std::env!("CARGO_PKG_NAME")),
        }
    }

    fn priority_expr(&self) -> TokenStream2 {
        let before = self.before.iter().map(HookFilter::expr);
        let after = self.after.iter().map(HookFilter::expr);

        quote! {
            ::quest_hook::hook_backend::Priority {
                before: ::std::vec![#(#before),*],
                after: ::std::vec![#(#after),*],
            }
        }
    }

    fn validate(&self) -> Result<(), Error> {
        if let Some(constness) = self.input.sig.constness {
            return Err(Error::new_spanned(constness, "Cannot hook const functions"));
        }

        if let Some(asyncness) = self.input.sig.asyncness {
            return Err(Error::new_spanned(asyncness, "Cannot hook async functions"));
        }

        if let Some(Abi {
            name: Some(abi), ..
        }) = &self.input.sig.abi
        {
            if abi.value() != "C" {
                return Err(Error::new_spanned(
                    abi,
                    "Cannot hook functions with non-C ABIs",
                ));
            }
        }

        let generics = &self.input.sig.generics;
        if !self
            .input
            .sig
            .generics
            .params
            .iter()
            .all(|g| matches!(g, GenericParam::Lifetime(_)))
        {
            return Err(Error::new_spanned(
                generics,
                "Cannot hook generic functions",
            ));
        }

        for (i, arg) in self.input.sig.inputs.iter().enumerate() {
            match arg {
                FnArg::Receiver(_) => {
                    return Err(Error::new_spanned(
                        arg,
                        "Cannot hook functions taking a `self` parameter",
                    ))
                }
                FnArg::Typed(PatType {
                    pat: box Pat::Ident(ident),
                    ..
                }) if ident.ident == "self" => {
                    return Err(Error::new_spanned(
                        arg,
                        "Cannot hook functions taking a `self` parameter",
                    ))
                }
                FnArg::Typed(PatType { attrs, .. }) if i != 0 => {
                    let has_this_attr = attrs.iter().any(|a| attr_is(a, "this"));
                    if has_this_attr {
                        return Err(Error::new_spanned(
                            arg,
                            "`this` can only be the first parameter",
                        ));
                    }
                }
                FnArg::Typed(_) => (),
            }
        }

        if let Some(variadic) = &self.input.sig.variadic {
            return Err(Error::new_spanned(
                variadic,
                "Cannot hook variadic functions",
            ));
        }

        Ok(())
    }

    fn hook_name(&self) -> &Ident {
        &self.input.sig.ident
    }

    fn struct_name(&self) -> Ident {
        let hook_name = self.hook_name().to_string();
        let struct_name = hook_name.to_lower_camel_case();
        format_ident!("{}Struct", struct_name)
    }

    fn fn_name(&self) -> Ident {
        let hook_name = self.hook_name().to_string();
        let fn_name = hook_name.to_snake_case();
        format_ident!("_{}_fn", fn_name)
    }

    fn filtered_attrs(&self) -> impl Iterator<Item = &'_ Attribute> + '_ {
        self.input.attrs.iter().filter(|a| !attr_is(a, "hook"))
    }

    fn this(&self) -> Option<&PatType> {
        let first_input = match self.input.sig.inputs.iter().next()? {
            FnArg::Typed(arg) => arg,
            FnArg::Receiver(_) => unreachable!(),
        };

        let is_this = match &first_input.pat {
            box Pat::Ident(ident) if ident.ident == "this" => true,
            _ => first_input.attrs.iter().any(|a| attr_is(a, "this")),
        };
        if !is_this {
            return None;
        }

        Some(first_input)
    }

    fn has_this(&self) -> bool {
        self.this().is_some()
    }

    fn this_ty(&self) -> Option<&Type> {
        self.this().map(|this| &*this.ty)
    }

    fn params(&self) -> impl Iterator<Item = &'_ PatType> + '_ {
        let skip = if self.has_this() { 1 } else { 0 };

        self.input
            .sig
            .inputs
            .iter()
            .skip(skip)
            .map(|arg| match arg {
                FnArg::Typed(arg) => arg,
                FnArg::Receiver(_) => unreachable!(),
            })
    }

    fn params_ty(&self) -> impl Iterator<Item = &'_ Type> + '_ {
        self.params().map(|param| &*param.ty)
    }

    fn return_ty(&self) -> Type {
        match &self.input.sig.output {
            ReturnType::Default => unit_ty(),
            ReturnType::Type(_, box ty) => ty.clone(),
        }
    }

    fn typechecking_this_ty(&self) -> Type {
        self.this_ty().cloned().unwrap_or_else(unit_ty)
    }

    fn typechecking_params_ty(&self) -> TokenStream2 {
        let params_ty = self.params_ty();
        quote!((#(#params_ty,)*))
    }

    fn actual_this_ty(&self) -> Option<TokenStream2> {
        self.this_ty().map(
            |t| quote_spanned!(t.span()=> <#t as ::quest_hook::libil2cpp::ThisParameter>::Actual),
        )
    }

    fn actual_params_ty(&self) -> impl Iterator<Item = TokenStream2> + '_ {
        self.params_ty()
            .map(|t| quote_spanned!(t.span()=> <#t as ::quest_hook::libil2cpp::Parameter>::Actual))
    }

    fn actual_return_ty(&self) -> TokenStream2 {
        let return_ty = self.return_ty();
        quote_spanned!(return_ty.span()=> <#return_ty as ::quest_hook::libil2cpp::Return>::Actual)
    }

    fn this_ident(&self) -> Option<Ident> {
        self.this().map(|_| format_ident!("this"))
    }

    fn params_ident(&self) -> impl Iterator<Item = Ident> + '_ {
        self.params()
            .enumerate()
            .map(|(i, _)| format_ident!("p{}", i))
    }

    fn inner_fn(&self) -> TokenStream2 {
        let attrs = self.filtered_attrs();
        let unsafety = &self.input.sig.unsafety;
        let inputs = &self.input.sig.inputs;
        let return_ty = self.return_ty();
        let block = &self.input.block;

        quote! {
            #(#attrs) *
            #unsafety fn inner(#inputs) -> #return_ty #block
        }
    }

    fn outer_fn(&self) -> TokenStream2 {
        let unsafety = self.input.sig.unsafety;
        let name = self.fn_name();
        let return_ty = self.actual_return_ty();
        let inner_fn = self.inner_fn();

        let this_param = self
            .this_ident()
            .zip(self.actual_this_ty())
            .map(|(i, t)| quote!(#i: #t,));

        let this_arg = self
            .this_ident()
            .map(|i| quote!(::quest_hook::libil2cpp::ThisParameter::from_actual(#i),));

        let params_params = self
            .params_ident()
            .zip(self.actual_params_ty())
            .map(|(i, t)| quote!(#i: #t,));

        let params_args = self
            .params_ident()
            .map(|i| quote!(::quest_hook::libil2cpp::Parameter::from_actual(#i),));

        quote! {
            #[inline(never)]
            pub #unsafety extern "C" fn #name(#this_param #(#params_params)*) -> #return_ty {
                #inner_fn
                let r = inner(#this_arg #(#params_args)*);
                ::quest_hook::libil2cpp::Return::into_actual(r)
            }
        }
    }

    fn struct_def(&self) -> TokenStream2 {
        let vis = &self.input.vis;
        let struct_name = self.struct_name();

        quote! {
            #vis struct #struct_name {
                hook: ::quest_hook::hook_backend::FunctionHook,
            }
        }
    }

    fn static_def(&self) -> TokenStream2 {
        let vis = &self.input.vis;
        let name = self.hook_name();
        let struct_name = self.struct_name();

        quote! {
            #[allow(non_upper_case_globals)]
            #vis static #name: #struct_name = #struct_name {
                hook: ::quest_hook::hook_backend::FunctionHook::new(),
            };
        }
    }

    fn install_fn(&self) -> TokenStream2 {
        let vis = &self.input.vis;

        let namespace = &self.namespace;
        let class = &self.class;
        let method = &self.method;

        let this_ty = self.typechecking_this_ty();
        let params_ty = self.typechecking_params_ty();
        let return_ty = self.return_ty();

        let fn_name = self.fn_name();
        let name = self.hook_name();
        let hook_namespace = self.hook_namespace_expr();
        let hook_name_str = self.hook_name().to_string();

        quote! {
            #vis fn install(&self) -> Result<::quest_hook::HookHandle, quest_hook::HookInstallError> {
                use ::std::ptr::null_mut;
                use ::std::sync::atomic::Ordering;
                use ::quest_hook::HookInstallError;
                use ::quest_hook::hook_backend::HookName;
                use ::quest_hook::libil2cpp::{Il2CppClass, WrapRaw};

                if self.hook.is_installed() {
                    return Err(HookInstallError::AlreadyInstalled);
                }

                let class = match Il2CppClass::find(#namespace, #class) {
                    Some(class) => class,
                    None => return Err(HookInstallError::ClassNotFound),
                };
                let method = match class.find_method_callee::<#this_ty, #params_ty, #return_ty>(#method) {
                    Ok(method) => method,
                    Err(_) => return Err(HookInstallError::MethodNotFound),
                };

                let hook_name = HookName { namespace: #hook_namespace, name: #hook_name_str };

                let success = unsafe {
                    self.hook.install(
                        method.raw().methodPointer.unwrap() as *const (),
                        #fn_name as *const (),
                        hook_name,
                        self.priority(),
                    )
                };
                if success {
                    Ok(::quest_hook::HookHandle::new(&#name.hook))
                } else {
                    Err(HookInstallError::InstallError)
                }
            }
        }
    }

    /// This hook's declared `before`/`after` priority, built fresh on each
    /// call
    fn priority_fn(&self) -> TokenStream2 {
        let vis = &self.input.vis;
        let priority_expr = self.priority_expr();

        quote! {
            #vis fn priority(&self) -> ::quest_hook::hook_backend::Priority {
                #priority_expr
            }
        }
    }

    fn original_ty(&self) -> TokenStream2 {
        let this_ty = self.actual_this_ty().map(|t| quote!(#t,));
        let params_ty = self.actual_params_ty().map(|t| quote!(#t,));
        let return_ty = self.actual_return_ty();

        quote!(extern "C" fn(#this_ty #(#params_ty)*) -> #return_ty)
    }

    fn original_fn(&self) -> TokenStream2 {
        let vis = &self.input.vis;
        let return_ty = self.return_ty();
        let original_ty = self.original_ty();

        let this_param = self
            .this_ident()
            .zip(self.this_ty())
            .map(|(i, t)| quote!(#i: #t,));

        let this_arg = self
            .this_ident()
            .map(|i| quote!(::quest_hook::libil2cpp::ThisParameter::into_actual(#i),));

        let params_params = self
            .params_ident()
            .zip(self.params_ty())
            .map(|(i, t)| quote!(#i: #t,));

        let params_args = self
            .params_ident()
            .map(|i| quote!(::quest_hook::libil2cpp::Parameter::into_actual(#i),));

        quote! {
            #vis fn original(&self, #this_param #(#params_params)*) -> #return_ty {
                use ::std::mem::transmute;
                use ::std::sync::atomic::Ordering;

                let ptr = self.hook.original().expect("hook is not installed");
                let original = unsafe { transmute::<*const (), #original_ty>(ptr) };

                let r = original(#this_arg #(#params_args)*);
                ::quest_hook::libil2cpp::Return::from_actual(r)
            }
        }
    }

    fn struct_impl(&self) -> TokenStream2 {
        let struct_name = self.struct_name();
        let install_fn = self.install_fn();
        let priority_fn = self.priority_fn();
        let original_fn = self.original_fn();

        quote! {
            impl #struct_name {
                #install_fn
                #priority_fn
                #original_fn
            }
        }
    }

    fn trait_impl(&self) -> TokenStream2 {
        let struct_name = self.struct_name();

        let namespace = &self.namespace;
        let class = &self.class;
        let method = &self.method;

        let this_ty = staticify(self.typechecking_this_ty());
        let params_ty = staticify(self.typechecking_params_ty());
        let return_ty = staticify(self.return_ty());

        let fn_name = self.fn_name();
        let hook_name_str = self.hook_name().to_string();
        let hook_namespace = self.hook_namespace_expr();

        quote! {
            impl ::quest_hook::Hook for #struct_name {
                type This = #this_ty;
                type Parameters = #params_ty;
                type Return = #return_ty;

                const NAMESPACE: &'static str = #namespace;
                const CLASS_NAME: &'static str = #class;
                const METHOD_NAME: &'static str = #method;
                const HOOK_NAMESPACE: &'static str = #hook_namespace;
                const HOOK_NAME: &'static str = #hook_name_str;

                fn install(&self) -> Result<::quest_hook::HookHandle, ::quest_hook::HookInstallError> {
                    self.install()
                }

                fn priority(&self) -> ::quest_hook::hook_backend::Priority {
                    self.priority()
                }

                fn original(&self) -> Option<*const ()> {
                    self.hook.original()
                }
                fn hook(&self) -> *const () {
                    #fn_name as *const ()
                }
            }
        }
    }
}

fn unit_ty() -> Type {
    Type::Tuple(TypeTuple {
        paren_token: Default::default(),
        elems: Default::default(),
    })
}

fn attr_is(attr: &Attribute, ident: &str) -> bool {
    matches!(attr.path().get_ident(), Some(ai) if ai == ident)
}

fn staticify(tokens: impl ToTokens) -> TokenStream2 {
    let mut ts = TokenStream2::new();
    let mut iter = tokens.to_token_stream().into_iter().peekable();
    while let Some(tt) = iter.next() {
        match &tt {
            TokenTree2::Group(g) => {
                let delimiter = g.delimiter();
                let stream = staticify(g.stream());
                ts.extend_one(TokenTree2::Group(Group::new(delimiter, stream)));
            }
            TokenTree2::Punct(p) if p.as_char() == '&' => match iter.peek() {
                Some(TokenTree2::Punct(p)) if p.as_char() == '\'' => ts.extend_one(tt),
                _ => ts.extend_one(quote_spanned!(tt.span()=> &'static)),
            },
            _ => ts.extend_one(tt),
        }
    }
    ts
}

#[cfg(test)]
mod tests {
    use quote::quote;
    use syn::LitStr;

    use super::{HookArgs, HookFilter};

    fn lit(value: &str) -> LitStr {
        LitStr::new(value, proc_macro2::Span::call_site())
    }

    #[test]
    fn filter_parses_name_only() {
        match HookFilter::parse(&lit("my_hook")).unwrap() {
            HookFilter::Name(name) => assert_eq!(name, "my_hook"),
            other => panic!("expected Name, got a filter with different fields set: {other:?}"),
        }
    }

    #[test]
    fn filter_parses_namespace_only() {
        match HookFilter::parse(&lit("my_crate::")).unwrap() {
            HookFilter::Namespace(namespace) => assert_eq!(namespace, "my_crate"),
            other => panic!("expected Namespace, got: {other:?}"),
        }
    }

    #[test]
    fn filter_parses_namespace_and_name() {
        match HookFilter::parse(&lit("my_crate::my_hook")).unwrap() {
            HookFilter::Both { namespace, name } => {
                assert_eq!(namespace, "my_crate");
                assert_eq!(name, "my_hook");
            }
            other => panic!("expected Both, got: {other:?}"),
        }
    }

    #[test]
    fn filter_rejects_empty_namespace_and_name() {
        assert!(HookFilter::parse(&lit("::")).is_err());
    }

    #[test]
    fn hook_args_parses_required_and_repeated_optional_arguments() {
        let args: HookArgs = syn::parse2(quote! {
            "MyNamespace", "MyClass", "MyMethod",
            namespace = "custom_ns",
            before = "other_hook",
            after = "ns::",
            after = "ns::name",
        })
        .unwrap();

        assert_eq!(args.namespace.value(), "MyNamespace");
        assert_eq!(args.class.value(), "MyClass");
        assert_eq!(args.method.value(), "MyMethod");
        assert_eq!(args.extra.len(), 4);
    }

    #[test]
    fn hook_args_allows_no_optional_arguments() {
        let args: HookArgs = syn::parse2(quote! {
            "MyNamespace", "MyClass", "MyMethod"
        })
        .unwrap();

        assert!(args.extra.is_empty());
    }

    #[test]
    fn hook_args_rejects_unknown_argument() {
        let result: syn::Result<HookArgs> = syn::parse2(quote! {
            "MyNamespace", "MyClass", "MyMethod",
            frobnicate = "oops",
        });
        assert!(result.is_err());
    }

    #[test]
    fn hook_args_rejects_missing_required_argument() {
        let result: syn::Result<HookArgs> = syn::parse2(quote! {
            "MyNamespace", "MyClass"
        });
        assert!(result.is_err());
    }
}
