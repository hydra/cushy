use std::ops::Deref;
use std::sync::OnceLock;

use kludgine::app::winit::event::Modifiers;
use kludgine::app::winit::keyboard::ModifiersState;

/// Invokes the provided macro with a pattern that can be matched using this
/// `macro_rules!` expression: `$($type:ident $field:tt),+`, where `$type` is an
/// identifier to use for the generic parameter and `$field` is the field index
/// inside of the tuple.
macro_rules! impl_all_tuples {
    ($macro_name:ident) => {
        $macro_name!(T0 0);
        $macro_name!(T0 0, T1 1);
        $macro_name!(T0 0, T1 1, T2 2);
        $macro_name!(T0 0, T1 1, T2 2, T3 3);
        $macro_name!(T0 0, T1 1, T2 2, T3 3, T4 4);
        $macro_name!(T0 0, T1 1, T2 2, T3 3, T4 4, T5 5);
    }
}

/// Invokes a function with a clone of `self`.
pub trait WithClone: Sized {
    /// The type that results from cloning.
    type Cloned;

    /// Maps `with` with the results of cloning `self`.
    fn with_clone<R>(&self, with: impl FnOnce(Self::Cloned) -> R) -> R;
}

macro_rules! impl_with_clone {
    ($($name:ident $field:tt),+) => {
        impl<'a, $($name: Clone,)+> WithClone for ($(&'a $name,)+)
        {
            type Cloned = ($($name,)+);

            fn with_clone<R>(&self, with: impl FnOnce(Self::Cloned) -> R) -> R {
                with(($(self.$field.clone(),)+))
            }
        }
    };
}

impl<'a, T> WithClone for &'a T
where
    T: Clone,
{
    type Cloned = T;

    fn with_clone<R>(&self, with: impl FnOnce(Self::Cloned) -> R) -> R {
        with((*self).clone())
    }
}

impl_all_tuples!(impl_with_clone);

pub trait ModifiersExt {
    fn primary(&self) -> bool;
    fn word_select(&self) -> bool;
}

impl ModifiersExt for ModifiersState {
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    fn primary(&self) -> bool {
        self.super_key()
    }

    #[cfg(not(any(target_os = "macos", target_os = "ios")))]
    fn primary(&self) -> bool {
        self.control_key()
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    fn word_select(&self) -> bool {
        self.alt_key()
    }

    #[cfg(not(any(target_os = "macos", target_os = "ios")))]
    fn word_select(&self) -> bool {
        self.control_key()
    }
}

impl ModifiersExt for Modifiers {
    fn primary(&self) -> bool {
        self.state().primary()
    }

    fn word_select(&self) -> bool {
        self.state().word_select()
    }
}

pub struct Lazy<T> {
    init: fn() -> T,
    once: OnceLock<T>,
}

impl<T> Lazy<T> {
    pub const fn new(init: fn() -> T) -> Self {
        Self {
            init,
            once: OnceLock::new(),
        }
    }
}

impl<T> Deref for Lazy<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.once.get_or_init(self.init)
    }
}
