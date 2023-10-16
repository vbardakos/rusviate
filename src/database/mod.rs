mod components;
mod errors;


mod this {
    pub struct Foo {
        pub(crate) x: usize
    }
}

use this::Foo;

impl Foo {
    fn new(x: usize) -> Self { Foo { x }}
    pub fn this(&self) -> usize { self.x }
}

pub fn create_foo(x: usize) -> Foo {
    Foo::new(x)
}