#![no_std]

use core::{any::type_name, hint::black_box};

#[doc(hidden)]
pub mod scope_macro_internals {
    pub use {
        crate::{scope, Scope},
        core::{column, line, mem::drop},
        partial_scope::partial_shadow,
    };

    pub fn limit_lifetime<'a, T: ?Sized>(_limiter: &'a mut (), value: &'a T) -> &'a T {
        value
    }

    pub struct ScopeDisambiguator<T, const LINE: u32, const COLUMN: u32>(T);
}

#[macro_export]
macro_rules! scope {
    (
        $from:expr => $to:ident;
        $($body:tt)*
    ) => {
		let mut __lifetime_limiter = ();
        let __scope_internal_to_token = {
            use $crate::scope_macro_internals::Scope as _;

            $crate::scope_macro_internals::scope!(InlineBlock);

            $crate::scope_macro_internals::limit_lifetime::<InlineBlock>(
				&mut __lifetime_limiter,
				$from.decl_call::<InlineBlock>(),
			)
        };

        $crate::scope_macro_internals::partial_shadow! {
            $to;
            let $to = __scope_internal_to_token;
            $($body)*
        }
		$crate::scope_macro_internals::drop(__lifetime_limiter);
    };
    (
        $from_and_to:ident:
        $($body:tt)*
    ) => {
        $crate::scope_macro_internals::scope! {
            $from_and_to => $from_and_to;
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

            fn new<'a>() -> &'a Self {
                &Self { _private: () }
            }
        }
    )*};
}

pub trait Scope: 'static + Sized {
    type _InternalDisamb: Sized;

    fn new<'a>() -> &'a Self;

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

    fn decl_call<G: Scope>(&self) -> &G {
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
