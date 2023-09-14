#![no_std]

use core::{any::type_name, hint::black_box};

#[doc(hidden)]
pub mod scope_macro_internals {
    use core::mem;

    pub use {
        crate::{scope, Scope},
        core::{column, line, mem::drop},
        partial_scope::partial_shadow,
    };

    pub trait BindScopeAsRef: Scope {
        fn __saddle_internal_bind_scope(&mut self) -> BoundScopeProof<'_, Self>;
    }

    impl<T: ?Sized + Scope> BindScopeAsRef for T {
        fn __saddle_internal_bind_scope(&mut self) -> BoundScopeProof<'_, Self> {
            BoundScopeProof(self)
        }
    }

    pub struct BoundScopeProof<'a, T: ?Sized>(&'a mut T);

    impl<'a, T: ?Sized> BoundScopeProof<'a, T> {
        pub fn unwrap(self) -> &'a mut T {
            self.0
        }
    }

    pub struct ScopeDisambiguator<T, const LINE: u32, const COLUMN: u32>(T);

    pub fn leak_zst<'a, T>(t: T) -> &'a mut T {
        assert_eq!(mem::size_of::<T>(), 0);
        mem::forget(t);
        unsafe { core::ptr::NonNull::<T>::dangling().as_mut() }
    }
}

#[macro_export]
macro_rules! scope {
    (
        $from:expr => $to:ident $(inherits $($grant_kw:ident $grant_ty:ty),*$(,)?)?;
        $($body:tt)*
    ) => {
        let __scope_internal_to_token = {
			let from = {
				use $crate::scope_macro_internals::BindScopeAsRef as _;
				$crate::scope_macro_internals::BoundScopeProof::unwrap($from.__saddle_internal_bind_scope())
			};

			$($($crate::scope_macro_internals::scope!(@__decl_dep from, $grant_kw $grant_ty);)*)?

            $crate::scope_macro_internals::scope!(InlineBlock);

            let to = $crate::scope_macro_internals::Scope::decl_call::<InlineBlock>(from);

			$($($crate::scope_macro_internals::scope!(@__decl_grant to, $grant_kw $grant_ty);)*)?

			to
        };

        $crate::scope_macro_internals::partial_shadow! {
            $to;
            let $to = __scope_internal_to_token;
            $($body)*
        }
    };
    (
        $from_and_to:ident $(inherits $($grant_kw:ident $grant_ty:ty),*$(,)?)?:
        $($body:tt)*
    ) => {
        $crate::scope_macro_internals::scope! {
            $from_and_to => $from_and_to $(inherits $($grant_kw $grant_ty),*)?;
            $($body)*
        }
    };
    (
        $(
            $(#[$attr:meta])*
            $vis:vis $name:ident
        );*
        $(;)?
    ) => {$(
        $(#[$attr])*
        $vis struct $name { _private: () }

        impl $crate::scope_macro_internals::Scope for $name {
			type _InternalDisamb = $crate::scope_macro_internals::ScopeDisambiguator<
				Self,
				{$crate::scope_macro_internals::line!()},
				{$crate::scope_macro_internals::column!()},
			>;

            fn new<'a>() -> &'a mut Self {
                $crate::scope_macro_internals::leak_zst(Self { _private: () })
            }
        }
    )*};
	(@__decl_dep $target:expr, ref $ty:ty) => {
		$crate::scope_macro_internals::Scope::decl_dep_ref::<$ty>($target);
	};
	(@__decl_dep $target:expr, mut $ty:ty) => {
		$crate::scope_macro_internals::Scope::decl_dep_mut::<$ty>($target);
	};
	(@__decl_grant $target:expr, ref $ty:ty) => {
		$crate::scope_macro_internals::Scope::decl_grant_ref::<$ty>($target);
	};
	(@__decl_grant $target:expr, mut $ty:ty) => {
		$crate::scope_macro_internals::Scope::decl_grant_mut::<$ty>($target);
	};
}

pub trait Scope: 'static + Sized {
    type _InternalDisamb: Sized;

    fn new<'a>() -> &'a mut Self;

    fn leak<'a>(&self) -> &'a mut Self {
        Self::new()
    }

    fn decl_dep_ref<T: 'static>(&self) {
        black_box(type_name::<
            SaddleInternalV1DeclForDepRef<Self::_InternalDisamb, T>,
        >());
    }

    fn decl_dep_mut<T: 'static>(&self) {
        black_box(type_name::<
            SaddleInternalV1DeclForDepMut<Self::_InternalDisamb, T>,
        >());
    }

    fn decl_grant_ref<T: 'static>(&self) {
        black_box(type_name::<
            SaddleInternalV1DeclForGrantRef<Self::_InternalDisamb, T>,
        >());
    }

    fn decl_grant_mut<T: 'static>(&self) {
        black_box(type_name::<
            SaddleInternalV1DeclForGrantMut<Self::_InternalDisamb, T>,
        >());
    }

    fn decl_call<G: Scope>(&mut self) -> &mut G {
        black_box(type_name::<
            SaddleInternalV1DeclForCall<Self::_InternalDisamb, G::_InternalDisamb>,
        >());

        G::new()
    }
}

struct SaddleInternalV1DeclForDepRef<F, T>(F, T);
struct SaddleInternalV1DeclForDepMut<F, T>(F, T);
struct SaddleInternalV1DeclForGrantRef<F, T>(F, T);
struct SaddleInternalV1DeclForGrantMut<F, T>(F, T);

struct SaddleInternalV1DeclForCall<F, G>(F, G);
