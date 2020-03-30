use std::borrow::Cow;
use std::convert::TryFrom;
use std::ffi::{CStr, CString};
use std::fmt;
use std::ptr::NonNull;

use crate::core::parser::IncrementLinenoError;
use crate::sys;

/// Filename of the top eval context.
pub const TOP_FILENAME: &[u8] = b"(eval)";

pub struct State {
    context: NonNull<sys::mrbc_context>,
    stack: Vec<Context>,
}

impl fmt::Debug for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("parser::State")
            .field("context", &"non-null mrb_context")
            .field("stack", &self.stack)
            .finish()
    }
}

impl State {
    pub fn new(mrb: &mut sys::mrb_state) -> Option<Self> {
        let context = unsafe { sys::mrbc_context_new(mrb) };
        let mut context = NonNull::new(context)?;
        reset_context_filename(mrb, unsafe { context.as_mut() });
        Some(Self {
            context,
            stack: vec![],
        })
    }

    /// Used for moving a `State` out of the larger Artichoke State to
    /// work around Artichoke State being stored in a [`RefCell`].
    ///
    /// # Safety
    ///
    /// This function creates an uninitialized parser state. Calling methods on
    /// it is unlikely to be correct.
    pub(crate) unsafe fn uninit() -> Self {
        Self {
            context: NonNull::dangling(),
            stack: vec![],
        }
    }

    pub fn close(mut self, mrb: &mut sys::mrb_state) {
        unsafe {
            sys::mrbc_context_free(mrb, self.context.as_mut());
        }
    }

    pub fn context_mut(&mut self) -> &mut sys::mrbc_context {
        unsafe { self.context.as_mut() }
    }

    /// Reset line number to `1`.
    pub fn reset(&mut self, mrb: &mut sys::mrb_state) {
        unsafe {
            self.context.as_mut().lineno = 1;
        }
        self.stack.clear();
        reset_context_filename(mrb, unsafe { self.context.as_mut() });
    }

    /// Fetch the current line number from the parser state.
    #[must_use]
    pub fn fetch_lineno(&self) -> usize {
        usize::from(unsafe { self.context.as_ref() }.lineno)
    }

    /// Increment line number and return the new value.
    ///
    /// # Errors
    ///
    /// This function returns [`IncrementLinenoError`] if the increment results
    /// in an overflow of the internal parser line number counter.
    pub fn add_fetch_lineno(&mut self, val: usize) -> Result<usize, IncrementLinenoError> {
        let old = usize::from(unsafe { self.context.as_ref() }.lineno);
        let new = old
            .checked_add(val)
            .ok_or_else(|| IncrementLinenoError::Overflow(usize::from(u16::max_value())))?;
        let store = u16::try_from(new)
            .map_err(|_| IncrementLinenoError::Overflow(usize::from(u16::max_value())))?;
        unsafe {
            self.context.as_mut().lineno = store;
        }
        Ok(new)
    }

    /// Push a [`Context`] onto the stack.
    ///
    /// The supplied [`Context`] becomes the currently active context. This
    /// function modifies the parser state so subsequently `eval`ed code will
    /// use the current active `Context`.
    pub fn push_context(&mut self, mrb: &mut sys::mrb_state, context: Context) {
        let filename = context.filename_as_c_str();
        unsafe {
            sys::mrbc_filename(mrb, self.context.as_mut(), filename.as_ptr() as *const i8);
        }
        self.stack.push(context);
    }

    /// Removes the last element from the context stack and returns it, or
    /// `None` if the stack is empty.
    ///
    /// Calls to this function modify the parser state so subsequently `eval`ed
    /// code will use the current active [`Context`].
    pub fn pop_context(&mut self, mrb: &mut sys::mrb_state) -> Option<Context> {
        let context = self.stack.pop();
        if let Some(current) = self.stack.last() {
            let filename = current.filename_as_c_str();
            unsafe {
                sys::mrbc_filename(mrb, self.context.as_mut(), filename.as_ptr() as *const i8);
            }
        } else {
            reset_context_filename(mrb, unsafe { self.context.as_mut() });
        }
        context
    }

    /// Returns the last [`Context`], or `None` if the context stack is empty.
    #[must_use]
    pub fn peek_context(&self) -> Option<&Context> {
        self.stack.last()
    }
}

fn reset_context_filename(mrb: &mut sys::mrb_state, context: &mut sys::mrbc_context) {
    let frame = Context::root();
    let filename = frame.filename_as_c_str();
    unsafe {
        sys::mrbc_filename(mrb, context, filename.as_ptr() as *const i8);
    }
}

/// `Context` is used to manipulate the current filename on the parser.
///
/// Parser [`State`] maintains a stack of `Context`s and
/// [`eval`](crate::eval::Eval) calls XXX to set the `__FILE__` magic constant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Context {
    /// Value of the `__FILE__` magic constant that also appears in stack
    /// frames.
    filename: Cow<'static, [u8]>,
    /// FFI variant of `filename` field.
    filename_cstring: CString,
}

impl Context {
    /// Create a new [`Context`].
    pub fn new<T>(filename: T) -> Option<Self>
    where
        T: Into<Cow<'static, [u8]>>,
    {
        let filename = filename.into();
        let cstring = CString::new(filename.as_ref()).ok()?;
        Some(Self {
            filename,
            filename_cstring: cstring,
        })
    }

    /// Create a new [`Context`] without checking for NUL bytes in the filename.
    ///
    /// # Safety
    ///
    /// `filename` must not contain any NUL bytes. `filename` must not contain a
    /// trailing `NUL`.
    pub unsafe fn new_unchecked<T>(filename: T) -> Self
    where
        T: Into<Cow<'static, [u8]>>,
    {
        let filename = filename.into();
        let cstring = CString::from_vec_unchecked(filename.clone().into_owned());
        Self {
            filename,
            filename_cstring: cstring,
        }
    }

    /// Create a root, or default, [`Context`].
    ///
    /// The root context sets the `__FILE__` magic constant to "(eval)".
    #[must_use]
    pub fn root() -> Self {
        Self::default()
    }

    /// Filename of this `Context`.
    #[must_use]
    pub fn filename(&self) -> &[u8] {
        self.filename.as_ref()
    }

    /// FFI-safe NUL-terminated C String of this `Context`.
    ///
    /// This [`CStr`] is valid as long as this `Context` is not dropped.
    #[must_use]
    pub fn filename_as_c_str(&self) -> &CStr {
        self.filename_cstring.as_c_str()
    }
}

impl Default for Context {
    fn default() -> Self {
        // Safety:
        //
        // - The `TOP_FILENAME` constant is controlled by this module.
        // - The `TOP_FILENAME` constant does not contain NUL bytes.
        // - This behavior is enforced by a test in this module.
        unsafe { Self::new_unchecked(TOP_FILENAME) }
    }
}

#[cfg(test)]
mod context_test {
    #[test]
    fn top_filename_does_not_contain_nul_byte() {
        let contains_nul_byte = super::TOP_FILENAME.iter().copied().any(|b| b == b'\0');
        assert!(!contains_nul_byte);
    }
}