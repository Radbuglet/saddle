# Saddle

> **Note:**
>
> This module is still a work-in-progress and this documentation may not reflect the current state
> of the project.

Saddle implements a flexible borrow linting mechanism to help validate the soundness of Rust `RefCell`s and other dynamically borrow-checked mechanisms. In particular, Saddle statically ensures that the user's program can never cause an invalid borrow across user-defined "scope" boundaries while giving the user complete flexibility to borrow data however they choose within a given scope.

```rust
use saddle::{scope, Scope};

fn main() {
    // This defines a new scope for our main function.
    scope!(MainScope);

    // And this defines a token representing the fact that our code is currently executing in the
    // main scope.
    let c = MainScope::new();

    // This call declares that our `MainScope` intends to call the scope `ExampleScope`, granting us
    // a token to call it. The type of the call target is inferred.
    example(c.decl_call());
}

scope!(ExampleScope);

fn example(c: &ExampleScope) {
    // This call declares that our `ExampleScope` intends to access an `i32`. This method has no
    // runtime behavior and is purely to introduce metadata into the binary for the validator to
    // read.
    c.decl_dep_ref::<i32>();

    // Borrows can be declared in other functions and they will still contribute to the scope they
    // were passed.
    borrows_i32(c);

    other_function(c.decl_call());
}

fn borrows_i32(c: &impl Scope) {
    c.decl_dep_ref::<i32>();
    // (borrow logic here)
}

scope!(OtherFunctionScope);

fn other_function(c: &OtherFunctionScope) {
    // This call declares that our `OtherFunctionScope` intends to access a `u32`.
    c.decl_dep_mut::<u32>();

    other_function_helper(c);
}

// Not every method is forced to introduce its own scope. Indeed, if we did introduce our own scope
// here, we would get a warning from the saddle validator.
fn other_function_helper(c: &OtherFunctionScope) {
    // This call declares that our `OtherFunctionScope` intends to access a `u32`.
    c.decl_dep_mut::<u32>();
}
```

A function can have several scopes inside it, which can be defined succinctly using an alternative form of the `scope!` macro. This may be necessary if the borrow of one component does not actually last the entire function duration and the function calls out to another scope depending on the component which is no longer being borrowed.

```rust
scope!(MySubScopedMethodScope);

fn my_sub_scoped_method(c: &MySubScopedMethodScope) {
    scope! { c => c;  // Reads as scope `c` is used to call a new scope, whose token we bind to `c`.
        c.decl_dep_ref::<u32>();
    }

    scope! { c: // This is an alternative way to say the same thing.
        // If we put this method call in the same scope as our borrow to `u32`, we would get a
        // warning from the saddle validator.
        depends_upon_u32(c);
    }
}

fn depends_upon_u32(c: &impl Scope) {
    // We can use `scope!` blocks to avoid having to name new public scopes for every new function.
    scope! { c:
        c.decl_dep_mut::<u32>();
    }
}
```

**TODO:** Document dynamic dispatch and interaction with generics.

As of the time of writing, Saddle borrow checking is performed by an external program called `cargo-saddle`. To invoke the checker, you can run:

```
cargo saddle check target/path/to/program
```

after installing it on your machine.
