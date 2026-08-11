use std::borrow::Cow;
use std::ffi::{CStr, CString};
use std::fmt::Display;
use std::mem::transmute;
use std::{fmt, ptr, slice, vec};

use crate::{
    raw, Arguments, FieldInfo, Gc, Generics, Il2CppException, Il2CppType, MethodInfo, Parameters,
    PropertyInfo, Return, Returned, ThisParameter, Type, WrapRaw,
};

#[cfg(feature = "il2cpp_v31")]
type FieldInfoSlice<'a> = &'a [FieldInfo];
#[cfg(feature = "il2cpp_v29")]
type FieldInfoSlice<'a> = &'a [FieldInfo];
#[cfg(feature = "il2cpp_v24")]
type FieldInfoSlice<'a> = &'a [FieldInfo];
#[cfg(feature = "unity2018")]
type FieldInfoSlice<'a> = &'a [&'static FieldInfo];

#[cfg(feature = "il2cpp_v31")]
type PropertyInfoSlice<'a> = &'a [PropertyInfo];
#[cfg(feature = "il2cpp_v29")]
type PropertyInfoSlice<'a> = &'a [PropertyInfo];
#[cfg(feature = "il2cpp_v24")]
type PropertyInfoSlice<'a> = &'a [PropertyInfo];
#[cfg(feature = "unity2018")]
type PropertyInfoSlice<'a> = &'a [&'static PropertyInfo];

/// An il2cpp class
#[repr(transparent)]
pub struct Il2CppClass(raw::Il2CppClass);

unsafe impl Send for Il2CppClass {}
unsafe impl Sync for Il2CppClass {}

impl Il2CppClass {
    /// Find a class by namespace and name
    #[crate::instrument(level = "debug")]
    pub fn find(namespace: &str, name: &str) -> Option<&'static Self> {
        #[cfg(feature = "cache")]
        let key = {
            let key = cache::ClassCacheKey {
                namespace: namespace.into(),
                name: name.into(),
            };
            if let Some(class) = cache::CLASS_CACHE.with(|c| c.borrow().get(&key).copied()) {
                debug!("cache hit");
                return Some(class);
            }
            debug!("cache miss");
            key
        };

        let c_namespace = CString::new(namespace).unwrap();
        let c_name = CString::new(name).unwrap();

        let domain = unsafe { raw::domain_get() };

        let mut assemblies_count = 0;
        let assemblies = unsafe { raw::domain_get_assemblies(domain, &mut assemblies_count) };

        debug!("assemblies_count: {}", assemblies_count);
        debug!("Looking for class: {}.{}", namespace, name);

        for assembly in assemblies.iter().take(assemblies_count) {
            // For some reason, an assembly might not have an image
            let image = match unsafe { raw::assembly_get_image(assembly) } {
                Some(image) => image,
                None => continue,
            };

            let class =
                unsafe { raw::class_from_name(image, c_namespace.as_ptr(), c_name.as_ptr()) }
                    .map(|class| unsafe { Self::wrap(class) });

            debug!("class: {class:?} in assembly image {image:?}",);

            if let Some(class) = class {
                // Ensure class is initialized
                // TODO: Call Class::Init somehow
                let _ =
                    unsafe { raw::class_get_method_from_name(&class.0, c"".as_ptr().cast(), 0) };

                debug!("class found: {class}", class = class);

                #[cfg(feature = "cache")]
                cache::CLASS_CACHE.with(move |c| c.borrow_mut().insert(key.into(), class));

                return Some(class);
            }
        }

        debug!("Class not found {}.{}", namespace, name);
        None
    }

    /// Finds a generic method by namespace, name and generic parameters
    pub fn find_generic<G>(namespace: &str, name: &str) -> Option<&'static Self>
    where
        G: Generics,
    {
        Self::find(namespace, &format!("{}`{}", name, G::COUNT))?
            .make_generic::<G>()
            .unwrap()
    }

    /// Find a method belonging to the class or its parents by name with type
    /// checking
    ///
    /// If more than one method type-checks (an overload), the best match is
    /// picked the same way beatsaber-hook's `find_method` does: a candidate
    /// whose parameters are an exact class-for-class match wins immediately,
    /// otherwise every candidate is scored by [`param_distance`] and the
    /// lowest-scoring one is used - this never fails merely because more
    /// than one overload type-checks (see [`Self::resolve_method`]).
    #[crate::instrument(level = "debug")]
    pub fn find_method<A, R, const N: usize>(
        &self,
        name: &str,
    ) -> Result<&'static MethodInfo, FindMethodError>
    where
        A: Arguments<N>,
        R: Returned,
    {
        #[cfg(feature = "cache")]
        let key = {
            let class_key = cache::ClassCacheKey {
                namespace: self.namespace(),
                name: self.name(),
            };
            let key = cache::MethodCacheKey {
                class: class_key,
                name: name.into(),
                ty: std::any::TypeId::of::<fn(Self, A::Type) -> R::Type>(),
            };
            if let Some(method) = cache::METHOD_CACHE.with(|c| c.borrow().get(&key).copied()) {
                debug!("cache hit");
                return Ok(method);
            }
            debug!("cache miss");
            key
        };

        let method = self.resolve_method::<A, R, N>(name, false)?;

        #[cfg(feature = "cache")]
        cache::METHOD_CACHE.with(move |c| c.borrow_mut().insert(key.into(), method));

        Ok(method)
    }

    /// Find a `static` method belonging to the class by name with type
    /// checking
    ///
    /// See [`find_method`](Self::find_method) for how overloads are
    /// resolved when more than one candidate type-checks.
    #[crate::instrument(level = "debug")]
    pub fn find_static_method<A, R, const N: usize>(
        &self,
        name: &str,
    ) -> Result<&'static MethodInfo, FindMethodError>
    where
        A: Arguments<N>,
        R: Returned,
    {
        #[cfg(feature = "cache")]
        let key = {
            let class_key = cache::ClassCacheKey {
                namespace: self.namespace(),
                name: self.name(),
            };
            let key = cache::MethodCacheKey {
                class: class_key,
                name: name.into(),
                ty: std::any::TypeId::of::<fn((), A::Type) -> R::Type>(),
            };
            if let Some(method) = cache::METHOD_CACHE.with(|c| c.borrow().get(&key).copied()) {
                debug!("cache hit");
                return Ok(method);
            }
            debug!("cache miss");
            key
        };

        let method = self.resolve_method::<A, R, N>(name, true)?;
        debug!("Found method: {}.{}", self, name);

        #[cfg(feature = "cache")]
        cache::METHOD_CACHE.with(move |c| c.borrow_mut().insert(key.into(), method));

        Ok(method)
    }

    /// Shared implementation of [`find_method`](Self::find_method) and
    /// [`find_static_method`](Self::find_static_method) - walks the class
    /// hierarchy collecting every method named `name` (optionally requiring
    /// it be `static`) that [`A::matches`](Arguments::matches)/
    /// [`R::matches`](Returned::matches) already approve as callable, then
    /// ranks the candidates the way beatsaber-hook's `find_method` +
    /// `param_distance` do (`find.cpp`): the first candidate whose
    /// parameters are all an exact class match wins immediately; otherwise
    /// every convertible candidate is scored (lower is closer, see
    /// [`method_weight`]) and the lowest-scoring one wins. Multiple
    /// convertible candidates only ever produce a `debug!` note, mirroring
    /// upstream, which never fails merely because more than one overload
    /// type-checks.
    fn resolve_method<A, R, const N: usize>(
        &self,
        name: &str,
        static_only: bool,
    ) -> Result<&'static MethodInfo, FindMethodError>
    where
        A: Arguments<N>,
        R: Returned,
    {
        let arg_classes = A::classes();

        let mut best: Option<&'static MethodInfo> = None;
        let mut best_weight = i32::MAX;
        let mut multiple = false;

        'hierarchy: for c in self.hierarchy() {
            for mi in c.methods().iter().copied() {
                if mi.name() != name
                    || (static_only && !mi.is_static())
                    || !A::matches(mi)
                    || !R::matches(mi.return_ty())
                {
                    continue;
                }

                match method_weight(mi, &arg_classes) {
                    Closeness::Exact => {
                        best = Some(mi);
                        break 'hierarchy;
                    }
                    Closeness::Convertible(weight) if weight < best_weight => {
                        multiple = best.is_some();
                        best_weight = weight;
                        best = Some(mi);
                    }
                    Closeness::Convertible(_) => {}
                }
            }
        }

        let Some(best) = best else {
            let info = FindMethodParameters {
                ty_name: self.to_string(),
                method_name: name.to_string(),
                // TODO!
                parameters: vec![format!("UNABLE TO PROVIDE! Count! {N}")],
            };
            return Err(FindMethodError::None(info));
        };

        if multiple {
            debug!(
                "Multiple overloads of {}.{} type-checked with different weights - picked {}",
                self, name, best
            );
        }

        Ok(best)
    }

    /// Find a method belonging to the class or its parents by name with type
    /// checking from a callee perspective
    #[crate::instrument(level = "debug")]
    pub fn find_method_callee<T, P, R>(
        &self,
        name: &str,
    ) -> Result<&'static MethodInfo, FindMethodError>
    where
        T: ThisParameter,
        P: Parameters,
        R: Return,
    {
        debug!("Looking for method: {}", name);

        let mut matching = self
            .methods()
            .iter()
            .filter(|mi| {
                debug!("Looking for method: {}", name);
                debug!("mi.name() == name: {}", mi.name() == name);
                debug!("T::matches(mi): {}", T::matches(mi));
                debug!(
                    "P::matches(mi): {} count {} method {}",
                    P::matches(mi),
                    P::COUNT,
                    mi.parameters().len()
                );
                debug!("R::matches(mi.return_ty()): {}", R::matches(mi.return_ty()));
                debug!("");
                mi.name() == name && T::matches(mi) && P::matches(mi) && R::matches(mi.return_ty())
            })
            .copied();

        match (matching.next(), matching.next()) {
            // one method found
            (Some(mi), None) | (None, Some(mi)) => Ok(mi),
            // multiple methods found
            (Some(mi1), Some(mi2)) => {
                let found: Vec<FindMethodParameters> = vec![mi1, mi2]
                    .into_iter()
                    .chain(matching)
                    .map(|mi| {
                        let info = FindMethodParameters {
                            ty_name: self.to_string(),
                            method_name: name.to_string(),
                            parameters: mi.parameters().iter().map(|t| t.to_string()).collect(),
                        };
                        info
                    })
                    .collect();

                Err(FindMethodError::Many(found))
            }
            // none
            _ => {
                let info = FindMethodParameters {
                    ty_name: self.to_string(),
                    method_name: name.to_string(),
                    // TODO!
                    parameters: vec![format!("UNABLE TO PROVIDE! Count {}", P::COUNT)],
                };
                Err(FindMethodError::None(info))
            }
        }
    }

    /// Find a method belonging to the class or its parents by name and
    /// parameter count, without type checking
    pub fn find_method_unchecked(
        &self,
        name: &str,
        parameters_count: usize,
    ) -> Result<&'static MethodInfo, FindMethodError> {
        for c in self.hierarchy() {
            let mut matching = c
                .methods()
                .iter()
                .filter(|mi| mi.name() == name && mi.parameters().len() == parameters_count)
                .copied();

            match (matching.next(), matching.next()) {
                // only one match
                (Some(mi), None) => return Ok(mi),
                // multiple matches
                (Some(mi), Some(mi2)) => {
                    let found = vec![mi, mi2]
                        .into_iter()
                        .chain(matching)
                        .map(|mi| {
                            let info = FindMethodParameters {
                                ty_name: c.to_string(),
                                method_name: name.to_string(),
                                parameters: mi.parameters().iter().map(|t| t.to_string()).collect(),
                            };
                            info
                        })
                        .collect();

                    return Err(FindMethodError::Many(found));
                }
                // If we have no matches, we continue to the parent
                _ => continue,
            }
        }

        let info = FindMethodParameters {
            ty_name: self.to_string(),
            method_name: name.to_string(),
            parameters: vec![format!("UNABLE TO PROVIDE! Count {}", parameters_count)],
        };

        Err(FindMethodError::None(info))
    }

    /// Find a method declared directly on this class (not its parents) by
    /// its vtable slot
    ///
    /// Unlike [`find_method`](Self::find_method) and friends, this does not
    /// walk the class hierarchy - a vtable slot only makes sense relative to
    /// the exact class that declares the method at that slot.
    pub fn find_method_by_slot(&self, slot: u16) -> Option<&'static MethodInfo> {
        self.methods().iter().find(|mi| mi.slot() == slot).copied()
    }

    /// Resolves the method that `self` (as an instance of some concrete
    /// type) actually runs for the virtual/interface method declared at
    /// `slot` in `declaring_class`, following the vtable the same way a
    /// real virtual/interface call would
    ///
    /// `declaring_class` is typically a base class or interface further up
    /// `self`'s hierarchy - this is the mechanism generated hooks use to
    /// hook a specific override without knowing its (possibly obfuscated)
    /// name.
    pub fn find_method_by_vtable(
        &self,
        declaring_class: &Self,
        slot: u16,
    ) -> Option<&'static MethodInfo> {
        // concrete type
        if !declaring_class.is_interface() {
            let entry = self.vtable().get(slot as usize)?;
            let method = unsafe { MethodInfo::wrap_ptr(entry.method) }?;

            // A vtable entry can point at a MethodInfo with a different
            // slot for abstract methods with no direct implementation -
            // fall back to a plain slot search in that case.
            if method.slot() != slot {
                return self.find_method_by_slot(slot);
            }

            return Some(method);
        }

        // `declaring_class` is an interface: find where its vtable is
        // spliced into `self`'s vtable.
        for pair in self.interface_offsets() {
            let interface_t = unsafe { Self::wrap_ptr(pair.interfaceType) };
            if interface_t == Some(declaring_class) {
                // The interface's vtable is spliced into `self`'s vtable at the
                // given offset, so the method at `slot` in the interface is
                // at `offset + slot` in `self`'s vtable.
                let index = pair.offset as usize + slot as usize;
                return self
                    .vtable()
                    .get(index)
                    .and_then(|entry| unsafe { MethodInfo::wrap_ptr(entry.method) });
            }
        }

        // `self` might be the interface itself, in which case its own
        // `methods` array (not the vtable) is what's indexed by slot.
        if self.is_interface() {
            return self.methods().get(slot as usize).copied();
        }

        None
    }

    /// Find a field belonging to the class or its parents by name
    #[crate::instrument(level = "debug")]
    pub fn find_field(&self, name: &str) -> Option<&FieldInfo> {
        for c in self.hierarchy() {
            let mut matching = c.fields().iter().filter(|fi| fi.name() == name);

            match matching.next() {
                // If we have no matches, we continue to the parent
                None => continue,
                Some(fi) => return Some(fi),
            }
        }

        None
    }

    /// Find a property belonging to the class or its parents by name
    #[crate::instrument(level = "debug")]
    pub fn find_property(&self, name: &str) -> Option<&PropertyInfo> {
        for c in self.hierarchy() {
            let mut matching = c.properties().iter().filter(|pi| pi.name() == name);

            match matching.next() {
                // If we have no matches, we continue to the parent
                None => continue,
                Some(pi) => return Some(pi),
            }
        }

        None
    }

    /// Instanciates a generic class template with the provided generic
    /// arguments
    pub fn make_generic<G>(&self) -> Result<Option<&'static Self>, Gc<Il2CppException>>
    where
        G: Generics,
    {
        match self.ty().reflection_object().make_generic::<G>() {
            Ok(Some(ty)) => Ok(Some(unsafe {
                Self::wrap(raw::class_from_system_type(ty.raw()))
            })),
            Ok(None) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Instanciates an object of the class
    #[rustfmt::skip]
    pub fn instantiate<T>(&self) -> &'static mut T
    where
        for<'a> T: Type<Held<'a> = Option<&'a mut T>>,
    {
        assert!(T::class() == self);
        unsafe {
            let object = raw::object_new(self.raw());
            transmute(object)
        }
    }

    /// Invokes the `static` method with the given name using the given
    /// arguments, with type checking
    pub fn invoke<A, R, const N: usize>(&self, name: &str, args: A) -> crate::Result<R>
    where
        A: Arguments<N>,
        R: Returned,
    {
        let method = self
            .find_static_method::<A, R, N>(name)
            .unwrap_or_else(|e| {
                panic!(
                    "no matching methods found for non-void {}.{}({}) Cause: {e:?}",
                    self, name, N
                )
            });
        unsafe { method.invoke_unchecked((), args) }
    }

    /// Invokes the `static void` method with the given name using the given
    /// arguments, with type checking
    pub fn invoke_void<A, const N: usize>(&self, name: &str, args: A) -> crate::Result<()>
    where
        A: Arguments<N>,
    {
        let method = self
            .find_static_method::<A, (), N>(name)
            .unwrap_or_else(|e| {
                panic!(
                    "no matching methods found for void {}.{}({}) Cause: {e:?}",
                    self, name, N
                )
            });
        unsafe { method.invoke_unchecked((), args) }
    }

    /// Name of the class
    pub fn name(&self) -> Cow<'_, str> {
        let name = self.raw().name;
        assert!(!name.is_null());
        unsafe { CStr::from_ptr(name) }.to_string_lossy()
    }

    /// Namespace containing the class
    pub fn namespace(&self) -> Cow<'_, str> {
        let namespace = self.raw().namespaze;
        assert!(!namespace.is_null());
        unsafe { CStr::from_ptr(namespace) }.to_string_lossy()
    }

    /// Methods of the class
    pub fn methods(&self) -> &[&'static MethodInfo] {
        let raw = self.raw();
        let methods = raw.methods;
        if !methods.is_null() {
            unsafe { slice::from_raw_parts(methods as _, raw.method_count as _) }
        } else {
            &[]
        }
    }

    /// Fields of the class
    pub fn fields(&self) -> FieldInfoSlice<'_> {
        let raw = self.raw();
        let fields = raw.fields;
        if !fields.is_null() {
            unsafe { slice::from_raw_parts(fields as _, raw.field_count as _) }
        } else {
            &[]
        }
    }

    /// Properties of the class
    pub fn properties(&self) -> PropertyInfoSlice<'_> {
        let raw = self.raw();
        let properties = raw.properties;
        if !properties.is_null() {
            unsafe { slice::from_raw_parts(properties.cast(), raw.property_count as _) }
        } else {
            &[]
        }
    }

    /// Parent of the class, if it inherits from any
    pub fn parent(&self) -> Option<&Self> {
        unsafe { Self::wrap_ptr(self.raw().parent) }
    }

    /// Iterator over the class hierarchy, starting with the class itself
    pub fn hierarchy(&self) -> Hierarchy<'_> {
        Hierarchy {
            current: Some(self),
        }
    }

    /// Interfaces this class implements
    pub fn implemented_interfaces(&self) -> &[&Self] {
        let raw = self.raw();
        let interfaces = raw.implementedInterfaces;
        if !interfaces.is_null() {
            unsafe { slice::from_raw_parts(interfaces as _, raw.interfaces_count as _) }
        } else {
            &[]
        }
    }

    /// Nested types of the class
    pub fn nested_types(&self) -> &[&Self] {
        let raw = self.raw();
        unsafe { slice::from_raw_parts(raw.nestedTypes as _, raw.nested_type_count as _) }
    }

    /// Whether the class is assignable from `other`
    pub fn is_assignable_from(&self, other: &Self) -> bool {
        // optimize
        if self == other {
            return true;
        }

        unsafe { raw::class_is_assignable_from(self.raw(), other.raw()) }
    }

    /// Whether this class represents a C# interface
    pub fn is_interface(&self) -> bool {
        self.raw().flags & raw::TYPE_ATTRIBUTE_INTERFACE != 0
    }

    /// This class' vtable, indexed by [`MethodInfo::slot`]
    fn vtable(&self) -> &[raw::VirtualInvokeData] {
        let raw = self.raw();
        unsafe { raw.vtable.as_slice(raw.vtable_count as _) }
    }

    /// Offsets of each interface this class implements into [`Self::vtable`]
    fn interface_offsets(&self) -> &[raw::Il2CppRuntimeInterfaceOffsetPair] {
        let raw = self.raw();
        let offsets = raw.interfaceOffsets;
        if !offsets.is_null() {
            unsafe { slice::from_raw_parts(offsets, raw.interface_offsets_count as _) }
        } else {
            &[]
        }
    }

    /// [`Il2CppType`] of `this` for the class
    pub fn this_arg_ty(&self) -> &Il2CppType {
        unsafe { Il2CppType::wrap(&self.raw().this_arg) }
    }

    /// [`Il2CppType`] of byval arguments for the class
    pub fn byval_arg_ty(&self) -> &Il2CppType {
        unsafe { Il2CppType::wrap(&self.raw().byval_arg) }
    }

    /// [`Il2CppType`] of the class
    pub fn ty(&self) -> &Il2CppType {
        unsafe { Il2CppType::wrap(raw::class_get_type(self.raw())) }
    }
}

/// Iterator over a class hierarchy
#[derive(Debug)]
pub struct Hierarchy<'a> {
    current: Option<&'a Il2CppClass>,
}

unsafe impl WrapRaw for Il2CppClass {
    type Raw = raw::Il2CppClass;
}

impl<'a> Iterator for Hierarchy<'a> {
    type Item = &'a Il2CppClass;

    fn next(&mut self) -> Option<Self::Item> {
        match self.current {
            Some(c) => {
                self.current = c.parent();
                Some(c)
            }
            None => None,
        }
    }
}

impl fmt::Debug for Il2CppClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let namespace = self.namespace();
        let name = self.name();
        f.debug_struct("Il2CppClass")
            .field("namespace", &namespace)
            .field("name", &name)
            .finish()
    }
}

impl fmt::Display for Il2CppClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let namespace = &*self.namespace();
        let name = &*self.name();
        match namespace {
            "" => f.write_str(name),
            _ => write!(f, "{}.{}", namespace, name),
        }
    }
}

impl PartialEq for Il2CppClass {
    fn eq(&self, other: &Self) -> bool {
        ptr::eq(self, other)
    }
}

impl<'a> From<&'a Il2CppType> for &'a Il2CppClass {
    fn from(ty: &'a Il2CppType) -> Self {
        ty.class()
    }
}

/// How closely a candidate method's parameters match the arguments an
/// overload is being resolved against - see [`param_distance`] and
/// [`method_weight`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Closeness {
    /// Every parameter's class is identical to the corresponding argument's
    /// class.
    Exact,
    /// At least one parameter only matched via assignability rather than an
    /// identical class - lower is a closer match. Can go negative (see
    /// [`param_distance`]).
    Convertible(i32),
}

/// Distance between a method parameter's declared class and the class of
/// the argument being passed for it - ports beatsaber-hook's
/// `param_distance` (`find.cpp`) verbatim, quirks included, since
/// [`Self::resolve_method`] uses it to rank overloads exactly the way
/// upstream does.
///
/// Lower is closer. `0` (well, [`Closeness::Exact`]) means the classes are
/// identical. Otherwise this walks `passed_class`'s assignability chain up
/// to `method_class` (one point of distance per step, breaking early if it
/// stops being assignable), then adjusts for shared implemented interfaces.
/// Interfaces are strongly preferred against other interfaces and strongly
/// avoided when a concrete `method_class` was expected but an interface was
/// passed (`1000` - `find.cpp` calls this out as "avoid" via a large
/// constant, not a hard rejection).
fn param_distance(method_class: &Il2CppClass, passed_class: &Il2CppClass) -> Closeness {
    if method_class == passed_class {
        return Closeness::Exact;
    }

    let mut distance = 0;

    let is_method_iface = method_class.is_interface();
    let is_passed_iface = passed_class.is_interface();
    if is_passed_iface && !is_method_iface {
        return Closeness::Convertible(1000);
    }
    if is_method_iface {
        distance += 5;
    }

    // Walk up `passed_class`'s parent chain towards `method_class`, one
    // point of distance per step.
    let mut passed = passed_class;
    while passed != method_class {
        if !method_class.is_assignable_from(passed) {
            break;
        }
        match passed.parent() {
            Some(parent) => passed = parent,
            None => break,
        }
        distance += 1;
    }

    // Mirrors `find.cpp`'s `param_distance` exactly: `passed` here is
    // whatever the walk above left it as (not necessarily the original
    // `passed_class`), and the two interface lists are compared as if
    // sorted by address - a merge-style intersection that only actually
    // finds shared entries if both classes' `implementedInterfaces` arrays
    // happen to list them in the same (address) order.
    let method_ifaces = method_class.implemented_interfaces();
    let passed_ifaces = passed.implemented_interfaces();
    let mut mi = 0;
    let mut pi = 0;
    while mi < method_ifaces.len() && pi < passed_ifaces.len() {
        let m = ptr::from_ref(method_ifaces[mi]);
        let p = ptr::from_ref(passed_ifaces[pi]);
        match m.cmp(&p) {
            std::cmp::Ordering::Less => mi += 1,
            std::cmp::Ordering::Greater => pi += 1,
            std::cmp::Ordering::Equal => {
                mi += 1;
                pi += 1;
                distance -= 1;
            }
        }
    }

    Closeness::Convertible(distance)
}

/// Aggregate [`Closeness`] of a candidate method against a full argument
/// list - [`Closeness::Exact`] only if every parameter is
/// [`Closeness::Exact`], otherwise the sum of every parameter's distance
/// (treating an exact parameter as contributing `0`). Mirrors the
/// `by_types` branch of beatsaber-hook's `find_method` switching on
/// `param_match`'s result.
///
/// Only meaningful for a candidate that's already passed the crate's own
/// (assignability-based) [`Arguments::matches`] check - this doesn't
/// independently re-verify convertibility, only ranks already-approved
/// candidates.
fn method_weight<const N: usize>(
    mi: &MethodInfo,
    arg_classes: &[&'static Il2CppClass; N],
) -> Closeness {
    let mut weight = 0;
    let mut exact = true;

    for (param, &arg_class) in mi.parameters().iter().zip(arg_classes) {
        match param_distance(param.ty().class(), arg_class) {
            Closeness::Exact => {}
            Closeness::Convertible(d) => {
                exact = false;
                weight += d;
            }
        }
    }

    if exact {
        Closeness::Exact
    } else {
        Closeness::Convertible(weight)
    }
}

/// No matching method were found
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FindMethodParameters {
    pub ty_name: String,
    pub method_name: String,
    pub parameters: Vec<String>,
}

/// Possible errors when looking up a method
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq, Hash)]
pub enum FindMethodError {
    /// No matching method was found
    #[error("no matching methods found for {0}")]
    None(FindMethodParameters),

    /// Multiple matching methods were found
    #[error("multiple matching methods found. {0:?}")]
    Many(Vec<FindMethodParameters>),
}

#[cfg(feature = "cache")]
mod cache {
    use std::any::TypeId;
    use std::borrow::{Borrow, Cow};
    use std::cell::RefCell;
    use std::collections::HashMap;

    #[derive(PartialEq, Eq, Hash)]
    pub(super) struct ClassCacheKey<'a> {
        pub(super) namespace: Cow<'a, str>,
        pub(super) name: Cow<'a, str>,
    }

    #[derive(PartialEq, Eq, Hash)]
    pub(super) struct StaticClassCacheKey(ClassCacheKey<'static>);

    impl<'a> From<ClassCacheKey<'a>> for StaticClassCacheKey {
        fn from(ClassCacheKey { namespace, name }: ClassCacheKey<'a>) -> Self {
            let namespace = namespace.into_owned().into();
            let name = name.into_owned().into();
            Self(ClassCacheKey { namespace, name })
        }
    }

    impl<'a> Borrow<ClassCacheKey<'a>> for StaticClassCacheKey {
        fn borrow(&self) -> &ClassCacheKey<'a> {
            &self.0
        }
    }

    #[derive(PartialEq, Eq, Hash)]
    pub(super) struct MethodCacheKey<'a> {
        pub(super) class: ClassCacheKey<'a>,
        pub(super) name: Cow<'a, str>,
        pub(super) ty: TypeId,
    }

    #[derive(PartialEq, Eq, Hash)]
    pub(super) struct StaticMethodCacheKey(MethodCacheKey<'static>);

    impl<'a> From<MethodCacheKey<'a>> for StaticMethodCacheKey {
        fn from(MethodCacheKey { class, name, ty }: MethodCacheKey<'a>) -> Self {
            let class = StaticClassCacheKey::from(class).0;
            let name = name.into_owned().into();
            Self(MethodCacheKey { class, name, ty })
        }
    }

    impl<'a> Borrow<MethodCacheKey<'a>> for StaticMethodCacheKey {
        fn borrow(&self) -> &MethodCacheKey<'a> {
            &self.0
        }
    }

    thread_local! {
        pub(super) static CLASS_CACHE: RefCell<HashMap<StaticClassCacheKey, &'static super::Il2CppClass>> = Default::default();
        pub(super) static METHOD_CACHE: RefCell<HashMap<StaticMethodCacheKey, &'static super::MethodInfo>> = Default::default();
    }
}

impl Display for FindMethodParameters {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}.{}({})",
            self.ty_name,
            self.method_name,
            self.parameters.join(", ")
        )
    }
}

// These tests build hand-populated raw structs (leaked for a `'static`
// lifetime) rather than loading a real il2cpp binary
#[cfg(test)]
mod tests {
    use std::mem;

    use super::*;

    fn leak<T>(value: T) -> &'static T {
        Box::leak(Box::new(value))
    }

    fn leak_slice<T>(items: Vec<T>) -> &'static [T] {
        Box::leak(items.into_boxed_slice())
    }

    fn fake_method(name: &'static CStr, slot: u16) -> &'static MethodInfo {
        let mut raw: raw::MethodInfo = unsafe { mem::zeroed() };
        raw.name = name.as_ptr();
        raw.slot = slot;
        unsafe { MethodInfo::wrap_ptr(leak(raw)) }.unwrap()
    }

    fn fake_property(name: &'static CStr) -> raw::PropertyInfo {
        let mut raw: raw::PropertyInfo = unsafe { mem::zeroed() };
        raw.name = name.as_ptr();
        raw
    }

    fn fake_interface_offset(
        interface: &'static Il2CppClass,
        offset: i32,
    ) -> raw::Il2CppRuntimeInterfaceOffsetPair {
        raw::Il2CppRuntimeInterfaceOffsetPair {
            interfaceType: interface.raw() as *const raw::Il2CppClass as *mut raw::Il2CppClass,
            offset,
        }
    }

    fn vtable_entry(method: &'static MethodInfo) -> raw::VirtualInvokeData {
        let mut entry: raw::VirtualInvokeData = unsafe { mem::zeroed() };
        entry.method = method.raw() as *const raw::MethodInfo;
        entry
    }

    fn zero_vtable_entry() -> raw::VirtualInvokeData {
        unsafe { mem::zeroed() }
    }

    /// Builds a leaked fake class with the given metadata, including a
    /// trailing vtable spliced directly after the `Il2CppClass` header -
    /// `Il2CppClass::vtable` is a C flexible array member (zero-sized on
    /// the Rust side), so a `#[repr(C)]` wrapper struct with a fixed-size
    /// trailing array reproduces the same layout real il2cpp allocates.
    fn fake_class_with_vtable<const N: usize>(
        flags: u32,
        parent: Option<&'static Il2CppClass>,
        methods: &'static [*const raw::MethodInfo],
        properties: &'static [raw::PropertyInfo],
        interface_offsets: &'static [raw::Il2CppRuntimeInterfaceOffsetPair],
        vtable: [raw::VirtualInvokeData; N],
    ) -> &'static Il2CppClass {
        #[repr(C)]
        struct WithVtable<const N: usize> {
            class: raw::Il2CppClass,
            vtable: [raw::VirtualInvokeData; N],
        }

        let mut class: raw::Il2CppClass = unsafe { mem::zeroed() };
        class.flags = flags;
        class.parent = match parent {
            Some(p) => p.raw() as *const raw::Il2CppClass as *mut raw::Il2CppClass,
            None => ptr::null_mut(),
        };
        class.methods = methods.as_ptr().cast_mut();
        class.method_count = methods.len() as u16;
        class.properties = properties.as_ptr();
        class.property_count = properties.len() as u16;
        class.interfaceOffsets = interface_offsets.as_ptr().cast_mut();
        class.interface_offsets_count = interface_offsets.len() as u16;
        class.vtable_count = N as u16;

        let leaked = leak(WithVtable { class, vtable });
        unsafe { Il2CppClass::wrap_ptr(&leaked.class) }.unwrap()
    }

    fn fake_class(flags: u32) -> &'static Il2CppClass {
        fake_class_with_vtable(flags, None, &[], &[], &[], [])
    }

    #[test]
    fn is_interface_reflects_type_attribute_interface_flag() {
        assert!(!fake_class(0).is_interface());
        assert!(fake_class(raw::TYPE_ATTRIBUTE_INTERFACE).is_interface());
    }

    #[test]
    fn properties_empty_when_class_has_none() {
        let class = fake_class(0);
        assert!(class.properties().is_empty());
        assert!(class.find_property("Missing").is_none());
    }

    #[test]
    fn find_property_walks_hierarchy() {
        let base_props = leak_slice(vec![fake_property(c"Health")]);
        let base = fake_class_with_vtable(0, None, &[], base_props, &[], []);
        let derived = fake_class_with_vtable(0, Some(base), &[], &[], &[], []);

        assert_eq!(derived.find_property("Health").unwrap().name(), "Health");
        assert!(derived.find_property("Missing").is_none());
    }

    #[test]
    fn find_method_by_slot_does_not_walk_hierarchy() {
        let base_methods = leak_slice(vec![fake_method(c"BaseOnly", 5).raw() as *const _]);
        let base = fake_class_with_vtable(0, None, base_methods, &[], &[], []);

        let derived_methods = leak_slice(vec![fake_method(c"Derived", 2).raw() as *const _]);
        let derived = fake_class_with_vtable(0, Some(base), derived_methods, &[], &[], []);

        assert_eq!(derived.find_method_by_slot(2).unwrap().name(), "Derived");
        // Slot 5 only exists on the parent - `find_method_by_slot` must
        // NOT walk the hierarchy the way `find_method`/`find_field` do.
        assert!(derived.find_method_by_slot(5).is_none());
        assert_eq!(base.find_method_by_slot(5).unwrap().name(), "BaseOnly");
    }

    #[test]
    fn find_method_by_vtable_non_interface_direct_match() {
        let m0 = fake_method(c"M0", 0);
        let m1 = fake_method(c"M1", 1);
        let declaring = fake_class(0);
        let class =
            fake_class_with_vtable(0, None, &[], &[], &[], [vtable_entry(m0), vtable_entry(m1)]);

        let found = class.find_method_by_vtable(declaring, 1).unwrap();
        assert_eq!(found.name(), "M1");
    }

    #[test]
    fn find_method_by_vtable_non_interface_falls_back_on_slot_mismatch() {
        // Simulates an abstract method: the vtable entry at slot 0 points
        // at a `MethodInfo` whose own `slot` doesn't match, so resolution
        // should fall back to a direct slot search over `self`'s methods.
        let stale = fake_method(c"Stale", 99);
        let real = fake_method(c"RealImpl", 0);

        let declaring = fake_class(0);
        let methods = leak_slice(vec![real.raw() as *const _]);
        let class = fake_class_with_vtable(0, None, methods, &[], &[], [vtable_entry(stale)]);

        let found = class.find_method_by_vtable(declaring, 0).unwrap();
        assert_eq!(found.name(), "RealImpl");
    }

    #[test]
    fn find_method_by_vtable_interface_uses_interface_offsets() {
        let iface = fake_class(raw::TYPE_ATTRIBUTE_INTERFACE);

        let impl0 = fake_method(c"Impl0", 10);
        let impl1 = fake_method(c"Impl1", 11);
        // The interface's methods are spliced in starting at vtable index
        // 3 - slots 0..3 belong to the concrete class's own methods.
        let offsets = leak_slice(vec![fake_interface_offset(iface, 3)]);
        let vtable = [
            zero_vtable_entry(),
            zero_vtable_entry(),
            zero_vtable_entry(),
            vtable_entry(impl0),
            vtable_entry(impl1),
        ];
        let class = fake_class_with_vtable(0, None, &[], &[], offsets, vtable);

        let found = class.find_method_by_vtable(iface, 1).unwrap();
        assert_eq!(found.name(), "Impl1");
    }

    #[test]
    fn find_method_by_vtable_falls_back_to_own_methods_when_self_is_the_interface() {
        let other_iface = fake_class(raw::TYPE_ATTRIBUTE_INTERFACE);

        let m0 = fake_method(c"M0", 0);
        let m1 = fake_method(c"M1", 1);
        let methods = leak_slice(vec![m0.raw() as *const _, m1.raw() as *const _]);

        // `self` IS the interface, and doesn't implement `other_iface`, so
        // resolution should fall back to indexing `self.methods()`
        // directly by slot rather than going through the (nonexistent for
        // an interface) vtable splice.
        let self_iface =
            fake_class_with_vtable(raw::TYPE_ATTRIBUTE_INTERFACE, None, methods, &[], &[], []);

        let found = self_iface.find_method_by_vtable(other_iface, 1).unwrap();
        assert_eq!(found.name(), "M1");
    }

    #[test]
    fn find_method_by_vtable_returns_none_when_not_implemented() {
        let iface = fake_class(raw::TYPE_ATTRIBUTE_INTERFACE);
        let class = fake_class(0); // implements nothing

        assert!(class.find_method_by_vtable(iface, 0).is_none());
    }

    // `param_distance` only has two branches reachable without a live
    // il2cpp runtime: the identity fast path, and the "passed an interface
    // where a concrete class was expected" early return - every other path
    // calls `Il2CppClass::is_assignable_from`, which (once the classes
    // differ) always falls through to the real `class_is_assignable_from`
    // FFI function. Ranking overloads end to end
    // (`method_weight`/`resolve_method`/`find_method`) also needs
    // `Il2CppType::class()`, which always calls `class_from_il2cpp_type` -
    // there's no fake-struct path around either. Both need a live,
    // initialized runtime to exercise safely, the same boundary
    // `tests/gc_alloc.rs` documents hitting for GC allocation.

    #[test]
    fn param_distance_is_exact_for_identical_classes() {
        let class = fake_class(0);
        assert_eq!(param_distance(class, class), Closeness::Exact);
    }

    #[test]
    fn param_distance_penalizes_passing_an_interface_for_a_concrete_parameter() {
        let method_param = fake_class(0); // concrete, not an interface
        let passed = fake_class(raw::TYPE_ATTRIBUTE_INTERFACE);

        assert_eq!(
            param_distance(method_param, passed),
            Closeness::Convertible(1000)
        );
    }
}
