//! Types that implement `Value` can be represented in the Artichoke VM.

use alloc::vec::Vec;

use crate::convert::{TryConvert, TryConvertMut};
use crate::types::Ruby;

/// A boxed Ruby value owned by the interpreter.
///
/// `Value` is equivalent to an `RValue` in MRI or `mrb_value` in mruby.
pub trait Value {
    /// Concrete type for Artichoke interpreter.
    type Artichoke;

    /// Concrete type for arguments passed to [`funcall`](Value::funcall).
    type Arg;

    /// Concrete type for results from [`funcall`](Value::funcall).
    type Value: Value;

    /// Concrete type for blocks passed to [`funcall`](Value::funcall).
    type Block;

    /// Concrete error type for funcall errors.
    type Error;

    /// Call a method on this [`Value`] with arguments and an optional block.
    ///
    /// # Errors
    ///
    /// If an exception is raised on the interpreter, then an error is returned.
    ///
    /// If a [`TryConvert`] conversion fails, then an error is returned.
    fn funcall(
        &self,
        interp: &mut Self::Artichoke,
        func: &str,
        args: &[Self::Arg],
        block: Option<Self::Block>,
    ) -> Result<Self::Value, Self::Error>;

    /// Consume `self` and try to convert `self` to type `T` using a
    /// [`TryConvert`] conversion.
    ///
    /// # Errors
    ///
    /// If a [`TryConvert`] conversion fails, then an error is returned.
    fn try_into<T>(self, interp: &Self::Artichoke) -> Result<T, Self::Error>
    where
        Self: Sized,
        Self::Artichoke: TryConvert<Self, T, Error = Self::Error>,
    {
        interp.try_convert(self)
    }

    /// Consume `self` and try to convert `self` to type `T` using a
    /// [`TryConvertMut`] conversion.
    ///
    /// # Errors
    ///
    /// If a [`TryConvertMut`] conversion fails, then an error is returned.
    fn try_into_mut<T>(self, interp: &mut Self::Artichoke) -> Result<T, Self::Error>
    where
        Self: Sized,
        Self::Artichoke: TryConvertMut<Self, T, Error = Self::Error>,
    {
        interp.try_convert_mut(self)
    }

    /// Call `#freeze` on this [`Value`].
    ///
    /// # Errors
    ///
    /// If an exception is raised on the interpreter, then an error is returned.
    fn freeze(&mut self, interp: &mut Self::Artichoke) -> Result<(), Self::Error>;

    /// Call `#frozen?` on this [`Value`].
    fn is_frozen(&self, interp: &mut Self::Artichoke) -> bool;

    /// Whether `self` is `nil`
    fn is_nil(&self) -> bool;

    /// Whether `self` responds to a method.
    ///
    /// Equivalent to invoking `#respond_to?` on this [`Value`].
    ///
    /// # Errors
    ///
    /// If an exception is raised on the interpreter, then an error is returned.
    fn respond_to(&self, interp: &mut Self::Artichoke, method: &str) -> Result<bool, Self::Error>;

    /// Call `#inspect` on this [`Value`].
    ///
    /// This function can never fail.
    fn inspect(&self, interp: &mut Self::Artichoke) -> Vec<u8>;

    /// Call `#to_s` on this [`Value`].
    ///
    /// This function can never fail.
    fn to_s(&self, interp: &mut Self::Artichoke) -> Vec<u8>;

    /// Return this values [Rust-mapped type tag](Ruby).
    fn ruby_type(&self) -> Ruby;
}

/// Return a name for this value's type suitable for using in an `Exception`
/// message.
///
/// Some immediate types like `true`, `false`, and `nil` are shown by value
/// rather than by class.
///
/// This function suppresses all errors and returns an empty string on
/// error.
pub fn pretty_name<'a, V, T>(value: V, interp: &mut T) -> &'a str
where
    V: Copy + Value<Artichoke = T>,
    T: TryConvert<V, Option<bool>, Error = V::Error>
        + TryConvertMut<V, &'a str, Error = V::Error>
        + TryConvertMut<<<V as Value>::Value as Value>::Value, &'a str, Error = V::Error>,
    V::Value: Value<Artichoke = T>,
    <V::Value as Value>::Value: Value<Artichoke = T, Error = V::Error>,
{
    match value.try_into(interp) {
        Ok(Some(true)) => "true",
        Ok(Some(false)) => "false",
        Ok(None) => "nil",
        Err(_) if matches!(value.ruby_type(), Ruby::Data | Ruby::Object) => {
            if let Ok(class) = value.funcall(interp, "class", &[], None) {
                if let Ok(class) = class.funcall(interp, "name", &[], None) {
                    if let Ok(class) = class.try_into_mut(interp) {
                        return class;
                    }
                }
            }
            ""
        }
        Err(_) => value.ruby_type().class_name(),
    }
}
