use saddle::{scope, Scope};

fn main() {
    scope! { Root }

    whee(Root::new());
}

fn whee(c: &mut impl Scope) {
    scope! { c:
        woo(c);
        c.decl_dep_ref::<u32>();
        c.decl_dep_ref::<i32>();
        Whee::default().do_something(c.decl_call());
    }
}

fn woo(c: &mut impl Scope) {
    scope! { c:
        woz(c);
        c.decl_dep_ref::<u32>();
    }
}

fn woz(c: &mut impl Scope) {
    scope! { c:
        borrows_i32_mut(c);
    }
}

fn borrows_i32_mut(c: &impl Scope) {
    c.decl_dep_mut::<i32>();
}

scope! { pub WheeCx }

#[derive(Default)]
pub struct Whee {}

impl Whee {
    pub fn do_something(&mut self, c: &mut WheeCx) {
        c.decl_dep_mut::<i32>();
        self.do_something_else(c);
    }

    pub fn do_something_else(&mut self, c: &mut WheeCx) {
        c.decl_dep_mut::<i32>();
    }
}
