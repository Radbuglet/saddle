#![no_std]

use core::{any::type_name, hint::black_box};

#[doc(hidden)]
pub mod scope_macro_internals {
    pub fn leak_zst<'a, T: 'a>(value: T) -> &'a mut T {
        // Ensure that the value is a ZST
        assert_eq!(core::mem::size_of::<T>(), 0);

        // Ensure that its destructor doesn't run
        core::mem::forget(value);

        unsafe { core::ptr::NonNull::<T>::dangling().as_mut() }
    }

    pub use {
        crate::{scope, Scope},
        core::mem::drop,
        partial_scope::partial_shadow,
    };
}

#[macro_export]
macro_rules! scope {
	(
		$from:expr => $to:ident;
		$($body:tt)*
	) => {
		let __scope_internal_to_token = {
			use $crate::scope_macro_internals::Scope as _;
			$crate::scope_macro_internals::scope!(InlineBlock);

			let to: &mut InlineBlock = $from.decl_call::<InlineBlock>();
			to
		};

		$crate::scope_macro_internals::partial_shadow! {
			$to;
			let $to = __scope_internal_to_token;
			$($body)*
			$crate::scope_macro_internals::drop($to);
		}
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

		impl $crate::Scope for $name {
			fn new<'a>() -> &'a mut Self {
				::std::boxed::Box::leak(::std::boxed::Box::new(Self { _private: () }))
			}
		}
	)*};
}

pub trait Scope: 'static + Sized {
    fn new<'a>() -> &'a mut Self;

    fn decl_dep_ref<T: 'static>(&self) {
        black_box(type_name::<SaddleInternalV1DeclForDepRef<Self, T>>());
    }

    fn decl_dep_mut<T: 'static>(&self) {
        black_box(type_name::<SaddleInternalV1DeclForDepMut<Self, T>>());
    }

    fn decl_call<G: Scope>(&mut self) -> &mut G {
        black_box(type_name::<SaddleInternalV1DeclForCall<Self, G>>());

        G::new()
    }
}

struct SaddleInternalV1DeclForDepRef<F, T>(F, T);
struct SaddleInternalV1DeclForDepMut<F, T>(F, T);
struct SaddleInternalV1DeclForCall<F, G>(F, G);
