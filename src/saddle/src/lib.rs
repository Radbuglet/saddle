use core::{any::type_name, hint::black_box};

#[macro_export]
macro_rules! scope {
	(
		$from:expr => $to:ident;
		$($body:tt)*
	) => {
		let $to = {
			$crate::scope!(InlineBlock);

			let to: &mut InlineBlock = $from.decl_call();
			to
		};
		$($body)*
		::std::mem::drop($to);
	};
	(
		$from_and_to:ident:
		$($body:tt)*
	) => {
		$crate::scope! {
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
