use core::{
    any::Any,
    fmt::{Debug, Display},
    hash::Hash,
    marker::PhantomData,
    mem::ManuallyDrop,
    panic::{RefUnwindSafe, UnwindSafe},
};

use crate::Sealed;

/// Represents a boolean value as a type.
///
/// # Safety
///
/// This type must behave as documented.
/// This is sealed, so it can only be implemented by this crate.
#[expect(private_bounds, reason = "There are limited possible implementations.")]
pub unsafe trait Bool:
    Sealed
    + Default
    + Clone
    + Copy
    + PartialEq
    + PartialOrd
    + Eq
    + Ord
    + Hash
    + Debug
    + Display
    + Send
    + Sync
    + Unpin
    + UnwindSafe
    + RefUnwindSafe
    + Any
    + 'static
{
    /// Either `true` or `false`.
    const VALUE: bool;
    /// The opposite of this.
    type Not: Bool;
    /// The "and" of this with `T`.
    type And<T: Bool>: Bool;
    /// The "or" of this with `T`.
    type Or<T: Bool>: Bool;
    /// The "xor" of this with `T`.
    type XOr<T: Bool>: Bool;
    /// This is `T` when true and `F` when false.
    type Either<T, F>;
}

/// This type is `T` when `B` is [`True`] and `F` when `B` is [`False`].
pub type Switch<B: Bool, T, F> = B::Either<T, F>;

/// This is equivalent to `T::VALUE` but is sometimes shorter to write.
/// Ex, this is preferable to `<T as Bool>::VALUE`.
#[inline(always)]
pub const fn bool_of<T: Bool>() -> bool {
    T::VALUE
}

/// The only type that implements [`Bool`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Cond<const COND: bool>;

impl<const COND: bool> Display for Cond<COND> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Display::fmt(&COND, f)
    }
}

impl<const COND: bool> Sealed for Cond<COND> {}

/// The type for `true`.
pub type True = Cond<true>;
/// The type for `false`.
pub type False = Cond<false>;

// SAFETY: Is correct.
unsafe impl Bool for True {
    const VALUE: bool = true;
    type Not = False;
    type And<T: Bool> = T;
    type Or<T: Bool> = True;
    type XOr<T: Bool> = T::Not;
    type Either<T, F> = T;
}

// SAFETY: Is correct.
unsafe impl Bool for False {
    const VALUE: bool = false;
    type Not = True;
    type And<T: Bool> = False;
    type Or<T: Bool> = T;
    type XOr<T: Bool> = T;
    type Either<T, F> = F;
}

/// `P && Q`
pub type And<P: Bool, Q: Bool> = P::And<Q>;
/// `P || Q`
pub type Or<P: Bool, Q: Bool> = P::Or<Q>;
/// `P ^ Q`
pub type XOr<P: Bool, Q: Bool> = P::XOr<Q>;
/// `!P`
pub type Not<P: Bool> = P::Not;

/// Allows doing "and" on any number of [`Bool`] types.
/// When there are no types, this is [`True`].
/// ```
/// # use type_switch::*;
/// let _x: all!(False, True, False,) = False::default();
/// ```
#[macro_export]
macro_rules! all {
    () => {
        $crate::True
    };
    ($B:ty $(,)?) => {
        $B
    };
    ($B1:ty, $($B:ty),+ $(,)?) => {
        <$B1 as $crate::Bool>::And< $crate::all!( $($B),+ ) >
    };
}

/// Allows doing "or" on any number of [`Bool`] types.
/// When there are no types, this is [`False`].
///
/// ```
/// # use type_switch::*;
/// let _x: any!(False, True, False,) = True::default();
/// ```
#[macro_export]
macro_rules! any {
    () => {
        $crate::False
    };
    ($B:ty $(,)?) => {
        $B
    };
    ($B1:ty, $($B:ty),+ $(,)?) => {
        <$B1 as $crate::Bool>::Or< $crate::any!( $($B),+ ) >
    };
}

#[cfg(test)]
mod bool_logic_is_correct {
    use crate::*;
    use static_assertions::*;

    assert_type_eq_all!(True, Or<True, True>, Or<False, True>, Or<True, False>, And<True, True>, XOr<False, True>, XOr<True, False>, all!(True, True), all!(True), all!(), any!(False, True), Not<False>, Not<Not<True>>);
    assert_type_eq_all!(False, Or<False, False>, And<False, True>, And<True, False>, Or<False, False>, XOr<False, False>, XOr<True, True>, all!(False, True), all!(False), any!(), Not<True>, Not<Not<False>>);
}

/// Represents a way to store a [`Switch`] while also tracking the switch condition.
/// This lets you write code that handles both the true and false cases without needing to know the condition's type.
///
/// This is implemented by [`SwitchCell`] and [`SwitchUnion`].
/// You should prefer [`SwitchCell`], except when you need auto traits.
///
/// # Safety
///
/// This type mist be transparent over [`Self::OnTrue`] when [`Self::Condition`] and [`Self::OnFalse`] otherwise.
/// For example, when true, it must be safe to transmute between [`Self::OnTrue`] and `Self`, and between `&'a Self::OnTrue` and `&'a Self`, etc.
pub unsafe trait SwitchStorage: Sized {
    /// The type this stores when true.
    type OnTrue;
    /// The type this stores when false.
    type OnFalse;
    /// Whether this is true or false.
    type Condition: Bool;

    /// Creates a new value of this [`Switch`].
    fn new(value: Switch<Self::Condition, Self::OnTrue, Self::OnFalse>) -> Self {
        // SAFETY: Ensured by implementer.
        unsafe { bool_macro_help::transmute_unchecked(value) }
    }

    /// Gets the inner [`Switch`] value.
    fn into_inner(self) -> Switch<Self::Condition, Self::OnTrue, Self::OnFalse> {
        // SAFETY: Ensured by implementer.
        unsafe { bool_macro_help::transmute_unchecked(self) }
    }
}

// SAFETY: Ensured by `S`'s impl safety.
unsafe impl<'a, S: SwitchStorage> SwitchStorage for &'a S {
    type OnTrue = &'a S::OnTrue;
    type OnFalse = &'a S::OnFalse;
    type Condition = S::Condition;
}

// SAFETY: Ensured by `S`'s impl safety.
unsafe impl<'a, S: SwitchStorage> SwitchStorage for &'a mut S {
    type OnTrue = &'a mut S::OnTrue;
    type OnFalse = &'a mut S::OnFalse;
    type Condition = S::Condition;
}

/// This [`SwitchStorage`] stores [`Switch`] directly and transparently.
///
/// That means if `T` is a `u32` and `F` is a `u128`, and `B` is [`True`], the size of this is only 4 bytes!
/// The disadvantage is that the compiler can not initially see that this is only ever `T` or `F`, so it has trouble with auto traits.
/// For example, even if `T` and `F` both implement [`Copy`], this type still can't.
/// See also [`SwitchUnion`], which has the opposite trade offs.
#[repr(transparent)]
pub struct SwitchCell<B: Bool, T, F = ()>(pub Switch<B, T, F>);

// SAFETY: Is transparent.
unsafe impl<B: Bool, T, F> SwitchStorage for SwitchCell<B, T, F> {
    type OnTrue = T;
    type OnFalse = F;
    type Condition = B;
}

/// This [`SwitchStorage`] stores `T` and `F` in a repr(C) union.
///
/// That means if `T` is a `u32` and `F` is a `u128`, and `B` is [`True`], the size of this is still 16 bytes!
/// The advantage is that the compiler can see that this is only ever `T` or `F`, so it has no trouble with auto traits.
/// For example, if `T` and `F` both implement [`Copy`], this does too.
/// See also [`SwitchCell`], which has the opposite trade offs.
#[repr(C)]
#[derive(Clone, Copy)]
pub union SwitchUnion<B: Bool, T: Copy, F: Copy = ()> {
    marker: PhantomData<B>,
    on_true: T,
    on_false: F,
}

// SAFETY: Is repr(C); `ManuallyDrop` is completely transparent.
unsafe impl<B: Bool, T: Copy, F: Copy> SwitchStorage for SwitchUnion<B, T, F> {
    type OnTrue = T;
    type OnFalse = F;
    type Condition = B;
}

/// This module contains helpers for various macros.
/// The macro interface is stable, but this module may change drastically between versions,
/// and those changes would not be considered a breaking change.
#[doc(hidden)]
pub mod bool_macro_help {
    use super::*;

    /// This version of transmute has no additional checks and must be used more carefully.
    /// This is useful when the compiler can not assume that the two types are of the same size.
    ///
    /// # Safety
    ///
    /// The `src` must be transmutable to `Dst`.
    #[inline(always)]
    pub(crate) const unsafe fn transmute_unchecked<Src, Dst>(src: Src) -> Dst {
        #[repr(C)]
        union Transmuter<Src, Dst> {
            src: ManuallyDrop<Src>,
            dst: ManuallyDrop<Dst>,
        }
        // SAFETY: Caller ensures they are transmutable and union is repr(C).
        // We can't use `transmute` or `transmute_copy` because we don't know their sizes.
        // We only know (from the caller) that the bytes of `Src` that overlap with `Dst` are a valid value of `Dst`,
        // and the remaining bytes of `Dst` may be left uninit for form a valid value.
        unsafe {
            let x = Transmuter {
                src: ManuallyDrop::new(src),
            };
            ManuallyDrop::into_inner(x.dst)
        }
    }

    #[inline(always)]
    pub fn condition_marker_value<T: Bool>(_condition: PhantomData<T>) -> bool {
        T::VALUE
    }

    #[inline(always)]
    pub fn switch_storage_condition_marker<T: SwitchStorage>(
        _storage: &T,
    ) -> PhantomData<T::Condition> {
        PhantomData
    }

    /// # Safety
    ///
    /// The `T` must be true.
    #[inline(always)]
    pub unsafe fn into_true<T: SwitchStorage>(storage: T) -> T::OnTrue {
        #[cfg(miri)]
        {
            assert!(T::Condition::VALUE);
        }
        // SAFETY: Ensured by caller and `T` impl safety.
        unsafe { transmute_unchecked(storage) }
    }

    /// # Safety
    ///
    /// The `T` must be true.
    #[inline(always)]
    pub unsafe fn from_true<T: SwitchStorage>(value: T::OnTrue) -> T {
        #[cfg(miri)]
        {
            assert!(T::Condition::VALUE);
        }
        // SAFETY: Ensured by caller and `T` impl safety.
        unsafe { transmute_unchecked(value) }
    }

    /// # Safety
    ///
    /// The `T` must be false.
    #[inline(always)]
    pub unsafe fn into_false<T: SwitchStorage>(storage: T) -> T::OnFalse {
        #[cfg(miri)]
        {
            assert!(!T::Condition::VALUE);
        }
        // SAFETY: Ensured by caller and `T` impl safety.
        unsafe { transmute_unchecked(storage) }
    }

    /// # Safety
    ///
    /// The `T` must be false.
    #[inline(always)]
    pub unsafe fn from_false<T: SwitchStorage>(value: T::OnFalse) -> T {
        #[cfg(miri)]
        {
            assert!(!T::Condition::VALUE);
        }
        // SAFETY: Ensured by caller and `T` impl safety.
        unsafe { transmute_unchecked(value) }
    }
}

/// This macro does a `match` on one or more [`SwitchStorage`]s with the same [`Bool`].
///
/// Here's a simple example:
///
/// ```
/// # use type_switch::*;
/// let a = SwitchCell::<True, u32, f32>(5);
/// let incremented = switch_match!(match (a) {
///     true => a + 1;
///     false => (a + 1.0) as u32;
/// });
/// assert_eq!(incremented, 6);
/// ```
///
/// Notice that `a` is in parentheses and is the name of the inner `u32` and `f32` in the `true` and `false` cases respectively.
/// Note also that you can't swap the order of the cases or add match guards or anything.
/// Also unlike `match`, the expressions must end with `;` instead of `,`, with the exception of the last case, which can be omitted.
/// This helps `rustfmt` and other tools properly parse the syntax without interfering with `rustc`'s ability to match the macro.
///
/// You can use more than one [`SwitchStorage`]s as long as they share the same condition type.
/// Optionally, each switch storage variable can be declared inline by following it with an `= *some expression*`.
/// Note that [`SwitchStorage`] is implemented for references as well.
///
/// ```
/// # use type_switch::*;
/// let a = SwitchCell::<True, u32, f32>(5);
/// let square = switch_match!(match (l = &a, r = &a) {
///     true => *l * *r;
///     false => (*l * *r) as u32
/// });
/// assert_eq!(square, 25);
/// ```
///
/// The compiler can't always verify that the [`Bool`] types are the same.
/// If you know they are, you can manually (and very unsafely) transmute them instead of using this macro.
///
/// ```compile_fail
/// # use type_switch::*;
/// fn uses_generic<B: Bool>() {
///     let a = SwitchCell::<False, u32, u64>(3);
///     let b = SwitchCell::<XOr<B, B>, u32, u64>(4);
///     let product = switch_match!(match (a, b) {
///         true => a * b;
///         false => (a * b) as u32
///     });
///     assert_eq!(product, 12);
/// }
/// ```
///
/// Note that, like a normal `match`, none of this is put behind it's own function.
/// This takes its own scope to prevent variable aliasing but stays in the same function where this is invoked.
/// That means the `?` operator, early returns, etc will all work as they would in a `match`.
///
/// In the case where the syntax for handling both cases happens to be the same, you only need to write one case.
/// For example:
///
/// ```
/// # use type_switch::*;
/// fn uses_generic<B: Bool>() {
///     let a = SwitchCell::<False, u32, u16>(3);
///     let b = SwitchCell::<False, u32, u16>(4);
///     let product = switch_match!(match (a, b) {
///         _ => (a * b) as u64
///     });
///     assert_eq!(product, 12);
/// }
/// ```
#[macro_export]
macro_rules! switch_match {
    (match ( $first_i:ident  $(= $first_e:expr)? $(, $i:ident $(= $e:expr)? )* $(,)? ) {true => $on_true:expr ; false => $on_false:expr $(;)? }) => {
        {
            $(let $first_i = $first_e;)?
            #[allow(unused_assignments)]
            #[allow(unused_mut)]
            let mut condition = $crate::bool_macro_help::switch_storage_condition_marker(&$first_i);
            $(
                 $(let $i = $e;)?
                #[allow(unused_assignments)]
                { condition = $crate::bool_macro_help::switch_storage_condition_marker(&$i); }
            )*
            let condition = $crate::bool_macro_help::condition_marker_value(condition);
            if condition {
                // SAFETY: Is true.
                let $first_i = unsafe {$crate::bool_macro_help::into_true($first_i)};
                $(
                    // SAFETY: Is true.
                    let $i = unsafe {$crate::bool_macro_help::into_true($i)};
                )*
                $on_true
            } else {
                // SAFETY: Is false.
                let $first_i = unsafe {$crate::bool_macro_help::into_false($first_i)};
                $(
                    // SAFETY: Is false.
                    let $i = unsafe {$crate::bool_macro_help::into_false($i)};
                )*
                $on_false
            }
        }
    };
    (match ( $first_i:ident  $(= $first_e:expr)? $(, $i:ident $(= $e:expr)? $(,)? )* ) { _ => $on_either:expr $(;)?}) => {
        $crate::switch_match!(match ($first_i $(= $first_e)? $(, $i $(= $e)? )* ) { true => $on_either ; false => $on_either })
    };
}

/// This macro is very similar to [`switch_match`] except that it can map into other types.
/// Read the docs for [`switch_match`] first if you haven't already.
///
/// Here's a basic example:
///
/// ```
/// # use type_switch::*;
/// let a = SwitchCell::<True, u32, f32>(5);
/// let square = switch_map!(match (a) -> (output: SwitchCell<True, u64, f64>) {
///     true => (a * a) as u64;
///     false => (a * a) as f64;
/// });
/// assert_eq!(square.0, 25);
/// ```
///
/// Notice that the input and output needs to be in parentheses and that the output needs to be given a concrete type.
/// Spelling out the type is annoying, but it is necessary to ensure the condition types match at compile time, ensuring safety.
///
/// Here's one more example:
///
/// ```
/// # use type_switch::*;
/// fn subtract_both_ways<B: Bool>(a: SwitchCell::<B, i32, i64>, b: SwitchCell::<B, i32, i64>) -> (SwitchCell<B, i32, i64>, SwitchCell<B, i32, i64>) {
///      switch_map!(match (a, b) -> (positive: SwitchCell<B, i32, i64>, negative: SwitchCell<B, i32, i64>) {
///          _ => (a - b, b - a)
///      })
/// }
/// let a = SwitchCell::<True, i32, i64>(5);
/// let b = SwitchCell::<True, i32, i64>(2);
/// let (positive, negative) = subtract_both_ways(a, b);
/// assert_eq!(positive.0, 3);
/// assert_eq!(negative.0, -3);
/// ```
#[macro_export]
macro_rules! switch_map {
    (match ( $first_i:ident $(= $first_e:expr)? $(, $i:ident $(= $e:expr)? )* $(,)? ) -> ( $($o:ident : $t:ty),+ $(,)? ) {true => $on_true:expr ; false => $on_false:expr $(;)? }) => {
        {
            $(let $first_i = $first_e;)?
            #[allow(unused_assignments)]
            let mut condition = $crate::bool_macro_help::switch_storage_condition_marker(&$first_i);
            #[allow(unused_parens)]
            $(
                 $(let $i = $e;)?
                #[allow(unused_assignments)]
                { condition = $crate::bool_macro_help::switch_storage_condition_marker(&$i); }
            )*
            $(
                #[allow(unused_assignments)]
                { condition = ::core::marker::PhantomData::<<$t as $crate::SwitchStorage>::Condition>; }
            )+
            let condition = $crate::bool_macro_help::condition_marker_value(condition);
            if condition {
                // SAFETY: Is true.
                let $first_i = unsafe {$crate::bool_macro_help::into_true($first_i)};
                $(
                    // SAFETY: Is true.
                    let $i = unsafe {$crate::bool_macro_help::into_true($i)};
                )*
                #[allow(unused_parens)]
                let ($($o),+) = $on_true;
                ($(
                    // SAFETY: Is true. The output type has the same condition as the inputs.
                    unsafe { $crate::bool_macro_help::from_true::<$t>($o) }
                ),+)
            } else {
                // SAFETY: Is false.
                let $first_i = unsafe {$crate::bool_macro_help::into_false($first_i)};
                $(
                    // SAFETY: Is false.
                    let $i = unsafe {$crate::bool_macro_help::into_false($i)};
                )*
                #[allow(unused_parens)]
                let ($($o),+) = $on_false;
                ($(
                    // SAFETY: Is false. The output type has the same condition as the inputs.
                    unsafe { $crate::bool_macro_help::from_false::<$t>($o) }
                ),+)
            }
        }
    };
    (match ( $first_i:ident  $(= $first_e:expr)? $(, $i:ident $(= $e:expr)? $(,)? )* ) -> ( $($o:ident : $t:ty),+ $(,)? ) { _ => $on_either:expr $(;)?}) => {
        $crate::switch_map!(match ($first_i $(= $first_e)? $(, $i $(= $e)? )* ) -> ( $( $o:$t ),+ ) { true => $on_either ; false => $on_either })
    };
}

/// This macro is very similar to [`switch_map`] except that it doesn't take any input.
/// Read the docs for [`switch_map`] first if you haven't already.
///
/// Here's a basic example:
///
/// ```
/// # use type_switch::*;
/// let a = 5u32;
/// let a = switch_new!(match -> (output: SwitchCell<True, u64, f64>) {
///     true => a as u64;
///     false => a as f64;
/// });
/// assert_eq!(a.0, 5);
/// ```
///
/// Notice that the output needs to be in parentheses and that the output needs to be given a concrete type.
/// Spelling out the type is annoying, but it is necessary to ensure the condition types match at compile time, ensuring safety.
///
/// Here's one more example:
///
/// ```
/// # use type_switch::*;
/// fn subtract_both_ways<B: Bool>(a: i16, b: i16) -> (SwitchCell<B, i32, i64>, SwitchCell<B, i32, i64>) {
///      switch_new!(match -> (positive: SwitchCell<B, i32, i64>, negative: SwitchCell<B, i32, i64>) {
///          _ => (From::from(a - b), From::from(b - a))
///      })
/// }
/// let (positive, negative) = subtract_both_ways::<True>(5, 2);
/// assert_eq!(positive.0, 3);
/// assert_eq!(negative.0, -3);
/// ```
#[macro_export]
macro_rules! switch_new {
    (match -> ( $first_i:ident : $first_t:ty $(, $i:ident : $t:ty )* $(,)? ) {true => $on_true:expr ; false => $on_false:expr $(;)? }) => {
        {
            #[allow(unused_mut)]
            #[allow(unused_assignments)]
            let mut condition = ::core::marker::PhantomData::<<$first_t as $crate::SwitchStorage>::Condition>;
            $(
                #[allow(unused_assignments)]
                { condition = ::core::marker::PhantomData::<<$t as $crate::SwitchStorage>::Condition>; }
            )*
            let condition = $crate::bool_macro_help::condition_marker_value(condition);
            if condition {
                #[allow(unused_parens)]
                let ($first_i $(, $i)*) = $on_true;
                (
                    // SAFETY: Is true. The output type has the same condition as the inputs.
                    unsafe { $crate::bool_macro_help::from_true::<$first_t>($first_i) }
                    $(
                        // SAFETY: Is true. The output type has the same condition as the inputs.
                        , unsafe { $crate::bool_macro_help::from_true::<$t>($i) }
                    )*
                )

            } else {
                #[allow(unused_parens)]
                let ($first_i $(, $i)*) = $on_false;
                (
                    // SAFETY: Is false. The output type has the same condition as the inputs.
                    unsafe { $crate::bool_macro_help::from_false::<$first_t>($first_i) }
                    $(
                        // SAFETY: Is false. The output type has the same condition as the inputs.
                        , unsafe { $crate::bool_macro_help::from_false::<$t>($i) }
                    )*
                )
            }
        }
    };
    (match -> ( $first_i:ident : $first_t:ty $(, $i:ident : $t:ty )* $(,)? ) { _ => $on_either:expr $(;)?}) => {
        $crate::switch_new!(match -> ( $first_i:$first_t $(, $i:$t )* ) { true => $on_either ; false => $on_either })
    };
}

impl<B: Bool, T: Debug, F: Debug> Debug for SwitchCell<B, T, F> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        switch_match!(match (this = self) {
            _ => Debug::fmt(this, f);
        })
    }
}

impl<B: Bool, T: Display, F: Display> Display for SwitchCell<B, T, F> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        switch_match!(match (this = self) {
            _ => Display::fmt(this, f);
        })
    }
}

impl<B: Bool, T: PartialEq, F: PartialEq> PartialEq for SwitchCell<B, T, F> {
    fn eq(&self, other: &Self) -> bool {
        switch_match!(match (this = self, other) {
            _ => PartialEq::eq(this, other);
        })
    }
}

impl<B: Bool, T: Eq, F: Eq> Eq for SwitchCell<B, T, F> {}

impl<B: Bool, T: PartialOrd, F: PartialOrd> PartialOrd for SwitchCell<B, T, F> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        switch_match!(match (this = self, other) {
            _ => PartialOrd::partial_cmp(this, other);
        })
    }
}

impl<B: Bool, T: Ord, F: Ord> Ord for SwitchCell<B, T, F> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        switch_match!(match (this = self, other) {
            _ => Ord::cmp(this, other);
        })
    }
}

impl<B: Bool, T: Hash, F: Hash> Hash for SwitchCell<B, T, F> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        switch_match!(match (this = self) {
            _ => Hash::hash(this, state);
        })
    }
}

// SAFETY: Is transparent.
unsafe impl<B: Bool, T: Send, F: Send> Send for SwitchCell<B, T, F> {}

// SAFETY: Is transparent.
unsafe impl<B: Bool, T: Sync, F: Sync> Sync for SwitchCell<B, T, F> {}

impl<B: Bool, T: Unpin, F: Unpin> Unpin for SwitchCell<B, T, F> {}

impl<B: Bool, T: UnwindSafe, F: UnwindSafe> UnwindSafe for SwitchCell<B, T, F> {}

impl<B: Bool, T: RefUnwindSafe, F: RefUnwindSafe> RefUnwindSafe for SwitchCell<B, T, F> {}

impl<B: Bool, T: Copy, F: Copy> Copy for SwitchCell<B, T, F> where Switch<B, T, F>: Copy {}

impl<B: Bool, T: Clone, F: Clone> Clone for SwitchCell<B, T, F> {
    fn clone(&self) -> Self {
        switch_map!(match (this = self) -> (output: Self) {
            _ => this.clone()
        })
    }
}

impl<B: Bool, T: Default, F: Default> Default for SwitchCell<B, T, F> {
    fn default() -> Self {
        switch_new!(match -> (output: Self) {
            _ => Default::default();
        })
    }
}

impl<B: Bool, T: Debug + Copy, F: Debug + Copy> Debug for SwitchUnion<B, T, F> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        switch_match!(match (this = self) {
            _ => Debug::fmt(this, f);
        })
    }
}

impl<B: Bool, T: Display + Copy, F: Display + Copy> Display for SwitchUnion<B, T, F> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        switch_match!(match (this = self) {
            _ => Display::fmt(this, f);
        })
    }
}

impl<B: Bool, T: PartialEq + Copy, F: PartialEq + Copy> PartialEq for SwitchUnion<B, T, F> {
    fn eq(&self, other: &Self) -> bool {
        switch_match!(match (this = self, other) {
            _ => PartialEq::eq(this, other);
        })
    }
}

impl<B: Bool, T: Eq + Copy, F: Eq + Copy> Eq for SwitchUnion<B, T, F> {}

impl<B: Bool, T: PartialOrd + Copy, F: PartialOrd + Copy> PartialOrd for SwitchUnion<B, T, F> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        switch_match!(match (this = self, other) {
            _ => PartialOrd::partial_cmp(this, other);
        })
    }
}

impl<B: Bool, T: Ord + Copy, F: Ord + Copy> Ord for SwitchUnion<B, T, F> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        switch_match!(match (this = self, other) {
            _ => Ord::cmp(this, other);
        })
    }
}

impl<B: Bool, T: Hash + Copy, F: Hash + Copy> Hash for SwitchUnion<B, T, F> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        switch_match!(match (this = self) {
            _ => Hash::hash(this, state);
        })
    }
}

impl<B: Bool, T: Default + Copy, F: Default + Copy> Default for SwitchUnion<B, T, F> {
    fn default() -> Self {
        switch_new!(match -> (output: Self) {
            _ => Default::default();
        })
    }
}

#[cfg(test)]
mod tests {
    use core::num::{NonZeroI16, NonZeroU32};

    use crate::*;

    fn lifecycle_union<B: Bool>() {
        let (a, mut b) = switch_new!(match -> (a: SwitchUnion<B, NonZeroU32, i16>, b: SwitchUnion<B, NonZeroU32, i16>) {
            true => (NonZeroU32::new(5).unwrap(), NonZeroU32::new(3).unwrap());
            false => (-2, 7);
        });
        let (x, y) = switch_map!(match (a, b = &mut b) -> (x: SwitchUnion<B, u32, NonZeroI16>, y: SwitchUnion<B, u32, NonZeroI16>) {
            true => {
                *b = b.saturating_add(1);
                (a.get() * b.get(), a.get() + b.get())
            };
            false => {
                *b = b.saturating_add(1);
                (NonZeroI16::new(a * *b).unwrap(), NonZeroI16::new(a + *b).unwrap())
            };
        });
        let truth = switch_match!(match (x, y, b) {
            true => {
                assert_eq!(x, 20);
                assert_eq!(y, 9);
                assert_eq!(b.get(), 4);
                true
            };
            false => {
                assert_eq!(x.get(), -16);
                assert_eq!(y.get(), 6);
                assert_eq!(b, 8);
                false
            }
        });
        assert_eq!(truth, B::VALUE);
    }

    fn lifecycle_cell<B: Bool>() {
        let (a, mut b) = switch_new!(match -> (a: SwitchCell<B, NonZeroU32, i16>, b: SwitchCell<B, NonZeroU32, i16>) {
            true => (NonZeroU32::new(5).unwrap(), NonZeroU32::new(3).unwrap());
            false => (-2, 7);
        });
        let (x, y) = switch_map!(match (a, b = &mut b) -> (x: SwitchCell<B, u32, NonZeroI16>, y: SwitchCell<B, u32, NonZeroI16>) {
            true => {
                *b = b.saturating_add(1);
                (a.get() * b.get(), a.get() + b.get())
            };
            false => {
                *b = b.saturating_add(1);
                (NonZeroI16::new(a * *b).unwrap(), NonZeroI16::new(a + *b).unwrap())
            };
        });
        let truth = switch_match!(match (x, y, b) {
            true => {
                assert_eq!(x, 20);
                assert_eq!(y, 9);
                assert_eq!(b.get(), 4);
                true
            };
            false => {
                assert_eq!(x.get(), -16);
                assert_eq!(y.get(), 6);
                assert_eq!(b, 8);
                false
            }
        });
        assert_eq!(truth, B::VALUE);
    }

    #[test]
    fn lifecycle_union_true() {
        lifecycle_union::<True>();
    }

    #[test]
    fn lifecycle_union_false() {
        lifecycle_union::<False>();
    }

    #[test]
    fn lifecycle_cell_true() {
        lifecycle_cell::<True>();
    }

    #[test]
    fn lifecycle_cell_false() {
        lifecycle_cell::<False>();
    }
}
