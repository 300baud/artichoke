#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![warn(clippy::cargo)]
#![warn(clippy::needless_borrow)]
#![allow(clippy::option_if_let_else)]
#![cfg_attr(test, allow(clippy::non_ascii_literal))]
#![allow(renamed_and_removed_lints)]
#![allow(unknown_lints)]
#![warn(broken_intra_doc_links)]
// TODO: warn on missing docs once crate is API-complete.
// #![warn(missing_docs)]
#![warn(missing_debug_implementations)]
#![warn(missing_copy_implementations)]
#![warn(rust_2018_idioms)]
#![warn(trivial_casts, trivial_numeric_casts)]
#![warn(unused_qualifications)]
#![warn(variant_size_differences)]
// Enable feature callouts in generated documentation:
// https://doc.rust-lang.org/beta/unstable-book/language-features/doc-cfg.html
//
// This approach is borrowed from tokio.
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(docsrs, feature(doc_alias))]

//! A String object holds and manipulates an arbitrary sequence of bytes,
//! typically representing characters.

#![no_std]

extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

use alloc::boxed::Box;
use alloc::vec::{self, Vec};
use core::cmp::Ordering;
use core::convert::TryFrom;
use core::fmt::{self, Write};
use core::iter::{Cycle, Take};
use core::mem;
use core::slice::{self, SliceIndex};
use core::str;

use bstr::{ByteSlice, ByteVec};
#[doc(inline)]
#[cfg(feature = "casecmp")]
#[cfg_attr(feature = "docsrs", doc(cfg(feature = "casecmp")))]
pub use focaccia::CaseFold;

mod chars;
mod encoding;
mod eq;
mod impls;

pub use chars::Chars;
pub use encoding::{Encoding, InvalidEncodingError};

/// Immutable [`String`] byte slice iterator.
///
/// This struct is created by the [`iter`] method on a Spinoso [`String`]. See
/// its documentation for more.
///
/// # Examples
///
/// ```
/// # use spinoso_string::String;
/// let s = String::utf8(b"Artichoke Ruby".to_vec());
///
/// let mut checksum: u32 = 0;
/// for &byte in s.iter() {
///     checksum += byte as u32;
/// }
/// assert_eq!(checksum, 1372);
/// ```
///
/// [`String`]: crate::String
/// [`iter`]: crate::String::iter
#[derive(Debug, Clone)]
pub struct Iter<'a>(slice::Iter<'a, u8>);

impl<'a> Iter<'a> {
    /// Views the underlying data as a subslice of the original data.
    ///
    /// This has the same lifetime as the original slice, and so the iterator
    /// can continue to be used while this exists.
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_string::String;
    /// let s = String::utf8(b"Artichoke Ruby".to_vec());
    ///
    /// // Then, we get the iterator:
    /// let mut iter = s.iter();
    /// assert_eq!(iter.as_slice(), b"Artichoke Ruby");
    ///
    /// // Next, we move to the second element of the slice:
    /// iter.next();
    /// // Now `as_slice` returns "rtichoke Ruby":
    /// assert_eq!(iter.as_slice(), b"rtichoke Ruby");
    /// ```
    #[inline]
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        self.0.as_slice()
    }
}

/// Mutable [`String`] byte iterator.
///
/// This struct is created by the [`iter_mut`] method on a Spinoso [`String`].
/// See its documentation for more.
///
/// # Examples
///
/// ```
/// # use spinoso_string::String;
/// let mut s = String::utf8(b"Artichoke Ruby".to_vec());
///
/// for byte in s.iter_mut() {
///     *byte = b'z';
/// }
/// assert_eq!(s, "zzzzzzzzzzzzzz");
/// ```
///
/// [`String`]: crate::String
/// [`iter_mut`]: crate::String::iter_mut
#[derive(Debug)]
pub struct IterMut<'a>(slice::IterMut<'a, u8>);

impl<'a> IterMut<'a> {
    /// Views the underlying data as a subslice of the original data.
    ///
    /// To avoid creating &mut references that alias, this is forced to consume
    /// the iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_string::String;
    /// let mut s = String::utf8(b"Artichoke Ruby".to_vec());
    /// {
    ///     let mut iter = s.iter_mut();
    ///     iter.next();
    ///     assert_eq!(iter.into_slice(), b"rtichoke Ruby");
    /// }
    /// {
    ///     let mut iter = s.iter_mut();
    ///     *iter.next().unwrap() = b'a';
    ///     *iter.nth(9).unwrap() = b'r';
    /// }
    /// assert_eq!(s, &b"artichoke ruby"[..]);
    /// ```
    #[inline]
    #[must_use]
    pub fn into_slice(self) -> &'a mut [u8] {
        self.0.into_slice()
    }
}

/// An iterator that moves out of a string.
///
/// This struct is created by the `into_iter` method on `String` (provided by
/// the [`IntoIterator`] trait).
///
/// # Examples
///
/// ```
/// use spinoso_string::String;
///
/// let s = String::from("hello");
/// let iter: spinoso_string::IntoIter = s.into_iter();
/// ```
#[derive(Debug)]
pub struct IntoIter(vec::IntoIter<u8>);

impl IntoIter {
    /// Returns the remaining bytes of this iterator as a slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use spinoso_string::String;
    ///
    /// let s = String::from("abc");
    /// let mut into_iter = s.into_iter();
    /// assert_eq!(into_iter.as_slice(), &[b'a', b'b', b'c']);
    /// let _ = into_iter.next().unwrap();
    /// assert_eq!(into_iter.as_slice(), &[b'b', b'c']);
    /// ```
    #[inline]
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        self.0.as_slice()
    }

    /// Returns the remaining bytes of this iterator as a mutable slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use spinoso_string::String;
    ///
    /// let s = String::from("abc");
    /// let mut into_iter = s.into_iter();
    /// assert_eq!(into_iter.as_slice(), &[b'a', b'b', b'c']);
    /// into_iter.as_mut_slice()[2] = b'z';
    /// assert_eq!(into_iter.next(), Some(b'a'));
    /// assert_eq!(into_iter.next(), Some(b'b'));
    /// assert_eq!(into_iter.next(), Some(b'z'));
    /// assert_eq!(into_iter.next(), None);
    /// ```
    #[inline]
    #[must_use]
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        self.0.as_mut_slice()
    }
}

/// Immutable [`String`] byte iterator.
///
/// This struct is created by the [`bytes`] method on a Spinoso [`String`]. See
/// its documentation for more.
///
/// # Examples
///
/// ```
/// # use spinoso_string::String;
/// let s = String::utf8(b"Artichoke Ruby".to_vec());
///
/// let mut checksum: u32 = 0;
/// for byte in s.bytes() {
///     checksum += byte as u32;
/// }
/// assert_eq!(checksum, 1372);
/// ```
///
/// [`String`]: crate::String
/// [`bytes`]: crate::String::bytes
#[derive(Debug, Clone)]
pub struct Bytes<'a>(slice::Iter<'a, u8>);

impl<'a> From<&'a [u8]> for Bytes<'a> {
    #[inline]
    fn from(bytes: &'a [u8]) -> Self {
        Self(bytes.iter())
    }
}

impl<'a> Bytes<'a> {
    /// Views the underlying data as a subslice of the original data.
    ///
    /// This has the same lifetime as the original slice, and so the iterator
    /// can continue to be used while this exists.
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_string::String;
    /// let s = String::utf8(b"Artichoke Ruby".to_vec());
    ///
    /// // Then, we get the iterator:
    /// let mut iter = s.bytes();
    /// assert_eq!(iter.as_slice(), b"Artichoke Ruby");
    ///
    /// // Next, we move to the second element of the slice:
    /// iter.next();
    /// // Now `as_slice` returns "rtichoke Ruby":
    /// assert_eq!(iter.as_slice(), b"rtichoke Ruby");
    /// ```
    #[inline]
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        self.0.as_slice()
    }
}

/// An iterator that yields a byte string centered within a padding byte string.
///
/// This struct is created by the [`center`] method on a Spinoso [`String`]. See
/// its documentation for more.
///
/// # Examples
///
/// ```
/// use spinoso_string::String;
///
/// let s = String::from("hello");
///
/// assert_eq!(s.center(4, None).collect::<Vec<_>>(), b"hello");
/// assert_eq!(s.center(20, None).collect::<Vec<_>>(), b"       hello        ");
/// assert_eq!(s.center(20, Some(&b"123"[..])).collect::<Vec<_>>(), b"1231231hello12312312");
/// ```
///
/// This iterator is [encoding-aware]. [Conventionally UTF-8] strings are
/// iterated by UTF-8 byte sequences.
///
/// ```
/// use spinoso_string::String;
///
/// let s = String::from("💎");
///
/// assert_eq!(s.center(3, None).collect::<Vec<_>>(), " 💎 ".as_bytes());
/// ```
///
/// [`center`]: crate::String::center
/// [encoding-aware]: crate::Encoding
/// [Conventionally UTF-8]: crate::Encoding::Utf8
#[derive(Debug, Clone)]
pub struct Center<'a, 'b> {
    left: Take<Cycle<slice::Iter<'b, u8>>>,
    next: Option<&'a [u8]>,
    s: Chars<'a>,
    right: Take<Cycle<slice::Iter<'b, u8>>>,
}

impl<'a, 'b> Default for Center<'a, 'b> {
    #[inline]
    fn default() -> Self {
        Self::with_chars_width_and_padding(Chars::new(), 0, &[])
    }
}

impl<'a, 'b> Center<'a, 'b> {
    #[inline]
    #[must_use]
    pub(crate) fn with_chars_width_and_padding(s: Chars<'a>, padding_width: usize, padding: &'b [u8]) -> Self {
        let pre_pad = padding_width / 2;
        let post_pad = (padding_width + 1) / 2;
        let left = padding.iter().cycle().take(pre_pad);
        let right = padding.iter().cycle().take(post_pad);
        Self {
            left,
            next: None,
            s,
            right,
        }
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
enum CodePointRangeError {
    InvalidUtf8Codepoint(u32),
    OutOfRange(i64),
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct InvalidCodepointError(CodePointRangeError);

impl InvalidCodepointError {
    pub const EXCEPTION_TYPE: &'static str = "RangeError";

    #[inline]
    #[must_use]
    pub const fn invalid_utf8_codepoint(codepoint: u32) -> Self {
        Self(CodePointRangeError::InvalidUtf8Codepoint(codepoint))
    }

    #[inline]
    #[must_use]
    pub const fn codepoint_out_of_range(codepoint: i64) -> Self {
        Self(CodePointRangeError::OutOfRange(codepoint))
    }

    #[inline]
    #[must_use]
    pub const fn is_invalid_utf8(self) -> bool {
        matches!(self.0, CodePointRangeError::InvalidUtf8Codepoint(_))
    }

    #[inline]
    #[must_use]
    pub const fn is_out_of_range(self) -> bool {
        matches!(self.0, CodePointRangeError::OutOfRange(_))
    }

    #[inline]
    #[must_use]
    pub fn message(self) -> alloc::string::String {
        // The longest error message is 27 bytes + a hex-encoded codepoint
        // formatted as `0x...`.
        const MESSAGE_MAX_LENGTH: usize = 27 + 2 + mem::size_of::<u32>() * 2;
        let mut s = alloc::string::String::with_capacity(MESSAGE_MAX_LENGTH);
        // In practice, the errors from `write!` below are safe to ignore
        // because the `core::fmt::Write` impl for `String` will never panic
        // and these `String`s will never approach `isize::MAX` bytes.
        //
        // See the `core::fmt::Display` impl for `InvalidCodepointError`.
        let _ = write!(s, "{}", self);
        s
    }
}

impl fmt::Display for InvalidCodepointError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            CodePointRangeError::InvalidUtf8Codepoint(codepoint) => {
                write!(f, "invalid codepoint {:X} in UTF-8", codepoint)
            }
            CodePointRangeError::OutOfRange(codepoint) => write!(f, "{} out of char range", codepoint),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for InvalidCodepointError {}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum OrdError {
    /// The first character in a [conventionally UTF-8] `String` is an invalid
    /// UTF-8 byte sequence.
    ///
    /// [conventionally UTF-8]: Encoding::Utf8
    InvalidUtf8ByteSequence,
    /// The given `String` is empty and has no first character.
    EmptyString,
}

impl OrdError {
    /// `OrdError` corresponds to an [`ArgumentError`] Ruby exception.
    ///
    /// [`ArgumentError`]: https://ruby-doc.org/core-2.6.3/ArgumentError.html
    pub const EXCEPTION_TYPE: &'static str = "ArgumentError";

    /// Construct a new `OrdError` for an invalid UTF-8 byte sequence.
    ///
    /// Only [conventionally UTF-8] `String`s can generate this error.
    ///
    /// # Examples
    ///
    /// ```
    /// use spinoso_string::{OrdError, String};
    ///
    /// let s = String::utf8(b"\xFFabc".to_vec());
    /// assert_eq!(s.ord(), Err(OrdError::invalid_utf8_byte_sequence()));
    ///
    /// let s = String::binary(b"\xFFabc".to_vec());
    /// assert_eq!(s.ord(), Ok(0xFF));
    /// ```
    ///
    /// [conventionally UTF-8]: Encoding::Utf8
    #[inline]
    #[must_use]
    pub const fn invalid_utf8_byte_sequence() -> Self {
        Self::InvalidUtf8ByteSequence
    }

    /// Construct a new `OrdError` for an empty `String`.
    ///
    /// Empty `String`s have no first character. Empty `String`s with any
    /// encoding return this error.
    ///
    /// # Examples
    ///
    /// ```
    /// use spinoso_string::{OrdError, String};
    ///
    /// let s = String::utf8(b"\xFFabc".to_vec());
    /// assert_eq!(s.ord(), Err(OrdError::invalid_utf8_byte_sequence()));
    /// ```
    #[inline]
    #[must_use]
    pub const fn empty_string() -> Self {
        Self::EmptyString
    }

    /// Error message for this `OrdError`.
    ///
    /// This message is suitable for generating an [`ArgumentError`] exception
    /// from this `OrdError`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_string::OrdError;
    ///
    /// assert_eq!(OrdError::invalid_utf8_byte_sequence().message(), "invalid byte sequence in UTF-8");
    /// assert_eq!(OrdError::empty_string().message(), "empty string");
    /// ```
    ///
    /// [`ArgumentError`]: https://ruby-doc.org/core-2.6.3/ArgumentError.html
    #[inline]
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::InvalidUtf8ByteSequence => "invalid byte sequence in UTF-8",
            Self::EmptyString => "empty string",
        }
    }
}

impl fmt::Display for OrdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

#[cfg(feature = "std")]
impl std::error::Error for OrdError {}

#[derive(Default, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct String {
    buf: Vec<u8>,
    encoding: Encoding,
}

impl fmt::Debug for String {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("String")
            .field("buf", &self.buf.as_bstr())
            .field("encoding", &self.encoding)
            .finish()
    }
}

// Constructors
impl String {
    /// Constructs a new, empty `String`.
    ///
    /// The `String` is [conventionally UTF-8].
    ///
    /// The string will not allocate until bytes are pushed onto it.
    ///
    /// # Examples
    ///
    /// ```
    /// use spinoso_string::{Encoding, String};
    ///
    /// const S: String = String::new();
    /// assert_eq!(S.encoding(), Encoding::Utf8);
    /// ```
    ///
    /// [conventionally UTF-8]: crate::Encoding::Utf8
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        let buf = Vec::new();
        let encoding = Encoding::Utf8;
        Self { buf, encoding }
    }

    /// Constructs a new, empty `String` with the specified capacity.
    ///
    /// The `String` is [conventionally UTF-8].
    ///
    /// The string will be able to hold exactly `capacity` bytes without
    /// reallocating. If `capacity` is 0, the vector will not allocate.
    ///
    /// It is important to note that although the returned string has the
    /// capacity specified, the string will have a zero length. For an
    /// explanation of the difference between length and capacity, see
    /// *[Capacity and reallocation]*.
    ///
    /// # Examples
    ///
    /// Encoding, capacity, and length:
    ///
    /// ```
    /// use spinoso_string::{Encoding, String};
    ///
    /// let s = String::with_capacity(10);
    /// assert_eq!(s.encoding(), Encoding::Utf8);
    /// assert_eq!(s.capacity(), 10);
    /// assert_eq!(s.len(), 0);
    /// ```
    ///
    /// Allocation:
    ///
    /// ```
    /// use spinoso_string::{Encoding, String};
    ///
    /// let mut s = String::with_capacity(10);
    ///
    /// for ch in 'a'..='j' {
    ///     s.push_byte(ch as u8);
    /// }
    /// // 10 elements have been inserted without reallocating.
    /// assert_eq!(s.capacity(), 10);
    /// assert_eq!(s.len(), 10);
    /// ```
    ///
    /// [Capacity and reallocation]: https://doc.rust-lang.org/std/vec/struct.Vec.html#capacity-and-reallocation
    #[inline]
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        let buf = Vec::with_capacity(capacity);
        let encoding = Encoding::Utf8;
        Self { buf, encoding }
    }

    #[inline]
    #[must_use]
    pub fn with_capacity_and_encoding(capacity: usize, encoding: Encoding) -> Self {
        let buf = Vec::with_capacity(capacity);
        Self { buf, encoding }
    }

    #[inline]
    #[must_use]
    pub fn utf8(buf: Vec<u8>) -> Self {
        let encoding = Encoding::Utf8;
        Self { buf, encoding }
    }

    #[inline]
    #[must_use]
    pub fn ascii(buf: Vec<u8>) -> Self {
        let encoding = Encoding::Ascii;
        Self { buf, encoding }
    }

    #[inline]
    #[must_use]
    pub fn binary(buf: Vec<u8>) -> Self {
        let encoding = Encoding::Binary;
        Self { buf, encoding }
    }
}

// Core data structure manipulation
impl String {
    /// Returns the [`Encoding`] of this `String`.
    ///
    /// # Examples
    ///
    /// ```
    /// use spinoso_string::{Encoding, String};
    ///
    /// let s = String::utf8(b"xyz".to_vec());
    /// assert_eq!(s.encoding(), Encoding::Utf8);
    /// ```
    #[inline]
    #[must_use]
    pub const fn encoding(&self) -> Encoding {
        self.encoding
    }

    /// Extracts a slice containing the entire byte string.
    ///
    /// Equivalent to `&s[..]`.
    #[inline]
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        self.buf.as_slice()
    }

    /// Extracts a mutable slice containing the entire byte string.
    ///
    /// Equivalent to `&mut s[..]`.
    #[inline]
    #[must_use]
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        self.buf.as_mut_slice()
    }

    #[inline]
    #[must_use]
    pub fn as_ptr(&self) -> *const u8 {
        self.buf.as_ptr()
    }

    #[inline]
    #[must_use]
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.buf.as_mut_ptr()
    }

    /// Converts self into a vector without clones or allocation.
    ///
    /// This method consumes this `String` and returns its inner [`Vec<u8>`]
    /// buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use spinoso_string::String;
    ///
    /// let s = String::from("hello");
    /// let buf = s.into_vec();
    /// // `s` cannot be used anymore because it has been converted into `buf`.
    ///
    /// assert_eq!(buf, b"hello".to_vec());
    /// ```
    /// [`Vec<u8>`]: alloc::vec::Vec
    #[inline]
    #[must_use]
    pub fn into_vec(self) -> Vec<u8> {
        self.buf
    }

    /// Converts the vector into `Box<[u8]>`.
    ///
    /// Note that this will drop any excess capacity.
    ///
    /// # Examples
    ///
    /// ```
    /// use spinoso_string::String;
    ///
    /// let s = String::from("abc");
    /// let slice = s.into_boxed_slice();
    /// ```
    ///
    /// Any excess capacity is removed:
    ///
    /// ```
    /// use spinoso_string::String;
    ///
    /// let mut s = String::with_capacity(10);
    /// s.extend_from_slice(&[b'a', b'b', b'c']);
    ///
    /// assert_eq!(s.capacity(), 10);
    /// let slice = s.into_boxed_slice();
    /// assert_eq!(slice.into_vec().capacity(), 3);
    /// ```
    ///
    /// [`Box<u8>`]: alloc::boxed::Box
    #[inline]
    #[must_use]
    pub fn into_boxed_slice(self) -> Box<[u8]> {
        self.buf.into_boxed_slice()
    }

    /// Returns the number of bytes the string can hold without reallocating.
    ///
    /// # Examples
    ///
    /// ```
    /// use spinoso_string::String;
    ///
    /// let s = String::with_capacity(10);
    /// assert_eq!(s.capacity(), 10);
    /// ```
    #[inline]
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.buf.capacity()
    }

    /// Returns true if the vector contains no bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use spinoso_string::String;
    ///
    /// let mut s = String::new();
    /// assert!(s.is_empty());
    ///
    /// s.push_char('x');
    /// assert!(!s.is_empty());
    /// ```
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    /// Returns the number of bytes in the string, also referred to as its
    /// 'length' or 'bytesize'.
    ///
    /// See also [`bytesize`].
    ///
    /// # Examples
    ///
    /// ```
    /// use spinoso_string::String;
    ///
    /// let s = String::from("xyz");
    /// assert_eq!(s.len(), 3);
    /// ```
    ///
    /// [`bytesize`]: Self::bytesize
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.buf.len()
    }
}

// Core iterators
impl String {
    /// Returns an iterator over this string's underlying byte slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use spinoso_string::String;
    ///
    /// let s = String::from("abc");
    /// let mut iterator = s.iter();
    ///
    /// assert_eq!(iterator.next(), Some(&b'a'));
    /// assert_eq!(iterator.next(), Some(&b'b'));
    /// assert_eq!(iterator.next(), Some(&b'c'));
    /// assert_eq!(iterator.next(), None);
    /// ```
    #[inline]
    #[must_use]
    pub fn iter(&self) -> Iter<'_> {
        Iter(self.buf.iter())
    }

    /// Returns an iterator that allows modifiying this string's underlying byte
    /// slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use spinoso_string::String;
    ///
    /// let mut s = String::from("abc");
    ///
    /// for byte in s.iter_mut() {
    ///     *byte = b'x';
    /// }
    ///
    /// assert_eq!(s, "xxx");
    /// ```
    #[inline]
    #[must_use]
    pub fn iter_mut(&mut self) -> IterMut<'_> {
        IterMut(self.buf.iter_mut())
    }

    /// Returns an iterator over the bytes in this byte string.
    ///
    /// # Examples
    ///
    /// ```
    /// use spinoso_string::String;
    ///
    /// let s = String::utf8(b"foobar".to_vec());
    /// let bytes: Vec<u8> = s.bytes().collect();
    /// assert_eq!(bytes, s);
    /// ```
    #[inline]
    #[must_use]
    pub fn bytes(&self) -> Bytes<'_> {
        Bytes(self.buf.iter())
    }
}

// Memory management
impl String {
    /// Reserves capacity for at least `additional` more bytes to be inserted in
    /// the given `String`. The string may reserve more space to avoid frequent
    /// reallocations. After calling `reserve`, capacity will be greater than or
    /// equal to `self.len() + additional`. Does nothing if capacity is already
    /// sufficient.
    ///
    /// # Panics
    ///
    /// Panics if the new capacity exceeds [`isize::MAX`] bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use spinoso_string::String;
    ///
    /// let mut s = String::from("x");
    /// s.reserve(10);
    /// assert!(s.capacity() >= 11);
    /// ```
    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        self.buf.reserve(additional)
    }

    /// Reserves the minimum capacity for exactly `additional` more bytes to be
    /// inserted in the given `String`. After calling `reserve_exact`, capacity
    /// will be greater than or equal to `self.len() + additional`. Does nothing
    /// if the capacity is already sufficient.
    ///
    /// Note that the allocator may give the string more space than it requests.
    /// Therefore, capacity can not be relied upon to be precisely minimal.
    /// Prefer `reserve` if future insertions are expected.
    ///
    /// # Panics
    ///
    /// Panics if the new capacity overflows [`usize`].
    ///
    /// # Examples
    ///
    /// ```
    /// use spinoso_string::String;
    ///
    /// let mut s = String::from("x");
    /// s.reserve_exact(10);
    /// assert!(s.capacity() >= 11);
    /// ```
    #[inline]
    pub fn reserve_exact(&mut self, additional: usize) {
        self.buf.reserve_exact(additional)
    }

    /// Shrinks the capacity of the vector as much as possible.
    ///
    /// It will drop down as close as possible to the length but the allocator
    /// may still inform the string that there is space for a few more bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use spinoso_string::String;
    ///
    /// let mut s = String::with_capacity(10);
    /// s.extend_from_slice(b"abc");
    /// assert_eq!(s.capacity(), 10);
    /// s.shrink_to_fit();
    /// assert!(s.capacity() >= 3);
    /// ```
    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.buf.shrink_to_fit()
    }
}

// Indexing
impl String {
    /// Returns a reference to a byte or sub-byteslice depending on the type of
    /// index.
    ///
    /// - If given a position, returns a reference to the byte at that position
    ///   or [`None`] if out of bounds.
    /// - If given a range, returns the subslice corresponding to that range, or
    ///   [`None`] if out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use spinoso_string::String;
    ///
    /// let s = String::from("abc");
    /// assert_eq!(s.get(1), Some(&b'b'));
    /// assert_eq!(s.get(0..2), Some(&b"ab"[..]));
    /// assert_eq!(s.get(3), None);
    /// assert_eq!(s.get(0..4), None);
    /// ```
    #[inline]
    #[must_use]
    pub fn get<I>(&self, index: I) -> Option<&I::Output>
    where
        I: SliceIndex<[u8]>,
    {
        self.buf.get(index)
    }

    /// Returns a mutable reference to a byte or sub-byteslice depending on the
    /// type of index (see [`get`]) or [`None`] if the index is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use spinoso_string::String;
    ///
    /// let mut s = String::from("abc");
    ///
    /// if let Some(byte) = s.get_mut(1) {
    ///     *byte = b'x';
    /// }
    /// assert_eq!(s, "axc");
    /// ```
    ///
    /// [`get`]: Self::get
    #[inline]
    #[must_use]
    pub fn get_mut<I>(&mut self, index: I) -> Option<&mut I::Output>
    where
        I: SliceIndex<[u8]>,
    {
        self.buf.get_mut(index)
    }

    /// Returns a reference to a byte or sub-byteslice, without doing bounds
    /// checking.
    ///
    /// For a safe alternative see [`get`].
    ///
    /// # Safety
    ///
    /// Calling this method with an out-of-bounds index is *[undefined
    /// behavior]* even if the resulting reference is not used.
    ///
    /// # Examples
    ///
    /// ```
    /// use spinoso_string::String;
    ///
    /// let s = String::from("abc");
    ///
    /// unsafe {
    ///     assert_eq!(s.get_unchecked(1), &b'b');
    /// }
    /// ```
    ///
    /// [`get`]: Self::get
    /// [undefined behavior]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
    #[inline]
    #[must_use]
    pub unsafe fn get_unchecked<I>(&self, index: I) -> &I::Output
    where
        I: SliceIndex<[u8]>,
    {
        self.buf.get_unchecked(index)
    }

    /// Returns a mutable reference to a byte or sub-byteslice, without doing
    /// bounds checking.
    ///
    /// For a safe alternative see [`get_mut`].
    ///
    /// # Safety
    ///
    /// Calling this method with an out-of-bounds index is *[undefined
    /// behavior]* even if the resulting reference is not used.
    ///
    /// # Examples
    ///
    /// ```
    /// use spinoso_string::String;
    ///
    /// let mut s = String::from("abc");
    ///
    /// unsafe {
    ///     let byte = s.get_unchecked_mut(1);
    ///     *byte = b'x';
    /// }
    /// assert_eq!(s, "axc");
    /// ```
    ///
    /// [`get_mut`]: Self::get_mut
    /// [undefined behavior]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
    #[inline]
    #[must_use]
    pub unsafe fn get_unchecked_mut<I>(&mut self, index: I) -> &mut I::Output
    where
        I: SliceIndex<[u8]>,
    {
        self.buf.get_unchecked_mut(index)
    }
}

// Pushing and popping bytes, codepoints, and strings.
impl String {
    /// Appends a given byte onto the end of this `String`.
    ///
    /// The given byte is not required to be a valid byte given this `String`'s
    /// [encoding] because encodings are only conventional.
    ///
    /// # Examples
    ///
    /// ```
    /// use spinoso_string::String;
    ///
    /// let mut s = String::utf8(b"UTF-8?".to_vec());
    /// s.push_byte(0xFF);
    /// assert_eq!(s, &b"UTF-8?\xFF"[..]);
    /// ```
    ///
    /// [encoding]: crate::Encoding
    #[inline]
    pub fn push_byte(&mut self, byte: u8) {
        self.buf.push_byte(byte);
    }

    /// Try to append a given Unicode codepoint onto the end of this `String`.
    ///
    /// This API is encoding-aware. For [UTF-8] strings, the given integer is
    /// converted to a [`char`] before appending to this `String` using
    /// [`push_char`]. For [ASCII] and [binary] strings, the given integer is
    /// converted to a byte before appending to this `String` using
    /// [`push_byte`].
    ///
    /// This function can be used to implement the Ruby method [`String#<<`] for
    /// [`Integer`][ruby-integer] arguments.
    ///
    /// # Errors
    ///
    /// If this `String` is [conventionally UTF-8] and the given codepoint is
    /// not a valid [`char`], an error is returned.
    ///
    /// If this `String` has [ASCII] or [binary] encoding and the given
    /// codepoint is not a valid byte, an error is returned.
    ///
    /// # Examples
    ///
    /// For [UTF-8] strings, the given codepoint is converted to a Unicode scalar
    /// value before appending:
    ///
    /// ```
    /// use spinoso_string::String;
    ///
    /// # fn example() -> Result<(), spinoso_string::InvalidCodepointError> {
    /// let mut s = String::utf8(b"".to_vec());
    /// s.try_push_codepoint(b'a' as i64)?;
    /// assert_eq!(s, "a");
    /// assert!(s.try_push_codepoint(0xD83F).is_err());
    /// assert!(s.try_push_codepoint(-1).is_err());
    /// # Ok(())
    /// # }
    /// # example().unwrap();
    /// ```
    ///
    /// For [ASCII] and [binary] strings, the given codepoint must be a valid
    /// byte:
    ///
    /// ```
    /// use spinoso_string::String;
    ///
    /// # fn example() -> Result<(), spinoso_string::InvalidCodepointError> {
    /// let mut s = String::binary(b"".to_vec());
    /// s.try_push_codepoint(b'a' as i64)?;
    /// assert_eq!(s, "a");
    /// assert!(s.try_push_codepoint(1024).is_err());
    /// assert!(s.try_push_codepoint(-1).is_err());
    /// # Ok(())
    /// # }
    /// # example().unwrap();
    /// ```
    ///
    /// [UTF-8]: crate::Encoding::Utf8
    /// [ASCII]: crate::Encoding::Ascii
    /// [binary]: crate::Encoding::Binary
    /// [`push_char`]: Self::push_char
    /// [`push_byte`]: Self::push_byte
    /// [`String#<<`]: https://ruby-doc.org/core-2.6.3/String.html#method-i-3C-3C
    /// [ruby-integer]: https://ruby-doc.org/core-2.6.3/Integer.html
    /// [conventionally UTF-8]: crate::Encoding::Utf8
    #[inline]
    pub fn try_push_codepoint(&mut self, codepoint: i64) -> Result<(), InvalidCodepointError> {
        match self.encoding {
            Encoding::Utf8 => {
                let codepoint = if let Ok(codepoint) = u32::try_from(codepoint) {
                    codepoint
                } else {
                    return Err(InvalidCodepointError::codepoint_out_of_range(codepoint));
                };
                if let Ok(ch) = char::try_from(codepoint) {
                    self.buf.push_char(ch);
                    Ok(())
                } else {
                    Err(InvalidCodepointError::invalid_utf8_codepoint(codepoint))
                }
            }
            Encoding::Ascii | Encoding::Binary => {
                if let Ok(byte) = u8::try_from(codepoint) {
                    self.buf.push_byte(byte);
                    Ok(())
                } else {
                    Err(InvalidCodepointError::codepoint_out_of_range(codepoint))
                }
            }
        }
    }

    /// Appends a given [`char`] onto the end of this `String`.
    ///
    /// The given char is UTF-8 encoded and the UTF-8 bytes are appended to the
    /// end of this `String`.
    ///
    /// # Examples
    ///
    /// ```
    /// use spinoso_string::String;
    ///
    /// let mut s = String::from("<3");
    /// s.push_char('💎');
    /// assert_eq!(s, &b"<3\xF0\x9F\x92\x8E"[..]); // "<3💎"
    /// ```
    #[inline]
    pub fn push_char(&mut self, ch: char) {
        self.buf.push_char(ch);
    }

    /// Appends a given string slice onto the end of this `String`.
    ///
    /// # Examples
    ///
    /// ```
    /// use spinoso_string::String;
    ///
    /// let mut s = String::utf8(b"spinoso".to_vec());
    /// s.push_str("-string");
    /// assert_eq!(s, "spinoso-string");
    /// ```
    #[inline]
    pub fn push_str(&mut self, s: &str) {
        self.buf.push_str(s);
    }

    /// Copies and appends all bytes in a slice to the `String`.
    ///
    /// Iterates over the slice `other`, copies each element, and then appends
    /// it to this `String`. The other byte slice is traversed in-order.
    ///
    /// # Examples
    ///
    /// ```
    /// use spinoso_string::String;
    ///
    /// let mut s = String::from("a");
    /// s.extend_from_slice(&b"bc"[..]);
    /// assert_eq!(s, "abc");
    /// ```
    #[inline]
    pub fn extend_from_slice(&mut self, other: &[u8]) {
        self.buf.extend_from_slice(other);
    }
}

// Ruby APIs
impl String {
    /// Appends the given bytes to this `String`.
    ///
    /// See also [`Extend`].
    ///
    /// This function can be used to implement the Ruby method [`String#<<`] for
    /// [`String`][ruby-string] arguments.
    ///
    /// # Examples
    ///
    /// ```
    /// use spinoso_string::String;
    ///
    /// let mut s = String::ascii(b"abc".to_vec());
    /// s.concat(", easy as 123");
    /// assert_eq!(s, "abc, easy as 123");
    /// ```
    ///
    /// [`String#<<`]: https://ruby-doc.org/core-2.6.3/String.html#method-i-3C-3C
    /// [ruby-string]: https://ruby-doc.org/core-2.6.3/String.html
    #[inline]
    pub fn concat<T: AsRef<[u8]>>(&mut self, other: T) {
        self.buf.extend_from_slice(other.as_ref())
    }

    /// Returns true for a string which has only ASCII characters.
    ///
    /// ASCII is an encoding that defines 128 codepoints. A byte corresponds to
    /// an ASCII codepoint if and only if it is in the inclusive range
    /// `[0, 127]`.
    ///
    /// This function ignores this `String`'s [encoding].
    ///
    /// # Examples
    ///
    /// ```
    /// use spinoso_string::String;
    ///
    /// let s = String::utf8("abc".as_bytes().to_vec());
    /// assert!(s.is_ascii_only());
    /// let s = String::utf8("abc\u{6666}".as_bytes().to_vec());
    /// assert!(!s.is_ascii_only());
    /// ```
    ///
    /// [encoding]: crate::Encoding
    #[inline]
    #[must_use]
    pub fn is_ascii_only(&self) -> bool {
        self.buf.is_ascii()
    }

    /// Change the [encoding] of this `String` to [`Encoding::Binary`].
    ///
    /// This function can be used to implement the Ruby method [`String#b`].
    ///
    /// # Examples
    ///
    /// ```
    /// use spinoso_string::{Encoding, String};
    ///
    /// let mut s = String::utf8(b"xyz".to_vec());
    /// assert_eq!(s.encoding(), Encoding::Utf8);
    /// s.make_binary();
    /// assert_eq!(s.encoding(), Encoding::Binary);
    /// ```
    ///
    /// [encoding]: crate::Encoding
    /// [`String#b`]: https://ruby-doc.org/core-2.6.3/String.html#method-i-b
    #[inline]
    pub fn make_binary(&mut self) {
        self.encoding = Encoding::Binary;
    }

    /// Returns the length of this `String` in bytes.
    ///
    /// `bytesize` is an [`Encoding`]-oblivious API and is equivalent to
    /// [`String::len`].
    ///
    /// This function can be used to implement the Ruby method
    /// [`String#bytesize`].
    ///
    /// # Examples
    ///
    /// ```
    /// use spinoso_string::String;
    ///
    /// let s = String::utf8("💎".as_bytes().to_vec());
    /// assert_eq!(s.bytesize(), 4);
    /// assert_eq!(s.bytesize(), s.len());
    /// ```
    ///
    /// [`String#bytesize`]: https://ruby-doc.org/core-2.6.3/String.html#method-i-bytesize
    #[inline]
    #[must_use]
    pub fn bytesize(&self) -> usize {
        self.buf.len()
    }

    /// Modify this `String` to have the first character converted to uppercase
    /// and the remainder to lowercase.
    #[inline]
    #[allow(clippy::match_same_arms)]
    pub fn make_capitalized(&mut self) {
        match self.encoding {
            Encoding::Ascii | Encoding::Binary => {
                if let Some((head, tail)) = self.buf.split_first_mut() {
                    head.make_ascii_uppercase();
                    tail.make_ascii_lowercase();
                }
            }
            Encoding::Utf8 => {
                // This allocation assumes that in the common case, capitalizing
                // and lowercasing `char`s do not change the length of the
                // `String`.
                let mut replacement = Vec::with_capacity(self.buf.len());
                let mut bytes = self.buf.as_slice();
                match bstr::decode_utf8(bytes) {
                    (Some(ch), size) => {
                        // Converting a UTF-8 character to uppercase may yield
                        // multiple codepoints.
                        for ch in ch.to_uppercase() {
                            replacement.push_char(ch)
                        }
                        bytes = &bytes[size..];
                    }
                    (None, size) if size == 0 => return,
                    (None, size) => {
                        let (substring, remainder) = bytes.split_at(size);
                        replacement.extend_from_slice(substring);
                        bytes = remainder;
                    }
                }
                while !bytes.is_empty() {
                    let (ch, size) = bstr::decode_utf8(bytes);
                    if let Some(ch) = ch {
                        // Converting a UTF-8 character to lowercase may yield
                        // multiple codepoints.
                        for ch in ch.to_lowercase() {
                            replacement.push_char(ch);
                        }
                        bytes = &bytes[size..];
                    } else {
                        let (substring, remainder) = bytes.split_at(size);
                        replacement.extend_from_slice(substring);
                        bytes = remainder;
                    }
                }
                self.buf = replacement;
            }
        }
    }

    #[inline]
    #[must_use]
    #[cfg(feature = "casecmp")]
    #[cfg_attr(feature = "docsrs", doc(cfg(feature = "casecmp")))]
    pub fn ascii_casecmp(&self, other: &[u8]) -> Ordering {
        focaccia::ascii_casecmp(self.buf.as_slice(), other)
    }

    #[inline]
    #[must_use]
    #[cfg(feature = "casecmp")]
    #[cfg_attr(feature = "docsrs", doc(cfg(feature = "casecmp")))]
    pub fn unicode_casecmp(&self, other: &String, options: CaseFold) -> Option<bool> {
        let left = self.buf.as_slice();
        let right = other;
        // If both `String`s are conventionally UTF-8, they must be case
        // compared using the given case folding strategy. This requires the
        // `String`s be well-formed UTF-8.
        if let (Encoding::Utf8, Encoding::Utf8) = (self.encoding, other.encoding) {
            if let (Ok(left), Ok(right)) = (str::from_utf8(left), str::from_utf8(right)) {
                // Both slices are UTF-8, compare with the given Unicode case
                // folding scheme.
                Some(options.case_eq(left, right))
            } else {
                // At least one `String` contains invalid UTF-8 bytes.
                None
            }
        } else {
            // At least one slice is not conventionally UTF-8, so fallback to
            // ASCII comparator.
            Some(focaccia::ascii_case_eq(left, right))
        }
    }

    /// Centers this `String` in width with the given padding.
    ///
    /// This function returns an iterator that yields [`u8`].
    ///
    /// If width is greater than the length of this `String`, the returned
    /// iterator yields a byte sequence of length `width` with the byte content
    /// of this `String` centered and padded with the given padding; otherwise,
    /// yields the original bytes.
    ///
    /// If the given padding is [`None`], the `String` is padded with an ASCII
    /// space.
    ///
    /// # Examples
    ///
    /// ```
    /// use spinoso_string::String;
    ///
    /// let s = String::from("hello");
    ///
    /// assert_eq!(s.center(4, None).collect::<Vec<_>>(), b"hello");
    /// assert_eq!(s.center(20, None).collect::<Vec<_>>(), b"       hello        ");
    /// assert_eq!(s.center(20, Some(&b"123"[..])).collect::<Vec<_>>(), b"1231231hello12312312");
    /// ```
    ///
    /// This iterator is [encoding-aware]. [Conventionally UTF-8] strings are
    /// iterated by UTF-8 byte sequences.
    ///
    /// ```
    /// use spinoso_string::String;
    ///
    /// let s = String::from("💎");
    ///
    /// assert_eq!(s.center(3, None).collect::<Vec<_>>(), " 💎 ".as_bytes());
    /// ```
    ///
    /// [`center`]: crate::String::center
    /// [encoding-aware]: crate::Encoding
    /// [Conventionally UTF-8]: crate::Encoding::Utf8
    #[inline]
    #[must_use]
    pub fn center<'a, 'b>(&'a self, width: usize, padding: Option<&'b [u8]>) -> Center<'a, 'b> {
        let padding_width = width.saturating_sub(self.char_len());
        Center::with_chars_width_and_padding(self.chars(), padding_width, padding.unwrap_or(b" "))
    }

    /// Modifies this `String` in-place with the given record separator removed
    /// from the end of str (if given).
    ///
    /// If `separator` is [`None`] (i.e. `separator` has not been changed from
    /// the default Ruby record separator, then `chomp` also removes carriage
    /// return characters (that is it will remove `\n`, `\r`, and `\r\n`). If
    /// `separator` is an empty string, it will remove all trailing newlines
    /// from the string.
    ///
    /// # Examples
    ///
    /// ```
    /// use spinoso_string::String;
    ///
    /// let mut s = String::utf8(b"This is a paragraph.\r\n\n\n".to_vec());
    /// let modified = s.chomp(None::<&[u8]>);
    /// assert!(modified);
    /// assert_eq!(s, "This is a paragraph.");
    ///
    /// let mut s = String::utf8(b"This is a paragraph.\r\n\n\n".to_vec());
    /// let modified = s.chomp(Some(""));
    /// assert!(modified);
    /// assert_eq!(s, "This is a paragraph.\r");
    ///
    /// let mut s = String::utf8(b"This is a paragraph.".to_vec());
    /// let modified = s.chomp(Some("."));
    /// assert!(modified);
    /// assert_eq!(s, "This is a paragraph");
    ///
    /// let mut s = String::utf8(b"This is a paragraph.".to_vec());
    /// let modified = s.chomp(Some("abc"));
    /// assert!(!modified);
    /// assert_eq!(s, "This is a paragraph.");
    /// ```
    #[inline]
    #[must_use]
    #[allow(clippy::needless_pass_by_value)]
    pub fn chomp<T: AsRef<[u8]>>(&mut self, separator: Option<T>) -> bool {
        if self.is_empty() {
            return false;
        }
        match separator.as_ref().map(AsRef::as_ref) {
            Some(separator) if separator.is_empty() => {
                let original_len = self.len();
                let substr = self.buf.trim_end_with(|ch| matches!(ch, '\n'));
                let truncate_to_len = substr.len();
                self.buf.truncate(truncate_to_len);
                truncate_to_len != original_len
            }
            Some(separator) if self.buf.ends_with(separator) => {
                let original_len = self.len();
                // This subtraction is guaranteed not to panic because
                // `separator` is a substring of `buf`.
                let truncate_to_len = original_len - separator.len();
                self.buf.truncate(truncate_to_len);
                // Separator is non-empty and we are always truncating, so this
                // branch always modifies the buffer.
                true
            }
            Some(_) => false,
            None => {
                let original_len = self.len();
                let substr = self.buf.trim_end_with(|ch| matches!(ch, '\r' | '\n'));
                let truncate_to_len = substr.len();
                self.buf.truncate(truncate_to_len);
                truncate_to_len != original_len
            }
        }
    }

    /// Modifies this `String` in-place and removes the last character.
    ///
    /// This method returns a [`bool`] that indicates if this string was modified.
    ///
    /// If the string ends with `\r\n`, both characters are removed. When
    /// applying `chop` to an empty string, the string remains empty.
    ///
    /// [`String::chomp`] is often a safer alternative, as it leaves the string
    /// unchanged if it doesn't end in a record separator.
    ///
    /// # Examples
    ///
    /// ```
    /// use spinoso_string::String;
    ///
    /// let mut s = String::utf8(b"This is a paragraph.\r\n".to_vec());
    /// let modified = s.chop();
    /// assert!(modified);
    /// assert_eq!(s, "This is a paragraph.");
    ///
    /// let mut s = String::utf8(b"This is a paragraph.".to_vec());
    /// let modified = s.chop();
    /// assert!(modified);
    /// assert_eq!(s, "This is a paragraph");
    ///
    /// let mut s = String::utf8(b"".to_vec());
    /// let modified = s.chop();
    /// assert!(!modified);
    /// assert_eq!(s, "");
    ///
    /// let mut s = String::utf8(b"x".to_vec());
    /// let modified = s.chop();
    /// assert!(modified);
    /// assert_eq!(s, "");
    /// ```
    #[inline]
    #[must_use]
    pub fn chop(&mut self) -> bool {
        if self.buf.is_empty() {
            return false;
        }
        let bytes_to_remove = if self.buf.ends_with(b"\r\n") {
            2
        } else if let Encoding::Utf8 = self.encoding {
            let (ch, size) = bstr::decode_last_utf8(&self.buf);
            if ch.is_some() {
                size
            } else {
                1
            }
        } else {
            // `buf` is checked to be non-empty above.
            1
        };
        // This subtraction is guaranteed to not panic because we have validated
        // that we're removing a subslice of `buf`.
        self.buf.truncate(self.buf.len() - bytes_to_remove);
        true
    }

    /// Returns a one-character string at the beginning of the string.
    ///
    /// # Examples
    ///
    /// [Conventionally UTF-8] `String`s perform a partial UTF-8 decode to
    /// compute the first character.
    ///
    /// ```
    /// use spinoso_string::String;
    ///
    /// let s = String::utf8(b"abcde".to_vec());
    /// assert_eq!(s.chr(), &b"a"[..]);
    ///
    /// let s = String::utf8(b"".to_vec());
    /// assert_eq!(s.chr(), &[]);
    ///
    /// let s = String::utf8("🦀spinoso💎".as_bytes().to_vec());
    /// assert_eq!(s.chr(), &b"\xF0\x9F\xA6\x80"[..]);
    ///
    /// let s = String::utf8(b"\xFFspinoso".to_vec());
    /// assert_eq!(s.chr(), &b"\xFF"[..]);
    /// ```
    ///
    /// For [ASCII] and [binary] `String`s this function returns a slice of the
    /// first byte or the empty slice if the `String` is empty.
    ///
    /// ```
    /// use spinoso_string::String;
    ///
    /// let s = String::binary(b"abcde".to_vec());
    /// assert_eq!(s.chr(), &b"a"[..]);
    ///
    /// let s = String::binary(b"".to_vec());
    /// assert_eq!(s.chr(), &[]);
    ///
    /// let s = String::binary("🦀spinoso💎".as_bytes().to_vec());
    /// assert_eq!(s.chr(), &b"\xF0"[..]);
    ///
    /// let s = String::binary(b"\xFFspinoso".to_vec());
    /// assert_eq!(s.chr(), &b"\xFF"[..]);
    /// ```
    ///
    /// [Conventionally UTF-8]: Encoding::Utf8
    /// [ASCII]: crate::Encoding::Ascii
    /// [binary]: crate::Encoding::Binary
    #[inline]
    #[must_use]
    pub fn chr(&self) -> &[u8] {
        if let Encoding::Utf8 = self.encoding {
            let (_, size) = bstr::decode_utf8(self.buf.as_slice());
            &self.buf[..size]
        } else {
            self.buf.get(0..1).unwrap_or_default()
        }
    }

    /// Returns the index of the first occurrence of the given substring in this
    /// `String`.
    ///
    /// Returns [`None`] if not found. If the second parameter is present, it
    /// specifies the position in the string to begin the search.
    ///
    /// This function can be used to implement [`String#index`].
    ///
    /// # Examples
    ///
    /// ```
    /// use spinoso_string::String;
    ///
    /// let s = String::from("hello");
    /// assert_eq!(s.index("e", None), Some(1));
    /// assert_eq!(s.index("lo", None), Some(3));
    /// assert_eq!(s.index("a", None), None);
    /// assert_eq!(s.index("l", Some(3)), Some(3));
    /// ```
    ///
    /// [`String#index`]: https://ruby-doc.org/core-2.6.3/String.html#method-i-index
    #[inline]
    #[must_use]
    pub fn index<T: AsRef<[u8]>>(&self, needle: T, offset: Option<usize>) -> Option<usize> {
        if let Some(offset) = offset {
            let buf = self.buf.get(offset..)?;
            let index = buf.find(needle.as_ref())?;
            // This addition is guaranteed not to overflow because the result is
            // a valid index of the underlying `Vec`.
            //
            // `self.buf.len() < isize::MAX` because `self.buf` is a `Vec` and
            // `Vec` documents `isize::MAX` as its maximum allocation size.
            Some(index + offset)
        } else {
            self.buf.find(needle.as_ref())
        }
    }

    #[inline]
    #[must_use]
    pub fn rindex<T: AsRef<[u8]>>(&self, needle: T, offset: Option<usize>) -> Option<usize> {
        if let Some(offset) = offset {
            let end = self.buf.len().checked_sub(offset).unwrap_or_default();
            let buf = self.buf.get(..end)?;
            buf.rfind(needle.as_ref())
        } else {
            self.buf.rfind(needle.as_ref())
        }
    }

    /// Returns the Integer ordinal of a one-character string.
    ///
    /// # Errors
    ///
    /// If this `String` is empty, an error is returned.
    ///
    /// If this `String` is [conventionally UTF-8] and the string contents begin
    /// with an invalid UTF-8 byte sequence, an error is returned.
    ///
    /// [conventionally UTF-8]: crate::Encoding::Utf8
    #[inline]
    pub fn ord(&self) -> Result<u32, OrdError> {
        if let Encoding::Utf8 = self.encoding {
            let (ch, size) = bstr::decode_utf8(self.buf.as_slice());
            match ch {
                // All `char`s are valid `u32`s
                // https://github.com/rust-lang/rust/blob/1.48.0/library/core/src/char/convert.rs#L12-L20
                Some(ch) => Ok(u32::from(ch)),
                None if size == 0 => Err(OrdError::empty_string()),
                None => Err(OrdError::invalid_utf8_byte_sequence()),
            }
        } else {
            let byte = self.buf.get(0).copied().ok_or_else(OrdError::empty_string)?;
            Ok(u32::from(byte))
        }
    }
}

// Encoding-aware APIs.
impl String {
    /// Returns an iterator over the chars of a `String`.
    ///
    /// This function is encoding-aware. `String`s with [UTF-8 encoding] are
    /// only [conventionally UTF-8]. This iterator yields `&[u8]` byte slices
    /// that correspond to either a valid UTF-8 byte sequence or a single
    /// invalid UTF-8 byte. For [ASCII encoded] and [binary encoded] strings,
    /// this iterator yields slices of single bytes.
    ///
    /// For UTF-8 encoded strings, the yielded byte slices can be parsed into
    /// [`char`]s with [`str::from_utf8`] and [`str::chars`].
    ///
    /// # Examples
    ///
    /// Iterating over the characters of a conventionally UTF-8 string:
    ///
    /// ```
    /// use core::str;
    /// use spinoso_string::String;
    ///
    /// let s = String::utf8(b"ab\xF0\x9F\x92\x8E\xFF".to_vec());
    /// let mut chars = s.chars();
    /// assert_eq!(chars.next(), Some(&b"a"[..]));
    /// assert_eq!(chars.next().map(str::from_utf8), Some(Ok("b")));
    /// assert_eq!(chars.next(), Some(&[0xF0, 0x9F, 0x92, 0x8E][..]));
    /// assert_eq!(chars.next(), Some(&b"\xFF"[..]));
    /// assert_eq!(chars.next(), None);
    /// ```
    ///
    /// Iterating over the characters of a binary string:
    ///
    /// ```
    /// use spinoso_string::String;
    ///
    /// let s = String::binary("💎".as_bytes().to_vec());
    /// let mut chars = s.chars();
    /// assert_eq!(chars.next(), Some(&[0xF0][..]));
    /// assert_eq!(chars.next(), Some(&[0x9F][..]));
    /// assert_eq!(chars.next(), Some(&[0x92][..]));
    /// assert_eq!(chars.next(), Some(&[0x8E][..]));
    /// assert_eq!(chars.next(), None);
    /// ```
    ///
    /// [UTF-8 encoding]: crate::Encoding::Utf8
    /// [conventionally UTF-8]: crate::Encoding::Utf8
    /// [ASCII encoded]: crate::Encoding::Ascii
    /// [binary encoded]: crate::Encoding::Binary
    /// [`str::from_utf8`]: core::str::from_utf8
    #[inline]
    #[must_use]
    pub fn chars(&self) -> Chars<'_> {
        Chars::from(self)
    }

    /// Returns the character length of this `String`.
    ///
    /// This function is encoding-aware. For `String`s with [UTF-8 encoding],
    /// multi-byte Unicode characters are length 1 and invalid UTF-8 bytes are
    /// length 1. For `String`s with [ASCII encoding] or [binary encoding],
    /// this function is equivalent to [`len`] and [`bytesize`].
    ///
    /// # Examples
    ///
    /// ```
    /// use spinoso_string::String;
    ///
    /// let s = String::utf8(b"abc\xF0\x9F\x92\x8E\xFF".to_vec()); // "abc💎\xFF"
    /// assert_eq!(s.char_len(), 5);
    ///
    /// let b = String::binary(b"abc\xF0\x9F\x92\x8E\xFF".to_vec()); // "abc💎\xFF"
    /// assert_eq!(b.char_len(), 8);
    /// ```
    ///
    /// [UTF-8 encoding]: crate::Encoding::Utf8
    /// [ASCII encoding]: crate::Encoding::Ascii
    /// [binary encoding]: crate::Encoding::Binary
    /// [`len`]: Self::len
    /// [`bytesize`]: Self::bytesize
    #[inline]
    #[must_use]
    pub fn char_len(&self) -> usize {
        match self.encoding {
            Encoding::Ascii | Encoding::Binary => self.buf.len(),
            Encoding::Utf8 => conventionally_utf8_bytestring_len(self.buf.as_slice()),
        }
    }

    /// Returns true for a `String` which is encoded correctly.
    ///
    /// For this method to return true, `String`s with [conventionally UTF-8]
    /// must be well-formed UTF-8; [ASCII]-encoded `String`s must only contain
    /// bytes in the range `0..=127`; [binary]-encoded `String`s may contain any
    /// byte sequence.
    ///
    /// This method is suitable for implementing the Ruby method
    /// [`String#valid_encoding?`].
    ///
    /// # Examples
    ///
    /// ```
    /// use spinoso_string::{Encoding, String};
    ///
    /// let s = String::utf8(b"xyz".to_vec());
    /// assert!(s.is_valid_encoding());
    /// let s = String::utf8("🚀".to_string().into_bytes());
    /// assert!(s.is_valid_encoding());
    /// let s = String::utf8(b"abc\xFF\xFExyz".to_vec());
    /// assert!(!s.is_valid_encoding());
    ///
    /// let s = String::ascii(b"xyz".to_vec());
    /// assert!(s.is_valid_encoding());
    /// let s = String::ascii("🚀".to_string().into_bytes());
    /// assert!(!s.is_valid_encoding());
    /// let s = String::ascii(b"abc\xFF\xFExyz".to_vec());
    /// assert!(!s.is_valid_encoding());
    ///
    /// let s = String::binary(b"xyz".to_vec());
    /// assert!(s.is_valid_encoding());
    /// let s = String::binary("🚀".to_string().into_bytes());
    /// assert!(s.is_valid_encoding());
    /// let s = String::binary(b"abc\xFF\xFExyz".to_vec());
    /// assert!(s.is_valid_encoding());
    /// ```
    ///
    /// [conventionally UTF-8]: crate::Encoding::Utf8
    /// [ASCII]: crate::Encoding::Ascii
    /// [binary]: crate::Encoding::Binary
    /// [`String#valid_encoding?`]: https://ruby-doc.org/core-3.0.0/String.html#method-i-valid_encoding-3F
    #[inline]
    #[must_use]
    pub fn is_valid_encoding(&self) -> bool {
        match self.encoding {
            Encoding::Utf8 => self.buf.is_utf8(),
            Encoding::Ascii => self.buf.is_ascii(),
            Encoding::Binary => true,
        }
    }
}

#[inline]
#[must_use]
fn conventionally_utf8_bytestring_len<T: AsRef<[u8]>>(bytes: T) -> usize {
    let mut bytes = bytes.as_ref();
    let mut char_len = 0;
    while !bytes.is_empty() {
        let (ch, size) = bstr::decode_utf8(bytes);
        char_len += if ch.is_some() { 1 } else { size };
        bytes = &bytes[size..];
    }
    char_len
}

#[cfg(test)]
#[allow(clippy::clippy::shadow_unrelated)]
#[allow(clippy::invisible_characters)]
mod tests {
    use alloc::string::ToString;
    use alloc::vec::Vec;
    use core::str;
    use quickcheck::quickcheck;

    use crate::{conventionally_utf8_bytestring_len, String};

    const REPLACEMENT_CHARACTER_BYTES: [u8; 3] = [239, 191, 189];

    #[test]
    fn utf8_char_len_empty() {
        let s = "";
        assert_eq!(conventionally_utf8_bytestring_len(s), 0);
    }

    #[test]
    fn utf8_char_len_ascii() {
        let s = "Artichoke Ruby";
        assert_eq!(conventionally_utf8_bytestring_len(s), 14);
    }

    #[test]
    fn utf8_char_len_emoji() {
        let s = "💎";
        assert_eq!(conventionally_utf8_bytestring_len(s), 1);
        let s = "💎🦀🎉";
        assert_eq!(conventionally_utf8_bytestring_len(s), 3);
        let s = "a💎b🦀c🎉d";
        assert_eq!(conventionally_utf8_bytestring_len(s), 7);
        // with invalid UFF-8 bytes
        let s = b"a\xF0\x9F\x92\x8E\xFFabc";
        assert_eq!(conventionally_utf8_bytestring_len(s), 6);
    }

    #[test]
    fn utf8_char_len_unicode_replacement_character() {
        let s = "�";
        assert_eq!(conventionally_utf8_bytestring_len(s), 1);
        let s = "���";
        assert_eq!(conventionally_utf8_bytestring_len(s), 3);
        let s = "a�b�c�d";
        assert_eq!(conventionally_utf8_bytestring_len(s), 7);
        let s = "�💎b🦀c🎉�";
        assert_eq!(conventionally_utf8_bytestring_len(s), 7);
        // with invalid UFF-8 bytes
        let s = b"\xEF\xBF\xBD\xF0\x9F\x92\x8E\xFF\xEF\xBF\xBDab";
        assert_eq!(conventionally_utf8_bytestring_len(s), 6);
        assert_eq!(conventionally_utf8_bytestring_len(REPLACEMENT_CHARACTER_BYTES), 1);
    }

    #[test]
    fn utf8_char_len_nul_byte() {
        let s = b"\x00";
        assert_eq!(conventionally_utf8_bytestring_len(s), 1);
        let s = b"abc\x00";
        assert_eq!(conventionally_utf8_bytestring_len(s), 4);
        let s = b"abc\x00xyz";
        assert_eq!(conventionally_utf8_bytestring_len(s), 7);
    }

    #[test]
    fn utf8_char_len_invalid_utf8_byte_sequences() {
        let s = b"\x00\x00\xD8\x00";
        assert_eq!(conventionally_utf8_bytestring_len(s), 4);
        let s = b"\xFF\xFE";
        assert_eq!(conventionally_utf8_bytestring_len(s), 2);
    }

    #[test]
    fn utf8_char_len_binary() {
        let bytes = &[
            0xB3, 0x7E, 0x39, 0x70, 0x8E, 0xFD, 0xBB, 0x75, 0x62, 0x77, 0xE7, 0xDF, 0x6F, 0xF2, 0x76, 0x27, 0x81,
            0x9A, 0x3A, 0x9D, 0xED, 0x6B, 0x4F, 0xAE, 0xC4, 0xE7, 0xA1, 0x66, 0x11, 0xF1, 0x08, 0x1C,
        ];
        assert_eq!(conventionally_utf8_bytestring_len(bytes), 32);
        // Mixed binary and ASCII
        let bytes = &[
            b'?', b'!', b'a', b'b', b'c', 0xFD, 0xBB, 0x75, 0x62, 0x77, 0xE7, 0xDF, 0x6F, 0xF2, 0x76, 0x27, 0x81,
            0x9A, 0x3A, 0x9D, 0xED, 0x6B, 0x4F, 0xAE, 0xC4, 0xE7, 0xA1, 0x66, 0x11, 0xF1, 0x08, 0x1C,
        ];
        assert_eq!(conventionally_utf8_bytestring_len(bytes), 32);
    }

    #[test]
    fn utf8_char_len_mixed_ascii_emoji_invalid_bytes() {
        // [2.6.3] > s = "🦀abc💎\xff"
        // => "🦀abc💎\xFF"
        // [2.6.3] > s.length
        // => 6
        // [2.6.3] > puts s.bytes.map{|b| "\\x#{b.to_s(16).upcase}"}.join
        // \xF0\x9F\xA6\x80\x61\x62\x63\xF0\x9F\x92\x8E\xFF
        let bytes = b"\xF0\x9F\xA6\x80\x61\x62\x63\xF0\x9F\x92\x8E\xFF";
        assert_eq!(conventionally_utf8_bytestring_len(&bytes), 6);
    }

    #[test]
    fn utf8_char_len_utf8() {
        // https://github.com/minimaxir/big-list-of-naughty-strings/blob/894882e7/blns.txt#L147-L157
        let s = "Ω≈ç√∫˜µ≤≥÷";
        assert_eq!(conventionally_utf8_bytestring_len(s), 10);
        let s = "åß∂ƒ©˙∆˚¬…æ";
        assert_eq!(conventionally_utf8_bytestring_len(s), 11);
        let s = "œ∑´®†¥¨ˆøπ“‘";
        assert_eq!(conventionally_utf8_bytestring_len(s), 12);
        let s = "¡™£¢∞§¶•ªº–≠";
        assert_eq!(conventionally_utf8_bytestring_len(s), 12);
        let s = "¸˛Ç◊ı˜Â¯˘¿";
        assert_eq!(conventionally_utf8_bytestring_len(s), 10);
        let s = "ÅÍÎÏ˝ÓÔÒÚÆ☃";
        assert_eq!(conventionally_utf8_bytestring_len(s), 12);
        let s = "Œ„´‰ˇÁ¨ˆØ∏”’";
        assert_eq!(conventionally_utf8_bytestring_len(s), 12);
        let s = "`⁄€‹›ﬁﬂ‡°·‚—±";
        assert_eq!(conventionally_utf8_bytestring_len(s), 13);
        let s = "⅛⅜⅝⅞";
        assert_eq!(conventionally_utf8_bytestring_len(s), 4);
        let s = "ЁЂЃЄЅІЇЈЉЊЋЌЍЎЏАБВГДЕЖЗИЙКЛМНОПРСТУФХЦЧШЩЪЫЬЭЮЯабвгдежзийклмнопрстуфхцчшщъыьэюя";
        assert_eq!(conventionally_utf8_bytestring_len(s), 79);
    }

    #[test]
    fn utf8_char_len_vmware_super_string() {
        // A super string recommended by VMware Inc. Globalization Team: can
        // effectively cause rendering issues or character-length issues to
        // validate product globalization readiness.
        //
        // https://github.com/minimaxir/big-list-of-naughty-strings/blob/894882e7/blns.txt#L202-L224
        let s = "表ポあA鷗ŒéＢ逍Üßªąñ丂㐀𠀀";
        assert_eq!(conventionally_utf8_bytestring_len(s), 17);
    }

    #[test]
    fn utf8_char_len_two_byte_chars() {
        // https://github.com/minimaxir/big-list-of-naughty-strings/blob/894882e7/blns.txt#L188-L196
        let s = "田中さんにあげて下さい";
        assert_eq!(conventionally_utf8_bytestring_len(s), 11);
        let s = "パーティーへ行かないか";
        assert_eq!(conventionally_utf8_bytestring_len(s), 11);
        let s = "和製漢語";
        assert_eq!(conventionally_utf8_bytestring_len(s), 4);
        let s = "部落格";
        assert_eq!(conventionally_utf8_bytestring_len(s), 3);
        let s = "사회과학원 어학연구소";
        assert_eq!(conventionally_utf8_bytestring_len(s), 11);
        let s = "찦차를 타고 온 펲시맨과 쑛다리 똠방각하";
        assert_eq!(conventionally_utf8_bytestring_len(s), 22);
        let s = "社會科學院語學研究所";
        assert_eq!(conventionally_utf8_bytestring_len(s), 10);
        let s = "울란바토르";
        assert_eq!(conventionally_utf8_bytestring_len(s), 5);
        let s = "𠜎𠜱𠝹𠱓𠱸𠲖𠳏";
        assert_eq!(conventionally_utf8_bytestring_len(s), 7);
    }

    #[test]
    fn utf8_char_len_space_chars() {
        // Whitespace: all of the characters with category Zs, Zl, or Zp (in Unicode
        // version 8.0.0), plus U+0009 (HT), U+000B (VT), U+000C (FF), U+0085 (NEL),
        // and U+200B (ZERO WIDTH SPACE), which are in the C categories but are often
        // treated as whitespace in some contexts.
        // This file unfortunately cannot express strings containing
        // U+0000, U+000A, or U+000D (NUL, LF, CR).
        // The next line may appear to be blank or mojibake in some viewers.
        // The next line may be flagged for "trailing whitespace" in some viewers.
        //
        // https://github.com/minimaxir/big-list-of-naughty-strings/blob/894882e7/blns.txt#L131
        let s = "	              ​    　
";
        assert_eq!(conventionally_utf8_bytestring_len(s), 24);
    }

    quickcheck! {
        #[allow(clippy::clippy::needless_pass_by_value)]
        fn fuzz_char_len_utf8_contents_utf8_string(contents: alloc::string::String) -> bool {
            let expected = contents.chars().count();
            let s = String::utf8(contents.into_bytes());
            s.char_len() == expected
        }

        #[allow(clippy::clippy::needless_pass_by_value)]
        fn fuzz_len_utf8_contents_utf8_string(contents: alloc::string::String) -> bool {
            let expected = contents.len();
            let s = String::utf8(contents.into_bytes());
            s.len() == expected
        }

        #[allow(clippy::clippy::needless_pass_by_value)]
        fn fuzz_bytesize_utf8_contents_utf8_string(contents: alloc::string::String) -> bool {
            let expected = contents.len();
            let s = String::utf8(contents.into_bytes());
            s.bytesize() == expected
        }

        #[allow(clippy::clippy::needless_pass_by_value)]
        fn fuzz_char_len_binary_contents_utf8_string(contents: Vec<u8>) -> bool {
            if let Ok(utf8_contents) = str::from_utf8(&contents) {
                let expected = utf8_contents.chars().count();
                let s = String::utf8(contents);
                s.char_len() == expected
            } else {
                let expected_at_most = contents.len();
                let s = String::utf8(contents);
                s.char_len() <= expected_at_most
            }
        }

        #[allow(clippy::clippy::needless_pass_by_value)]
        fn fuzz_len_binary_contents_utf8_string(contents: Vec<u8>) -> bool {
            let expected = contents.len();
            let s = String::utf8(contents);
            s.len() == expected
        }

        #[allow(clippy::clippy::needless_pass_by_value)]
        fn fuzz_bytesize_binary_contents_utf8_string(contents: Vec<u8>) -> bool {
            let expected = contents.len();
            let s = String::utf8(contents);
            s.bytesize() == expected
        }

        #[allow(clippy::clippy::needless_pass_by_value)]
        fn fuzz_char_len_utf8_contents_ascii_string(contents: alloc::string::String) -> bool {
            let expected = contents.len();
            let s = String::ascii(contents.into_bytes());
            s.char_len() == expected
        }

        #[allow(clippy::clippy::needless_pass_by_value)]
        fn fuzz_len_utf8_contents_ascii_string(contents: alloc::string::String) -> bool {
            let expected = contents.len();
            let s = String::ascii(contents.into_bytes());
            s.len() == expected
        }

        #[allow(clippy::clippy::needless_pass_by_value)]
        fn fuzz_bytesize_utf8_contents_ascii_string(contents: alloc::string::String) -> bool {
            let expected = contents.len();
            let s = String::ascii(contents.into_bytes());
            s.bytesize() == expected
        }

        #[allow(clippy::clippy::needless_pass_by_value)]
        fn fuzz_char_len_binary_contents_ascii_string(contents: Vec<u8>) -> bool {
            let expected = contents.len();
            let s = String::ascii(contents);
            s.char_len() == expected
        }

        #[allow(clippy::clippy::needless_pass_by_value)]
        fn fuzz_len_binary_contents_ascii_string(contents: Vec<u8>) -> bool {
            let expected = contents.len();
            let s = String::ascii(contents);
            s.len() == expected
        }

        #[allow(clippy::clippy::needless_pass_by_value)]
        fn fuzz_bytesize_binary_contents_ascii_string(contents: Vec<u8>) -> bool {
            let expected = contents.len();
            let s = String::ascii(contents);
            s.bytesize() == expected
        }

        #[allow(clippy::clippy::needless_pass_by_value)]
        fn fuzz_char_len_utf8_contents_binary_string(contents: alloc::string::String) -> bool {
            let expected = contents.len();
            let s = String::binary(contents.into_bytes());
            s.char_len() == expected
        }

        #[allow(clippy::clippy::needless_pass_by_value)]
        fn fuzz_len_utf8_contents_binary_string(contents: alloc::string::String) -> bool {
            let expected = contents.len();
            let s = String::binary(contents.into_bytes());
            s.len() == expected
        }

        #[allow(clippy::clippy::needless_pass_by_value)]
        fn fuzz_bytesize_utf8_contents_binary_string(contents: alloc::string::String) -> bool {
            let expected = contents.len();
            let s = String::binary(contents.into_bytes());
            s.bytesize() == expected
        }

        #[allow(clippy::clippy::needless_pass_by_value)]
        fn fuzz_char_len_binary_contents_binary_string(contents: Vec<u8>) -> bool {
            let expected = contents.len();
            let s = String::binary(contents);
            s.char_len() == expected
        }

        #[allow(clippy::clippy::needless_pass_by_value)]
        fn fuzz_len_binary_contents_binary_string(contents: Vec<u8>) -> bool {
            let expected = contents.len();
            let s = String::binary(contents);
            s.len() == expected
        }

        #[allow(clippy::clippy::needless_pass_by_value)]
        fn fuzz_bytesize_binary_contents_binary_string(contents: Vec<u8>) -> bool {
            let expected = contents.len();
            let s = String::binary(contents);
            s.bytesize() == expected
        }
    }

    #[test]
    fn make_capitalized_utf8_string_empty() {
        let mut s = String::utf8(b"".to_vec());
        s.make_capitalized();
        assert_eq!(s, "");
    }

    #[test]
    fn make_capitalized_utf8_string_ascii() {
        let mut s = String::utf8(b"abc".to_vec());
        s.make_capitalized();
        assert_eq!(s, "Abc");

        let mut s = String::utf8(b"aBC".to_vec());
        s.make_capitalized();
        assert_eq!(s, "Abc");

        let mut s = String::utf8(b"ABC".to_vec());
        s.make_capitalized();
        assert_eq!(s, "Abc");

        let mut s = String::utf8(b"aBC, 123, ABC, baby you and me girl".to_vec());
        s.make_capitalized();
        assert_eq!(s, "Abc, 123, abc, baby you and me girl");
    }

    #[test]
    fn make_capitalized_utf8_string_utf8() {
        let mut s = String::utf8("ß".to_string().into_bytes());
        s.make_capitalized();
        // This differs from MRI:
        //
        // ```console
        // [2.6.3] > "ß".capitalize
        // => "Ss"
        // ```
        assert_eq!(s, "SS");

        let mut s = String::utf8("αύριο".to_string().into_bytes());
        s.make_capitalized();
        assert_eq!(s, "Αύριο");

        let mut s = String::utf8("έτος".to_string().into_bytes());
        s.make_capitalized();
        assert_eq!(s, "Έτος");

        // two-byte characters
        // https://github.com/minimaxir/big-list-of-naughty-strings/blob/894882e7/blns.txt#L198-L200
        let mut s = String::utf8(
            "𐐜 𐐔𐐇𐐝𐐀𐐡𐐇𐐓 𐐙𐐊𐐡𐐝𐐓/𐐝𐐇𐐗𐐊𐐤𐐔 𐐒𐐋𐐗 𐐒𐐌 𐐜 𐐡𐐀𐐖𐐇𐐤𐐓𐐝 𐐱𐑂 𐑄 𐐔𐐇𐐝𐐀𐐡𐐇𐐓 𐐏𐐆𐐅𐐤𐐆𐐚𐐊𐐡𐐝𐐆𐐓𐐆"
                .to_string()
                .into_bytes(),
        );
        s.make_capitalized();
        assert_eq!(s, "𐐜 𐐼𐐯𐑅𐐨𐑉𐐯𐐻 𐑁𐐲𐑉𐑅𐐻/𐑅𐐯𐐿𐐲𐑌𐐼 𐐺𐐳𐐿 𐐺𐐴 𐑄 𐑉𐐨𐐾𐐯𐑌𐐻𐑅 𐐱𐑂 𐑄 𐐼𐐯𐑅𐐨𐑉𐐯𐐻 𐐷𐐮𐐭𐑌𐐮𐑂𐐲𐑉𐑅𐐮𐐻𐐮");

        // Change length when lowercased
        // https://github.com/minimaxir/big-list-of-naughty-strings/blob/894882e7/blns.txt#L226-L232
        let mut s = String::utf8("zȺȾ".to_string().into_bytes());
        s.make_capitalized();
        assert_eq!(s, "Zⱥⱦ");
    }

    #[test]
    fn make_capitalized_utf8_string_invalid_utf8() {
        let mut s = String::utf8(b"\xFF\xFE".to_vec());
        s.make_capitalized();
        assert_eq!(s, &b"\xFF\xFE"[..]);
    }

    #[test]
    fn make_capitalized_utf8_string_unicode_replacement_character() {
        let mut s = String::utf8("�".to_string().into_bytes());
        s.make_capitalized();
        assert_eq!(s, "�");
    }

    #[test]
    fn make_capitalized_ascii_string_empty() {
        let mut s = String::ascii(b"".to_vec());
        s.make_capitalized();
        assert_eq!(s, "");
    }

    #[test]
    fn make_capitalized_ascii_string_ascii() {
        let mut s = String::ascii(b"abc".to_vec());
        s.make_capitalized();
        assert_eq!(s, "Abc");

        let mut s = String::ascii(b"aBC".to_vec());
        s.make_capitalized();
        assert_eq!(s, "Abc");

        let mut s = String::ascii(b"ABC".to_vec());
        s.make_capitalized();
        assert_eq!(s, "Abc");

        let mut s = String::ascii(b"aBC, 123, ABC, baby you and me girl".to_vec());
        s.make_capitalized();
        assert_eq!(s, "Abc, 123, abc, baby you and me girl");
    }

    #[test]
    fn make_capitalized_ascii_string_utf8() {
        let mut s = String::ascii("ß".to_string().into_bytes());
        s.make_capitalized();
        assert_eq!(s, "ß");

        let mut s = String::ascii("αύριο".to_string().into_bytes());
        s.make_capitalized();
        assert_eq!(s, "αύριο");

        let mut s = String::ascii("έτος".to_string().into_bytes());
        s.make_capitalized();
        assert_eq!(s, "έτος");

        // two-byte characters
        // https://github.com/minimaxir/big-list-of-naughty-strings/blob/894882e7/blns.txt#L198-L200
        let mut s = String::ascii(
            "𐐜 𐐔𐐇𐐝𐐀𐐡𐐇𐐓 𐐙𐐊𐐡𐐝𐐓/𐐝𐐇𐐗𐐊𐐤𐐔 𐐒𐐋𐐗 𐐒𐐌 𐐜 𐐡𐐀𐐖𐐇𐐤𐐓𐐝 𐐱𐑂 𐑄 𐐔𐐇𐐝𐐀𐐡𐐇𐐓 𐐏𐐆𐐅𐐤𐐆𐐚𐐊𐐡𐐝𐐆𐐓𐐆"
                .to_string()
                .into_bytes(),
        );
        s.make_capitalized();
        assert_eq!(s, "𐐜 𐐔𐐇𐐝𐐀𐐡𐐇𐐓 𐐙𐐊𐐡𐐝𐐓/𐐝𐐇𐐗𐐊𐐤𐐔 𐐒𐐋𐐗 𐐒𐐌 𐐜 𐐡𐐀𐐖𐐇𐐤𐐓𐐝 𐐱𐑂 𐑄 𐐔𐐇𐐝𐐀𐐡𐐇𐐓 𐐏𐐆𐐅𐐤𐐆𐐚𐐊𐐡𐐝𐐆𐐓𐐆");

        // Change length when lowercased
        // https://github.com/minimaxir/big-list-of-naughty-strings/blob/894882e7/blns.txt#L226-L232
        let mut s = String::ascii("zȺȾ".to_string().into_bytes());
        s.make_capitalized();
        assert_eq!(s, "ZȺȾ");
    }

    #[test]
    fn make_capitalized_ascii_string_invalid_utf8() {
        let mut s = String::ascii(b"\xFF\xFE".to_vec());
        s.make_capitalized();
        assert_eq!(s, &b"\xFF\xFE"[..]);
    }

    #[test]
    fn make_capitalized_ascii_string_unicode_replacement_character() {
        let mut s = String::ascii("�".to_string().into_bytes());
        s.make_capitalized();
        assert_eq!(s, "�");
    }

    #[test]
    fn make_capitalized_binary_string_empty() {
        let mut s = String::binary(b"".to_vec());
        s.make_capitalized();
        assert_eq!(s, "");
    }

    #[test]
    fn make_capitalized_binary_string_ascii() {
        let mut s = String::binary(b"abc".to_vec());
        s.make_capitalized();
        assert_eq!(s, "Abc");

        let mut s = String::ascii(b"aBC".to_vec());
        s.make_capitalized();
        assert_eq!(s, "Abc");

        let mut s = String::ascii(b"ABC".to_vec());
        s.make_capitalized();
        assert_eq!(s, "Abc");

        let mut s = String::ascii(b"aBC, 123, ABC, baby you and me girl".to_vec());
        s.make_capitalized();
        assert_eq!(s, "Abc, 123, abc, baby you and me girl");
    }

    #[test]
    fn make_capitalized_binary_string_utf8() {
        let mut s = String::binary("ß".to_string().into_bytes());
        s.make_capitalized();
        assert_eq!(s, "ß");

        let mut s = String::binary("αύριο".to_string().into_bytes());
        s.make_capitalized();
        assert_eq!(s, "αύριο");

        let mut s = String::binary("έτος".to_string().into_bytes());
        s.make_capitalized();
        assert_eq!(s, "έτος");

        // two-byte characters
        // https://github.com/minimaxir/big-list-of-naughty-strings/blob/894882e7/blns.txt#L198-L200
        let mut s = String::binary(
            "𐐜 𐐔𐐇𐐝𐐀𐐡𐐇𐐓 𐐙𐐊𐐡𐐝𐐓/𐐝𐐇𐐗𐐊𐐤𐐔 𐐒𐐋𐐗 𐐒𐐌 𐐜 𐐡𐐀𐐖𐐇𐐤𐐓𐐝 𐐱𐑂 𐑄 𐐔𐐇𐐝𐐀𐐡𐐇𐐓 𐐏𐐆𐐅𐐤𐐆𐐚𐐊𐐡𐐝𐐆𐐓𐐆"
                .to_string()
                .into_bytes(),
        );
        s.make_capitalized();
        assert_eq!(s, "𐐜 𐐔𐐇𐐝𐐀𐐡𐐇𐐓 𐐙𐐊𐐡𐐝𐐓/𐐝𐐇𐐗𐐊𐐤𐐔 𐐒𐐋𐐗 𐐒𐐌 𐐜 𐐡𐐀𐐖𐐇𐐤𐐓𐐝 𐐱𐑂 𐑄 𐐔𐐇𐐝𐐀𐐡𐐇𐐓 𐐏𐐆𐐅𐐤𐐆𐐚𐐊𐐡𐐝𐐆𐐓𐐆");

        // Change length when lowercased
        // https://github.com/minimaxir/big-list-of-naughty-strings/blob/894882e7/blns.txt#L226-L232
        let mut s = String::binary("zȺȾ".to_string().into_bytes());
        s.make_capitalized();
        assert_eq!(s, "ZȺȾ");
    }

    #[test]
    fn make_capitalized_binary_string_invalid_utf8() {
        let mut s = String::binary(b"\xFF\xFE".to_vec());
        s.make_capitalized();
        assert_eq!(s, &b"\xFF\xFE"[..]);
    }

    #[test]
    fn make_capitalized_binary_string_unicode_replacement_character() {
        let mut s = String::binary("�".to_string().into_bytes());
        s.make_capitalized();
        assert_eq!(s, "�");
    }
}
