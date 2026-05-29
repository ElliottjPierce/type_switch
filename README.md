# Type Switch

This crate provides a very useful, very niche utility: Type switches.
A type switch is a little like an enum, a little like a union, and a little like a unit type.
It is a type that can represent multiple states (like an enum) without any overhead (like a union) and only be constructed in one state (like a unit type).

Currently, this crate only has boolean type switches.
There is a [`Bool`] trait, which is either [`True`] or [`False`].
You can do all the typical logic gates with them like [`And`], [`XOr`], etc and group them with macros like [`all!`].
Then you can use the [`Switch<B, T, F>`] type alias to get `T` when `B` is true and `F` when it is false.
If you are working in a generic context, you might want a [`SwitchStorage`], which helps expose whether `B` is true or false when it is a generic or otherwise unknown type.
This crate provides two storages, [`SwitchCell`] and [`SwitchUnion`], which have different trade-offs.

## Use case 1: A faster enum

Say you have a an enum like a `Result<T, F>`.
In your particular case, you know at compile time that only one variant will ever be possible in this context.
But... you don't know which variant that is, so you still need to `match`, etc, even though the compiler can skip the match with enough inlining.
You can make this faster by using a union and `const IS_ERR: bool`, but the compiler *might* be wasting space if `F` is significantly larger than `T`.
Type switches to the rescue!
Use a `SwitchCell<IsErr, T, F>`, with a `IsErr: Bool` generic instead of `const IS_ERR: bool`.
This wastes no space, lets the compiler use bit niches, lets the compiler optimize out all the checking of `IsErr`, and lets you write those `match`s without any `unsafe`.
The only downside? You'll need to use [`switch_match!`] and other macros, and it'll probably take a little longer to compile (but you're using rust, so clearly that doesn't bother you).

## Use case 2: Typed constants

Let's say you have a trait:
```rust
trait UnsafeFromString {
    /// # Safety
    /// 
    /// The `from` must encode a valid value of `Self`.
    unsafe fn from_string_unchecked(from: &str) -> Self;
}
```

But some implementers, like `String`, can do this safely.
This crate lets you do that!

```rust
use type_switch::*;

/// # Safety
/// 
/// `IsAlwaysSafe` must be correct.
unsafe trait UnsafeFromString {
    type IsAlwaysSafe: Bool;
    
    /// # Safety
    /// 
    /// The `from` must encode a valid value of `Self`.
    unsafe fn from_string_unchecked(from: &str) -> Self;
}

fn from_string_safe<T: UnsafeFromString<IsAlwaysSafe = True>>(from: &str) -> T {
    // SAFETY: Ensured by `T`'s implementation.
    unsafe { T::from_string_unchecked(from) }
}
```

This looks pretty niche but is actually incredibly helpful for building fast, flexible interfaces that have minimal user-facing `unsafe`, especially for library devs.
If this is your only use case, see also the `typenum` crate.

## Use case 3: Simplifying interfaces

Consider the following trait:

```rust
trait Reader<T> {
    type Metadata;
    
    fn read(&self, index: usize) -> (&T, Self::Metadata);
}
```

Let's imagine that almost no `Reader` actually needs `Metadata`, and almost all just have `Metadata = ()`.
On it's own, this might be ok.
But say this trait is used everywhere, and constantly discarding the metadata is truly getting in the way, especially for types where you know already that `Metadata = ()`.
This crate can help!

```rust
use type_switch::*;

trait Reader<T> {
    type HasMetadata: Bool;
    type Metadata;
    
    fn read(&self, index: usize) -> Switch<Self::HasMetadata, (&T, Self::Metadata), &T>;
}
```

Tada! Now, without needing any new function names or anything, places that know `HasMetadata = False` can skip all that headache.
Other places where `HasMetadata` is unknown will need a little more work using a [`SwitchStorage`], probably [`SwitchCell`], but that's easy.
You can also make this even easier with a helper trait:

```rust ignore
use type_switch::*;

trait PureReader<T>: Reader<HasMetadata = False> {}

impl<R: Reader<T, HasMetadata = False>, T> PureReader<T> for R {}
```

This particular example is a bit contrived, but for library developers, depending on how much of this complexity spills over into user code, this kind of trade-off can be worth it.

## Use case 4: Guarding invariants

Take a look at this trait:

```rust
use type_switch::*;

trait SomeComplexStructure {
    type State;
    type CanWriteState: Bool;

    fn update_sate(&mut self) -> Switch<Self::CanWriteState, &mut Self::State, &Self::State>;
}
```

Maybe some implementers require strict invariants that prevent the state from being exposed mutably in safe context.
Arguably, a simple `type StateGuard: Deref<Target = Self::State>;` might be better here, but you might want to have stricter requirements or otherwise.
Anyway, this is a cool thing you can do.

There are probably lots of other ways you can abuse the type system with this crate, but these are a few of the more practical ones I've used.
Once const generics become more powerful in stable rust, they will still integrate with this crate because [`True`] and [`False`] are type aliases of [`Cond<const COND: bool>`].
