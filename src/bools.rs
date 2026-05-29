use core::{
    any::Any,
    fmt::{Debug, Display},
    hash::Hash,
    marker::PhantomData,
    mem::ManuallyDrop,
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

/// The only type that implements [`Bool`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Cond<const COND: bool>;

impl<const COND: bool> Display for Cond<COND> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
pub unsafe trait SwitchStorage {
    /// The type this stores when true.
    type OnTrue;
    /// The type this stores when false.
    type OnFalse;
    /// Whether this is true or false.
    type Condition: Bool;
    /// The type this maps to.
    type Mapped<T, F>: SwitchStorage<Condition = Self::Condition, OnFalse = F, OnTrue = T>;
}

// SAFETY: Ensured by `S`'s impl safety.
unsafe impl<'a, S: SwitchStorage> SwitchStorage for &'a S {
    type OnTrue = &'a S::OnTrue;
    type OnFalse = &'a S::OnFalse;
    type Condition = S::Condition;
    type Mapped<T, F> = S::Mapped<T, F>;
}

// SAFETY: Ensured by `S`'s impl safety.
unsafe impl<'a, S: SwitchStorage> SwitchStorage for &'a mut S {
    type OnTrue = &'a mut S::OnTrue;
    type OnFalse = &'a mut S::OnFalse;
    type Condition = S::Condition;
    type Mapped<T, F> = S::Mapped<T, F>;
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
    type Mapped<T2, F2> = SwitchCell<B, T2, F2>;
}

/// This [`SwitchStorage`] stores `T` and `F` in a repr(C) union.
///
/// That means if `T` is a `u32` and `F` is a `u128`, and `B` is [`True`], the size of this is still 16 bytes!
/// The advantage is that the compiler can see that this is only ever `T` or `F`, so it has no trouble with auto traits.
/// For example, if `T` and `F` both implement [`Copy`], this does too.
/// See also [`SwitchCell`], which has the opposite trade offs.
#[repr(C)]
pub union SwitchUnion<B: Bool, T, F = ()> {
    marker: PhantomData<B>,
    on_true: ManuallyDrop<T>,
    on_false: ManuallyDrop<F>,
}

// SAFETY: Is repr(C); `ManuallyDrop` is completely transparent.
unsafe impl<B: Bool, T, F> SwitchStorage for SwitchUnion<B, T, F> {
    type OnTrue = T;
    type OnFalse = F;
    type Condition = B;
    type Mapped<T2, F2> = SwitchUnion<B, T2, F2>;
}

impl<B: Bool, T, F> Drop for SwitchUnion<B, T, F> {
    #[inline(always)]
    fn drop(&mut self) {
        // SAFETY: Is being dropped and is correct.
        unsafe {
            if B::VALUE {
                ManuallyDrop::drop(&mut self.on_true);
            } else {
                ManuallyDrop::drop(&mut self.on_false);
            }
        }
    }
}

impl<T, F> SwitchUnion<True, T, F> {
    /// Creates a new value of this union.
    #[inline(always)]
    pub fn new(inner: T) -> Self {
        Self {
            on_true: ManuallyDrop::new(inner),
        }
    }

    /// Gets the inner value of this union.
    #[inline(always)]
    pub fn into_inner(self) -> T {
        unsafe { bool_macro_help::into_true(self) }
    }
}

impl<T, F> SwitchUnion<False, T, F> {
    /// Creates a new value of this union.
    #[inline(always)]
    pub fn new(inner: F) -> Self {
        Self {
            on_false: ManuallyDrop::new(inner),
        }
    }

    /// Gets the inner value of this union.
    #[inline(always)]
    pub fn into_inner(self) -> F {
        unsafe { bool_macro_help::into_false(self) }
    }
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
    const unsafe fn transmute_unchecked<Src, Dst>(src: Src) -> Dst {
        let src = ManuallyDrop::new(src);
        // SAFETY: Ensured by caller. The `ManuallyDrop` is transparent.
        // The original `src` is forgotten, so the returned value has ownership.
        unsafe { core::mem::transmute_copy::<ManuallyDrop<Src>, Dst>(&src) }
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
        unsafe { transmute_unchecked(storage) }
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
        unsafe { transmute_unchecked(storage) }
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
///     true => a + 1,
///     false => (a + 1.0) as u32,
/// });
/// assert_eq!(incremented, 6);
/// ```
///
/// Notice that `a` is in parentheses and is the name of the inner `u32` and `f32` in the `true` and `false` cases respectively.
///
/// You can use more than one [`SwitchStorage`]s as long as they share the same condition type.
/// Optionally, each switch storage variable can be declared inline by following it with an `= *some expression*`.
/// Note that [`SwitchStorage`] is implemented for references as well.
///
/// ```
/// # use type_switch::*;
/// let a = SwitchCell::<True, u32, f32>(5);
/// let square = switch_match!(match (l = &a, r = &a) {
///     true => *l * *r,
///     false => (*l * *r) as u32,
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
///         true => a * b,
///         false => (a * b) as u32,
///     });
///     assert_eq!(product, 12);
/// }
/// ```
///
/// Note that, like a normal `match`, none of this is put behind it's own function.
/// This takes its own scope to prevent variable aliasing but stays in the same function where this is invoked.
/// That means the `?` operator, early returns, etc will all work as they would in a `match`.
#[macro_export]
macro_rules! switch_match {
    (match ( $first_i:ident  $(= $first_e:expr)? $(, $i:ident $(= $e:expr)? )* ) {true => $on_true:expr , false => $on_false:expr $(,)?}) => {
        {
            $(let $first_i = $first_e;)?
            #[allow(unused_assignments)]
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
}
