//! Parser for classifying bytestrings as Ruby identifiers.
//!
//! This module exposes a parser for determining if a sequence of bytes is a
//! valid Ruby identifier. These routines also classify idents by type, for
//! example, a local variable (`is_spinoso`), constant name (`SPINOSO_SYMBOL`),
//! or class variable (`@@spinoso_symbol`).
//!
//! # Examples – local variable
//!
//! ```
//! # use spinoso_symbol::IdentifierType;
//! assert_eq!("spinoso".parse::<IdentifierType>(), Ok(IdentifierType::Local));
//! assert_eq!("spinoso_symbol_features".parse::<IdentifierType>(), Ok(IdentifierType::Local));
//! ```
//!
//! # Examples – constant
//!
//! ```
//! # use spinoso_symbol::IdentifierType;
//! assert_eq!("Spinoso".parse::<IdentifierType>(), Ok(IdentifierType::Constant));
//! assert_eq!("SpinosoSymbol".parse::<IdentifierType>(), Ok(IdentifierType::Constant));
//! assert_eq!("SPINOSO_SYMBOL_FEATURES".parse::<IdentifierType>(), Ok(IdentifierType::Constant));
//! ```
//!
//! # Examples – global
//!
//! ```
//! # use spinoso_symbol::IdentifierType;
//! assert_eq!("$use_spinoso_symbol".parse::<IdentifierType>(), Ok(IdentifierType::Global));
//! assert_eq!("$USE_SPINOSO_SYMBOL".parse::<IdentifierType>(), Ok(IdentifierType::Global));
//! ```
//!
//! # Examples – instance and class variables
//!
//! ```
//! # use spinoso_symbol::IdentifierType;
//! assert_eq!("@artichoke".parse::<IdentifierType>(), Ok(IdentifierType::Instance));
//! assert_eq!("@@rumble".parse::<IdentifierType>(), Ok(IdentifierType::Class));
//! ```
//!
//! # Example – attribute setter
//!
//! Attribute setters are local idents that end in `=`.
//!
//! ```
//! # use spinoso_symbol::IdentifierType;
//! assert_eq!("artichoke=".parse::<IdentifierType>(), Ok(IdentifierType::AttrSet));
//! assert_eq!("spinoso_symbol=".parse::<IdentifierType>(), Ok(IdentifierType::AttrSet));
//! ```

use bstr::ByteSlice;
use core::convert::TryFrom;
use core::fmt;
use core::str::FromStr;

/// Valid types for Ruby identifiers.
///
/// Spinoso symbol parses bytestrings to determine if they are valid idents for
/// the [`Inspect`] iterator (which requires the **inspect** Cargo feature to be
/// enabled). Symbols that are valid idents do not get wrapped in `"` when
/// generating their debug output.
///
/// See variant documentation for the set of ident types.
///
/// `IdentifierType`'s primary interface is through the [`TryFrom`] and
/// [`FromStr`] conversion traits. Parsing `&str` and `&[u8]` is supported.
///
/// # Examples – local variable
///
/// ```
/// # use spinoso_symbol::IdentifierType;
/// assert_eq!("spinoso".parse::<IdentifierType>(), Ok(IdentifierType::Local));
/// assert_eq!("spinoso_symbol_features".parse::<IdentifierType>(), Ok(IdentifierType::Local));
/// ```
///
/// # Examples – constant
///
/// ```
/// # use spinoso_symbol::IdentifierType;
/// assert_eq!("Spinoso".parse::<IdentifierType>(), Ok(IdentifierType::Constant));
/// assert_eq!("SpinosoSymbol".parse::<IdentifierType>(), Ok(IdentifierType::Constant));
/// assert_eq!("SPINOSO_SYMBOL_FEATURES".parse::<IdentifierType>(), Ok(IdentifierType::Constant));
/// ```
///
/// # Examples – global
///
/// ```
/// # use spinoso_symbol::IdentifierType;
/// assert_eq!("$use_spinoso_symbol".parse::<IdentifierType>(), Ok(IdentifierType::Global));
/// assert_eq!("$USE_SPINOSO_SYMBOL".parse::<IdentifierType>(), Ok(IdentifierType::Global));
/// ```
///
/// # Examples – instance and class variables
///
/// ```
/// # use spinoso_symbol::IdentifierType;
/// assert_eq!("@artichoke".parse::<IdentifierType>(), Ok(IdentifierType::Instance));
/// assert_eq!("@@rumble".parse::<IdentifierType>(), Ok(IdentifierType::Class));
/// ```
///
/// # Example – attribute setter
///
/// Attribute setters are local idents that end in `=`.
///
/// ```
/// # use spinoso_symbol::IdentifierType;
/// assert_eq!("artichoke=".parse::<IdentifierType>(), Ok(IdentifierType::AttrSet));
/// assert_eq!("spinoso_symbol=".parse::<IdentifierType>(), Ok(IdentifierType::AttrSet));
/// ```
///
/// [`Inspect`]: crate::Inspect
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum IdentifierType {
    /// Identifier that contains "junk".
    ///
    /// Junk mostly equates to non-sigil ASCII symbols. Identifiers like
    /// `empty?` and `flatten!` are junk idents. All special symbolic Ruby
    /// methods like `<=>` and `!~` are junk identifiers.
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_symbol::IdentifierType;
    /// assert_eq!("empty?".parse::<IdentifierType>(), Ok(IdentifierType::Junk));
    /// assert_eq!("flatten!".parse::<IdentifierType>(), Ok(IdentifierType::Junk));
    /// assert_eq!("<=>".parse::<IdentifierType>(), Ok(IdentifierType::Junk));
    /// assert_eq!("!~".parse::<IdentifierType>(), Ok(IdentifierType::Junk));
    /// assert_eq!("[]".parse::<IdentifierType>(), Ok(IdentifierType::Junk));
    /// assert_eq!("[]=".parse::<IdentifierType>(), Ok(IdentifierType::Junk));
    /// assert_eq!("=~".parse::<IdentifierType>(), Ok(IdentifierType::Junk));
    /// assert_eq!("==".parse::<IdentifierType>(), Ok(IdentifierType::Junk));
    /// assert_eq!("===".parse::<IdentifierType>(), Ok(IdentifierType::Junk));
    /// ```
    Junk,
    /// Identifier that is a global variable name.
    ///
    /// Global variables are prefixed with the sigil `$`. There are two types of
    /// global variables:
    ///
    /// - `$` followed by a `IdentifierType::Ident` sequence.
    /// - Special global variables, which include `Regexp` globals (`$1`..`$9`)
    ///   and `$-w` type globals.
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_symbol::{IdentifierType, ParseIdentifierError};
    /// assert_eq!("$".parse::<IdentifierType>(), Err(ParseIdentifierError::new()));
    /// assert_eq!("$foo".parse::<IdentifierType>(), Ok(IdentifierType::Global));
    /// assert_eq!("$@foo".parse::<IdentifierType>(), Err(ParseIdentifierError::new()));
    /// assert_eq!("$0".parse::<IdentifierType>(), Ok(IdentifierType::Global));
    /// assert_eq!("$1".parse::<IdentifierType>(), Ok(IdentifierType::Global));
    /// assert_eq!("$9".parse::<IdentifierType>(), Ok(IdentifierType::Global));
    /// assert_eq!("$-w".parse::<IdentifierType>(), Ok(IdentifierType::Global));
    /// assert_eq!("$-www".parse::<IdentifierType>(), Err(ParseIdentifierError::new()));
    /// ```
    Global,
    /// Identifier that is an instance variable name.
    ///
    /// Instance variables are prefixed with a single `@` sigil. The remaining
    /// bytes must be a valid [`Constant`] or [`Local`] ident.
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_symbol::{IdentifierType, ParseIdentifierError};
    /// assert_eq!("@".parse::<IdentifierType>(), Err(ParseIdentifierError::new()));
    /// assert_eq!("@foo".parse::<IdentifierType>(), Ok(IdentifierType::Instance));
    /// assert_eq!("@Foo".parse::<IdentifierType>(), Ok(IdentifierType::Instance));
    /// assert_eq!("@FOO".parse::<IdentifierType>(), Ok(IdentifierType::Instance));
    /// assert_eq!("@foo_bar".parse::<IdentifierType>(), Ok(IdentifierType::Instance));
    /// assert_eq!("@FooBar".parse::<IdentifierType>(), Ok(IdentifierType::Instance));
    /// assert_eq!("@FOO_BAR".parse::<IdentifierType>(), Ok(IdentifierType::Instance));
    /// assert_eq!("@$foo".parse::<IdentifierType>(), Err(ParseIdentifierError::new()));
    /// assert_eq!("@0".parse::<IdentifierType>(), Err(ParseIdentifierError::new()));
    /// assert_eq!("@1".parse::<IdentifierType>(), Err(ParseIdentifierError::new()));
    /// assert_eq!("@9".parse::<IdentifierType>(), Err(ParseIdentifierError::new()));
    /// ```
    ///
    /// [`Constant`]: Self::Constant
    /// [`Local`]: Self::Local
    Instance,
    /// Identifier that is a class variable name.
    ///
    /// Class variables are prefixed with a double `@@` sigil. The remaining
    /// bytes must be a valid [`Constant`] or [`Local`] ident.
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_symbol::{IdentifierType, ParseIdentifierError};
    /// assert_eq!("@@".parse::<IdentifierType>(), Err(ParseIdentifierError::new()));
    /// assert_eq!("@@foo".parse::<IdentifierType>(), Ok(IdentifierType::Class));
    /// assert_eq!("@@Foo".parse::<IdentifierType>(), Ok(IdentifierType::Class));
    /// assert_eq!("@@FOO".parse::<IdentifierType>(), Ok(IdentifierType::Class));
    /// assert_eq!("@@foo_bar".parse::<IdentifierType>(), Ok(IdentifierType::Class));
    /// assert_eq!("@@FooBar".parse::<IdentifierType>(), Ok(IdentifierType::Class));
    /// assert_eq!("@@FOO_BAR".parse::<IdentifierType>(), Ok(IdentifierType::Class));
    /// assert_eq!("@@$foo".parse::<IdentifierType>(), Err(ParseIdentifierError::new()));
    /// assert_eq!("@@0".parse::<IdentifierType>(), Err(ParseIdentifierError::new()));
    /// assert_eq!("@@1".parse::<IdentifierType>(), Err(ParseIdentifierError::new()));
    /// assert_eq!("@@9".parse::<IdentifierType>(), Err(ParseIdentifierError::new()));
    /// ```
    ///
    /// [`Constant`]: Self::Constant
    /// [`Local`]: Self::Local
    Class,
    /// Identifier that is an "attribute setter" method name.
    ///
    /// AttrSet end in the `=` sigil and are otherwise valid [`Local`] or
    /// [`Constant`] idents.  AttrSet idents cannot have any other "junk"
    /// symbols.
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_symbol::{IdentifierType, ParseIdentifierError};
    /// assert_eq!("Foo=".parse::<IdentifierType>(), Ok(IdentifierType::AttrSet));
    /// assert_eq!("foo=".parse::<IdentifierType>(), Ok(IdentifierType::AttrSet));
    /// assert_eq!("foo_bar=".parse::<IdentifierType>(), Ok(IdentifierType::AttrSet));
    /// assert_eq!("foo_bar?=".parse::<IdentifierType>(), Err(ParseIdentifierError::new()));
    /// assert_eq!("ω=".parse::<IdentifierType>(), Ok(IdentifierType::AttrSet));
    /// ```
    ///
    /// [`Constant`]: Self::Constant
    /// [`Local`]: Self::Local
    AttrSet,
    /// Identifier that is a constant name.
    ///
    /// Constant names can be either ASCII or Unicode and must start with a
    /// uppercase character.
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_symbol::{IdentifierType, ParseIdentifierError};
    /// assert_eq!("Foo".parse::<IdentifierType>(), Ok(IdentifierType::Constant));
    /// assert_eq!("FOO".parse::<IdentifierType>(), Ok(IdentifierType::Constant));
    /// assert_eq!("FooBar".parse::<IdentifierType>(), Ok(IdentifierType::Constant));
    /// assert_eq!("FOO_BAR".parse::<IdentifierType>(), Ok(IdentifierType::Constant));
    /// assert_eq!("Ω".parse::<IdentifierType>(), Ok(IdentifierType::Constant));
    /// ```
    Constant,
    /// Identifier that is a local variable or method name.
    ///
    /// Local names can be either ASCII or Unicode and must start with a
    /// lowercase character.
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_symbol::{IdentifierType, ParseIdentifierError};
    /// assert_eq!("foo".parse::<IdentifierType>(), Ok(IdentifierType::Local));
    /// assert_eq!("fOO".parse::<IdentifierType>(), Ok(IdentifierType::Local));
    /// assert_eq!("fooBar".parse::<IdentifierType>(), Ok(IdentifierType::Local));
    /// assert_eq!("foo_bar".parse::<IdentifierType>(), Ok(IdentifierType::Local));
    /// assert_eq!("ω".parse::<IdentifierType>(), Ok(IdentifierType::Local));
    /// ```
    Local,
}

impl IdentifierType {
    /// Return a new, default `IdentifierType`.
    ///
    /// Prefer to use `new()` over `default()` since `new()` is a const fn.
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_symbol::IdentifierType;
    /// const ID_TYPE: IdentifierType = IdentifierType::new();
    /// assert_eq!(ID_TYPE, IdentifierType::Junk);
    /// assert_eq!(ID_TYPE, IdentifierType::default());
    /// ```
    #[must_use]
    pub const fn new() -> Self {
        Self::Junk
    }
}

impl Default for IdentifierType {
    /// Construct a "junk" identifier type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_symbol::IdentifierType;
    /// const ID_TYPE: IdentifierType = IdentifierType::new();
    /// assert_eq!(ID_TYPE, IdentifierType::Junk);
    /// assert_eq!(ID_TYPE, IdentifierType::default());
    /// ```
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl FromStr for IdentifierType {
    type Err = ParseIdentifierError;

    #[inline]
    #[allow(clippy::or_fun_call)] // https://github.com/rust-lang/rust-clippy/issues/5886
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse(s.as_bytes()).ok_or(ParseIdentifierError::new())
    }
}

impl TryFrom<&str> for IdentifierType {
    type Error = ParseIdentifierError;

    #[inline]
    #[allow(clippy::or_fun_call)] // https://github.com/rust-lang/rust-clippy/issues/5886
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        parse(value.as_bytes()).ok_or(ParseIdentifierError::new())
    }
}

impl TryFrom<&[u8]> for IdentifierType {
    type Error = ParseIdentifierError;

    #[inline]
    #[allow(clippy::or_fun_call)] // https://github.com/rust-lang/rust-clippy/issues/5886
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        parse(value).ok_or(ParseIdentifierError::new())
    }
}

/// Error type returned from the [`FromStr`] implementation on [`IdentifierType`].
///
/// # Examples
///
/// ```
/// # use spinoso_symbol::{IdentifierType, ParseIdentifierError};
/// const ERR: ParseIdentifierError = ParseIdentifierError::new();
/// assert_eq!("not a valid ident".parse::<IdentifierType>(), Err(ERR));
/// ```
#[derive(Default, Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ParseIdentifierError {
    _private: (),
}

impl ParseIdentifierError {
    /// Construct a new `ParseIdentifierError`.
    ///
    /// Prefer to use `new()` over `default()` since `new()` is a const fn.
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_symbol::{IdentifierType, ParseIdentifierError};
    /// const ERR: ParseIdentifierError = ParseIdentifierError::new();
    /// assert_eq!("not a valid ident".parse::<IdentifierType>(), Err(ERR));
    /// assert_eq!(ERR, ParseIdentifierError::default());
    /// ```
    #[must_use]
    pub const fn new() -> Self {
        Self { _private: () }
    }
}

impl fmt::Display for ParseIdentifierError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Failed to parse given string as a known identifier type")
    }
}

#[inline]
fn parse(name: &[u8]) -> Option<IdentifierType> {
    match name {
        b"" | b"\0" => None,
        // special global variable
        [b'$', name @ ..] if is_special_global_name(name) => Some(IdentifierType::Global),
        // global vairable
        [b'$', name @ ..] => parse_ident(name, IdentifierType::Global),
        // class variable
        [b'@', b'@', name @ ..] => parse_ident(name, IdentifierType::Class),
        // instance variable
        [b'@', name @ ..] => parse_ident(name, IdentifierType::Instance),
        // Symbolic method names
        name if is_symbolic_method_name(name) => Some(IdentifierType::Junk),
        [b'=', ..] | [b'!', ..] | [b'[', ..] => None,
        [first, ..] if *first != b'_' && first.is_ascii() && !first.is_ascii_alphabetic() => None,
        // Constant name
        name if is_const_name(name) => parse_ident(name, IdentifierType::Constant),
        // Local variable
        name => parse_ident(name, IdentifierType::Local),
    }
}

#[inline]
fn parse_ident(name: &[u8], id_type: IdentifierType) -> Option<IdentifierType> {
    match name {
        b"" => None,
        [first, name @ .., b'=']
            if *first != b'_' && first.is_ascii() && !first.is_ascii_alphabetic() =>
        {
            if let None | Some(IdentifierType::AttrSet) = parse_ident(name, id_type) {
                None
            } else {
                Some(id_type)
            }
        }
        [first, ..] if *first != b'_' && first.is_ascii() && !first.is_ascii_alphabetic() => None,
        name if is_ident_until(name).is_none() => Some(id_type),
        [name @ .., b'!'] | [name @ .., b'?'] if is_ident_until(name).is_none() => {
            if matches!(
                id_type,
                IdentifierType::Global | IdentifierType::Class | IdentifierType::Instance
            ) {
                return None;
            }
            Some(IdentifierType::Junk)
        }
        [name @ .., b'='] if is_ident_until(name).is_none() => {
            if matches!(id_type, IdentifierType::Local | IdentifierType::Constant) {
                return Some(IdentifierType::AttrSet);
            }
            None
        }
        _ => None,
    }
}

#[inline]
fn is_special_global_name(name: &[u8]) -> bool {
    match name {
        b"" => false,
        [first, rest @ ..] if is_special_global_punct(*first) => rest.is_empty(),
        b"-" => false,
        [b'-', rest @ ..] if is_next_ident_exhausting(rest) => true,
        [b'-', ..] => false,
        name => name.char_indices().map(|idx| idx.2).all(char::is_numeric),
    }
}

/// Return whether the input is a "junk" symbolic method name.
///
/// There are fixed number of valid Ruby method names that only contain ASCII
/// symbols.
#[inline]
fn is_symbolic_method_name(name: &[u8]) -> bool {
    matches!(
        name,
        b"<" | b"<<"
            | b"<="
            | b"<=>"
            | b">"
            | b">>"
            | b">="
            | b"=~"
            | b"=="
            | b"==="
            | b"*"
            | b"**"
            | b"+"
            | b"-"
            | b"+@"
            | b"-@"
            | b"|"
            | b"^"
            | b"&"
            | b"/"
            | b"%"
            | b"~"
            | b"`"
            | b"[]"
            | b"[]="
            | b"!"
            | b"!="
            | b"!~"
    )
}

/// Return whther the input is a valid constant name.
///
/// Constant names require the first character to be either ASCII or Unicode
/// uppercase.
#[inline]
fn is_const_name(name: &[u8]) -> bool {
    match name {
        b"" => false,
        name if name.is_ascii() => name
            .iter()
            .next()
            .map(u8::is_ascii_uppercase)
            .unwrap_or_default(),
        name if name.is_utf8() => name
            .char_indices()
            .next()
            .map(|(_, _, ch)| ch.is_uppercase()) // uses Unicode `Uppercase` property
            .unwrap_or_default(),
        _ => false,
    }
}

/// Determine if a [`char`] can be used in a valid identifier.
///
/// # Header declaration
///
/// Ported from the following C macro in `string.c`:
///
/// ```c
/// #define is_identchar(p,e,enc) (ISALNUM((unsigned char)*(p)) || (*(p)) == '_' || !ISASCII(*(p)))
/// ```
#[inline]
fn is_ident_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_' || !ch.is_ascii()
}

/// Consume the input until a non-ident character is found.
///
/// Scan the [`char`]s in the input until either invalid UTF-8 or an invalid
/// ident is found. See [`is_ident_char`].
///
/// This method returns `Some(index)` of the start of the first invalid ident
/// or `None` if the whole input is a valid ident.
///
/// Empty slices are not valid idents.
#[inline]
fn is_ident_until(name: &[u8]) -> Option<usize> {
    if name.is_empty() {
        return Some(0);
    }
    for (start, _, ch) in name.char_indices() {
        if !is_ident_char(ch) {
            return Some(start);
        }
    }
    None
}

/// Determine if the next char is a valid ident char and consumes all bytes in
/// the input.
///
/// This function is used to determine whether certain kinds of single character
/// globals are valid idents.
///
/// See also [`is_ident_char`].
#[inline]
fn is_next_ident_exhausting(name: &[u8]) -> bool {
    let mut iter = name.char_indices();
    match iter.next() {
        Some((_, _, ch)) if is_ident_char(ch) => iter.next().is_none(),
        _ => false,
    }
}

// This function is defined by a macro in `parse.y` in MRI.
//
// ```c
// #define BIT(c, idx) (((c) / 32 - 1 == idx) ? (1U << ((c) % 32)) : 0)
// #define SPECIAL_PUNCT(idx) ( \
// 	BIT('~', idx) | BIT('*', idx) | BIT('$', idx) | BIT('?', idx) | \
// 	BIT('!', idx) | BIT('@', idx) | BIT('/', idx) | BIT('\\', idx) | \
// 	BIT(';', idx) | BIT(',', idx) | BIT('.', idx) | BIT('=', idx) | \
// 	BIT(':', idx) | BIT('<', idx) | BIT('>', idx) | BIT('\"', idx) | \
// 	BIT('&', idx) | BIT('`', idx) | BIT('\'', idx) | BIT('+', idx) | \
// 	BIT('0', idx))
// const unsigned int ruby_global_name_punct_bits[] = {
//     SPECIAL_PUNCT(0),
//     SPECIAL_PUNCT(1),
//     SPECIAL_PUNCT(2),
// };
// ```
//
// The contents of `ruby_global_name_punct_bits` are:
//
// ```console
// [2.6.6] > def bit(c, idx); c / 32 - 1 == idx ? 1 << (c % 32) : 0; end
// [2.6.6] > chars = ["~", "*", "$", "?", "!", "@", "/", "\\", ";", ",", ".", "=", ":", "<", ">", "\"", "&", "`", "'", "+", "0"]
//
// [2.6.6] > chars.map(&:ord).map { |ch| bit(ch, 0) }.reduce(0, :|)
// => 4227980502
// [2.6.6] > chars.map(&:ord).map { |ch| bit(ch, 1) }.reduce(0, :|)
// => 268435457
// [2.6.6] > chars.map(&:ord).map { |ch| bit(ch, 2) }.reduce(0, :|)
// => 1073741825
// ```
//
// Which corresponds to a fixed set of 21 ASCII symbols:
//
// ```ruby
// def is_special_global_punct(ch)
//   idx = (ch - 0x20) / 32;
//   case idx
//   when 0 then (4_227_980_502 >> (ch % 32)) & 1 > 0
//   when 1 then (268_435_457 >> (ch % 32)) & 1 > 0
//   when 2 then (1_073_741_825 >> (ch % 32)) & 1 > 0
//   else
//     false
//   end
// end
//
// h = {}
// (0..255).each do |ch|
//   h[ch.chr] = ch if is_special_global_punct(ch)
// end
// h.keys.map {|k| "b'#{k.inspect[1..-2]}'"}.join(" | ")
// ```
//
// TODO: Switch to generating this table inside the const function once const
// functions are expressive enough. This requires const `match`, `if`, and loop
// which will be stable in Rust 1.46.0.
#[inline]
fn is_special_global_punct(ch: u8) -> bool {
    matches!(
        ch,
        b'!' | b'"'
            | b'$'
            | b'&'
            | b'\''
            | b'*'
            | b'+'
            | b','
            | b'.'
            | b'/'
            | b'0'
            | b':'
            | b';'
            | b'<'
            | b'='
            | b'>'
            | b'?'
            | b'@'
            | b'\\'
            | b'`'
            | b'~'
    )
}

#[cfg(test)]
mod tests {
    use super::{IdentifierType, ParseIdentifierError};
    use core::convert::TryFrom;

    #[test]
    fn ascii_ident() {
        assert_eq!(
            "foobar".parse::<IdentifierType>(),
            Ok(IdentifierType::Local)
        );
        assert_eq!(
            "ruby_is_simple".parse::<IdentifierType>(),
            Ok(IdentifierType::Local)
        );
    }

    #[test]
    fn ascii_constant() {
        assert_eq!(
            "Foobar".parse::<IdentifierType>(),
            Ok(IdentifierType::Constant)
        );
        assert_eq!(
            "FooBar".parse::<IdentifierType>(),
            Ok(IdentifierType::Constant)
        );
        assert_eq!(
            "FOOBAR".parse::<IdentifierType>(),
            Ok(IdentifierType::Constant)
        );
        assert_eq!(
            "FOO_BAR".parse::<IdentifierType>(),
            Ok(IdentifierType::Constant)
        );
        assert_eq!(
            "RUBY_IS_SIMPLE".parse::<IdentifierType>(),
            Ok(IdentifierType::Constant)
        );
    }

    #[test]
    fn empty() {
        assert_eq!(
            "".parse::<IdentifierType>(),
            Err(ParseIdentifierError::new())
        );
    }

    #[test]
    fn single_nul() {
        assert_eq!(
            "\0".parse::<IdentifierType>(),
            Err(ParseIdentifierError::new())
        );
    }

    #[test]
    fn non_ascii_numerics() {
        assert_eq!("١".parse::<IdentifierType>(), Ok(IdentifierType::Local));
        assert_eq!(
            "١١١١١١١١١١١١١١١١١١".parse::<IdentifierType>(),
            Ok(IdentifierType::Local)
        );
        assert_eq!("①".parse::<IdentifierType>(), Ok(IdentifierType::Local));
    }

    #[test]
    fn recursive_ident() {
        assert_eq!(
            "@@@foo".parse::<IdentifierType>(),
            Err(ParseIdentifierError::new())
        );
        assert_eq!(
            "@@@@foo".parse::<IdentifierType>(),
            Err(ParseIdentifierError::new())
        );
        assert_eq!(
            "@$foo".parse::<IdentifierType>(),
            Err(ParseIdentifierError::new())
        );
        assert_eq!(
            "@$-w".parse::<IdentifierType>(),
            Err(ParseIdentifierError::new())
        );
        assert_eq!(
            "@@$foo".parse::<IdentifierType>(),
            Err(ParseIdentifierError::new())
        );
        assert_eq!(
            "@@$-w".parse::<IdentifierType>(),
            Err(ParseIdentifierError::new())
        );
        assert_eq!(
            "$@foo".parse::<IdentifierType>(),
            Err(ParseIdentifierError::new())
        );
        assert_eq!(
            "$@@foo".parse::<IdentifierType>(),
            Err(ParseIdentifierError::new())
        );
        assert_eq!(
            "$$-w".parse::<IdentifierType>(),
            Err(ParseIdentifierError::new())
        );
    }

    #[test]
    fn attr_bang() {
        assert_eq!(
            "@foo!".parse::<IdentifierType>(),
            Err(ParseIdentifierError::new())
        );
        assert_eq!(
            "@@foo!".parse::<IdentifierType>(),
            Err(ParseIdentifierError::new())
        );
        assert_eq!(
            "$foo!".parse::<IdentifierType>(),
            Err(ParseIdentifierError::new())
        );
    }

    #[test]
    fn attr_question() {
        assert_eq!(
            "@foo?".parse::<IdentifierType>(),
            Err(ParseIdentifierError::new())
        );
        assert_eq!(
            "@@foo?".parse::<IdentifierType>(),
            Err(ParseIdentifierError::new())
        );
        assert_eq!(
            "$foo?".parse::<IdentifierType>(),
            Err(ParseIdentifierError::new())
        );
    }

    #[test]
    fn attr_setter() {
        assert_eq!(
            "@foo=".parse::<IdentifierType>(),
            Err(ParseIdentifierError::new())
        );
        assert_eq!(
            "@@foo=".parse::<IdentifierType>(),
            Err(ParseIdentifierError::new())
        );
        assert_eq!(
            "$foo=".parse::<IdentifierType>(),
            Err(ParseIdentifierError::new())
        );
    }

    #[test]
    fn invalid_utf8() {
        assert_eq!(
            IdentifierType::try_from(&b"invalid-\xFF-utf8"[..]),
            Err(ParseIdentifierError::new())
        );
    }

    #[test]
    fn emoji() {
        assert_eq!(IdentifierType::try_from("💎"), Ok(IdentifierType::Local));
        assert_eq!(IdentifierType::try_from("$💎"), Ok(IdentifierType::Global));
    }
}

#[cfg(test)]
mod specs {
    use super::IdentifierType;

    // From spec/core/symbol/inspect_spec.rb:
    //
    // ```ruby
    // symbols = {
    //   fred:         ":fred",
    //   :fred?     => ":fred?",
    //   :fred!     => ":fred!",
    //   :$ruby     => ":$ruby",
    //   :@ruby     => ":@ruby",
    //   :@@ruby    => ":@@ruby",
    //   :"$ruby!"  => ":\"$ruby!\"",
    //   :"$ruby?"  => ":\"$ruby?\"",
    //   :"@ruby!"  => ":\"@ruby!\"",
    //   :"@ruby?"  => ":\"@ruby?\"",
    //   :"@@ruby!" => ":\"@@ruby!\"",
    //   :"@@ruby?" => ":\"@@ruby?\"",
    //
    //   :$-w       => ":$-w",
    //   :"$-ww"    => ":\"$-ww\"",
    //   :"$+"      => ":$+",
    //   :"$~"      => ":$~",
    //   :"$:"      => ":$:",
    //   :"$?"      => ":$?",
    //   :"$<"      => ":$<",
    //   :"$_"      => ":$_",
    //   :"$/"      => ":$/",
    //   :"$'"      => ":$'",
    //   :"$\""     => ":$\"",
    //   :"$$"      => ":$$",
    //   :"$."      => ":$.",
    //   :"$,"      => ":$,",
    //   :"$`"      => ":$`",
    //   :"$!"      => ":$!",
    //   :"$;"      => ":$;",
    //   :"$\\"     => ":$\\",
    //   :"$="      => ":$=",
    //   :"$*"      => ":$*",
    //   :"$>"      => ":$>",
    //   :"$&"      => ":$&",
    //   :"$@"      => ":$@",
    //   :"$1234"   => ":$1234",
    //
    //   :-@        => ":-@",
    //   :+@        => ":+@",
    //   :%         => ":%",
    //   :&         => ":&",
    //   :*         => ":*",
    //   :**        => ":**",
    //   :"/"       => ":/",     # lhs quoted for emacs happiness
    //   :<         => ":<",
    //   :<=        => ":<=",
    //   :<=>       => ":<=>",
    //   :==        => ":==",
    //   :===       => ":===",
    //   :=~        => ":=~",
    //   :>         => ":>",
    //   :>=        => ":>=",
    //   :>>        => ":>>",
    //   :[]        => ":[]",
    //   :[]=       => ":[]=",
    //   :"\<\<"    => ":\<\<",
    //   :^         => ":^",
    //   :"`"       => ":`",     # for emacs, and justice!
    //   :~         => ":~",
    //   :|         => ":|",
    //
    //   :"!"       => [":\"!\"",  ":!" ],
    //   :"!="      => [":\"!=\"", ":!="],
    //   :"!~"      => [":\"!~\"", ":!~"],
    //   :"\$"      => ":\"$\"", # for justice!
    //   :"&&"      => ":\"&&\"",
    //   :"'"       => ":\"\'\"",
    //   :","       => ":\",\"",
    //   :"."       => ":\".\"",
    //   :".."      => ":\"..\"",
    //   :"..."     => ":\"...\"",
    //   :":"       => ":\":\"",
    //   :"::"      => ":\"::\"",
    //   :";"       => ":\";\"",
    //   :"="       => ":\"=\"",
    //   :"=>"      => ":\"=>\"",
    //   :"\?"      => ":\"?\"", # rawr!
    //   :"@"       => ":\"@\"",
    //   :"||"      => ":\"||\"",
    //   :"|||"     => ":\"|||\"",
    //   :"++"      => ":\"++\"",
    //
    //   :"\""      => ":\"\\\"\"",
    //   :"\"\""    => ":\"\\\"\\\"\"",
    //
    //   :"9"       => ":\"9\"",
    //   :"foo bar" => ":\"foo bar\"",
    //   :"*foo"    => ":\"*foo\"",
    //   :"foo "    => ":\"foo \"",
    //   :" foo"    => ":\" foo\"",
    //   :" "       => ":\" \"",
    // }
    // ```

    #[test]
    fn specs() {
        // idents
        assert!("fred".parse::<IdentifierType>().is_ok());
        assert!("fred?".parse::<IdentifierType>().is_ok());
        assert!("fred!".parse::<IdentifierType>().is_ok());
        assert!("$ruby".parse::<IdentifierType>().is_ok());
        assert!("@ruby".parse::<IdentifierType>().is_ok());
        assert!("@@ruby".parse::<IdentifierType>().is_ok());

        // idents can't end in bang or question
        assert!("$ruby!".parse::<IdentifierType>().is_err());
        assert!("$ruby?".parse::<IdentifierType>().is_err());
        assert!("@ruby!".parse::<IdentifierType>().is_err());
        assert!("@ruby?".parse::<IdentifierType>().is_err());
        assert!("@@ruby!".parse::<IdentifierType>().is_err());
        assert!("@@ruby?".parse::<IdentifierType>().is_err());

        // globals
        assert!("$-w".parse::<IdentifierType>().is_ok());
        assert!("$-ww".parse::<IdentifierType>().is_err());
        assert!("$+".parse::<IdentifierType>().is_ok());
        assert!("$~".parse::<IdentifierType>().is_ok());
        assert!("$:".parse::<IdentifierType>().is_ok());
        assert!("$?".parse::<IdentifierType>().is_ok());
        assert!("$<".parse::<IdentifierType>().is_ok());
        assert!("$_".parse::<IdentifierType>().is_ok());
        assert!("$/".parse::<IdentifierType>().is_ok());
        assert!("$\"".parse::<IdentifierType>().is_ok());
        assert!("$$".parse::<IdentifierType>().is_ok());
        assert!("$.".parse::<IdentifierType>().is_ok());
        assert!("$,".parse::<IdentifierType>().is_ok());
        assert!("$`".parse::<IdentifierType>().is_ok());
        assert!("$!".parse::<IdentifierType>().is_ok());
        assert!("$;".parse::<IdentifierType>().is_ok());
        assert!("$\\".parse::<IdentifierType>().is_ok());
        assert!("$=".parse::<IdentifierType>().is_ok());
        assert!("$*".parse::<IdentifierType>().is_ok());
        assert!("$>".parse::<IdentifierType>().is_ok());
        assert!("$&".parse::<IdentifierType>().is_ok());
        assert!("$@".parse::<IdentifierType>().is_ok());
        assert!("$1234".parse::<IdentifierType>().is_ok());

        // symbolic methods
        assert!("-@".parse::<IdentifierType>().is_ok());
        assert!("+@".parse::<IdentifierType>().is_ok());
        assert!("%".parse::<IdentifierType>().is_ok());
        assert!("&".parse::<IdentifierType>().is_ok());
        assert!("*".parse::<IdentifierType>().is_ok());
        assert!("**".parse::<IdentifierType>().is_ok());
        assert!("/".parse::<IdentifierType>().is_ok());
        assert!("<".parse::<IdentifierType>().is_ok());
        assert!("<=".parse::<IdentifierType>().is_ok());
        assert!("<=>".parse::<IdentifierType>().is_ok());
        assert!("==".parse::<IdentifierType>().is_ok());
        assert!("===".parse::<IdentifierType>().is_ok());
        assert!("=~".parse::<IdentifierType>().is_ok());
        assert!(">".parse::<IdentifierType>().is_ok());
        assert!(">=".parse::<IdentifierType>().is_ok());
        assert!(">>".parse::<IdentifierType>().is_ok());
        assert!("[]".parse::<IdentifierType>().is_ok());
        assert!("[]=".parse::<IdentifierType>().is_ok());
        assert!("<<".parse::<IdentifierType>().is_ok());
        assert!("^".parse::<IdentifierType>().is_ok());
        assert!("`".parse::<IdentifierType>().is_ok());
        assert!("~".parse::<IdentifierType>().is_ok());
        assert!("|".parse::<IdentifierType>().is_ok());

        // non-symbol symbolics
        assert!("!".parse::<IdentifierType>().is_ok());
        assert!("!=".parse::<IdentifierType>().is_ok());
        assert!("!~".parse::<IdentifierType>().is_ok());
        assert!("$".parse::<IdentifierType>().is_err());
        assert!("&&".parse::<IdentifierType>().is_err());
        assert!("'".parse::<IdentifierType>().is_err());
        assert!(",".parse::<IdentifierType>().is_err());
        assert!(".".parse::<IdentifierType>().is_err());
        assert!("..".parse::<IdentifierType>().is_err());
        assert!("...".parse::<IdentifierType>().is_err());
        assert!(":".parse::<IdentifierType>().is_err());
        assert!("::".parse::<IdentifierType>().is_err());
        assert!(";".parse::<IdentifierType>().is_err());
        assert!("=".parse::<IdentifierType>().is_err());
        assert!("=>".parse::<IdentifierType>().is_err());
        assert!("?".parse::<IdentifierType>().is_err());
        assert!("@".parse::<IdentifierType>().is_err());
        assert!("||".parse::<IdentifierType>().is_err());
        assert!("|||".parse::<IdentifierType>().is_err());
        assert!("++".parse::<IdentifierType>().is_err());

        // quotes
        assert!(r#"""#.parse::<IdentifierType>().is_err());
        assert!(r#""""#.parse::<IdentifierType>().is_err());

        assert!("9".parse::<IdentifierType>().is_err());
        assert!("foo bar".parse::<IdentifierType>().is_err());
        assert!("*foo".parse::<IdentifierType>().is_err());
        assert!("foo ".parse::<IdentifierType>().is_err());
        assert!(" foo".parse::<IdentifierType>().is_err());
        assert!(" ".parse::<IdentifierType>().is_err());
    }
}

/// Tests generated from symbols loaded at MRI interpreter boot.
///
/// # Generation
///
/// ```shell
/// cat <<EOF | ruby --disable-gems --disable-did_you_mean
/// def boot_identifier_symbols
///   syms = Symbol.all_symbols.map(&:inspect)
///   # remove symbols that must be debug wrapped in quotes
///
///   syms = syms.reject { |s| s[0..1] == ':"' }
///   fixture = syms.map { |s| "r##\"#{s[1..]}\"##" }
///   puts fixture.join(",\n")
/// end
///
/// boot_identifier_symbols
/// EOF
/// ```
#[cfg(test)]
mod functionals {
    use super::IdentifierType;

    // These Symbols are the result of calling `Symbol.all_symbols` after MRI
    // interpreter boot.
    const IDENTS: &[&str] = &[
        r##"!"##,
        r##"%"##,
        r##"&"##,
        r##"*"##,
        r##"+"##,
        r##"-"##,
        r##"/"##,
        r##"<"##,
        r##">"##,
        r##"^"##,
        r##"`"##,
        r##"|"##,
        r##"~"##,
        r##"+@"##,
        r##"-@"##,
        r##"**"##,
        r##"<=>"##,
        r##"<<"##,
        r##">>"##,
        r##"<="##,
        r##">="##,
        r##"=="##,
        r##"==="##,
        r##"!="##,
        r##"=~"##,
        r##"!~"##,
        r##"[]"##,
        r##"[]="##,
        r##"max"##,
        r##"min"##,
        r##"freeze"##,
        r##"inspect"##,
        r##"intern"##,
        r##"object_id"##,
        r##"const_missing"##,
        r##"method_missing"##,
        r##"method_added"##,
        r##"singleton_method_added"##,
        r##"method_removed"##,
        r##"singleton_method_removed"##,
        r##"method_undefined"##,
        r##"singleton_method_undefined"##,
        r##"length"##,
        r##"size"##,
        r##"gets"##,
        r##"succ"##,
        r##"each"##,
        r##"proc"##,
        r##"lambda"##,
        r##"send"##,
        r##"__send__"##,
        r##"__attached__"##,
        r##"initialize"##,
        r##"initialize_copy"##,
        r##"initialize_clone"##,
        r##"initialize_dup"##,
        r##"to_int"##,
        r##"to_ary"##,
        r##"to_str"##,
        r##"to_sym"##,
        r##"to_hash"##,
        r##"to_proc"##,
        r##"to_io"##,
        r##"to_a"##,
        r##"to_s"##,
        r##"to_i"##,
        r##"to_f"##,
        r##"to_r"##,
        r##"bt"##,
        r##"bt_locations"##,
        r##"call"##,
        r##"mesg"##,
        r##"exception"##,
        r##"not"##,
        r##"and"##,
        r##"or"##,
        r##"_"##,
        r##"empty?"##,
        r##"eql?"##,
        r##"respond_to?"##,
        r##"respond_to_missing?"##,
        r##"$_"##,
        r##"$~"##,
        r##"__autoload__"##,
        r##"__classpath__"##,
        r##"__tmp_classpath__"##,
        r##"__classid__"##,
        r##"dig"##,
        r##"BasicObject"##,
        r##"Object"##,
        r##"Module"##,
        r##"Class"##,
        r##"equal?"##,
        r##"Kernel"##,
        r##"__refined_class__"##,
        r##"inherited"##,
        r##"included"##,
        r##"extended"##,
        r##"prepended"##,
        r##"nil?"##,
        r##"hash"##,
        r##"class"##,
        r##"singleton_class"##,
        r##"clone"##,
        r##"dup"##,
        r##"itself"##,
        r##"yield_self"##,
        r##"then"##,
        r##"taint"##,
        r##"tainted?"##,
        r##"untaint"##,
        r##"untrust"##,
        r##"untrusted?"##,
        r##"trust"##,
        r##"frozen?"##,
        r##"methods"##,
        r##"singleton_methods"##,
        r##"protected_methods"##,
        r##"private_methods"##,
        r##"public_methods"##,
        r##"instance_variables"##,
        r##"instance_variable_get"##,
        r##"instance_variable_set"##,
        r##"instance_variable_defined?"##,
        r##"remove_instance_variable"##,
        r##"instance_of?"##,
        r##"kind_of?"##,
        r##"is_a?"##,
        r##"tap"##,
        r##"sprintf"##,
        r##"format"##,
        r##"Integer"##,
        r##"Float"##,
        r##"String"##,
        r##"Array"##,
        r##"Hash"##,
        r##"NilClass"##,
        r##"to_h"##,
        r##"new"##,
        r##"NIL"##,
        r##"included_modules"##,
        r##"include?"##,
        r##"name"##,
        r##"ancestors"##,
        r##"attr"##,
        r##"attr_reader"##,
        r##"attr_writer"##,
        r##"attr_accessor"##,
        r##"instance_methods"##,
        r##"public_instance_methods"##,
        r##"protected_instance_methods"##,
        r##"private_instance_methods"##,
        r##"constants"##,
        r##"const_get"##,
        r##"const_set"##,
        r##"const_defined?"##,
        r##"remove_const"##,
        r##"class_variables"##,
        r##"remove_class_variable"##,
        r##"class_variable_get"##,
        r##"class_variable_set"##,
        r##"class_variable_defined?"##,
        r##"public_constant"##,
        r##"private_constant"##,
        r##"deprecate_constant"##,
        r##"singleton_class?"##,
        r##"allocate"##,
        r##"superclass"##,
        r##"extend_object"##,
        r##"append_features"##,
        r##"prepend_features"##,
        r##"Data"##,
        r##"TrueClass"##,
        r##"TRUE"##,
        r##"FalseClass"##,
        r##"FALSE"##,
        r##"Encoding"##,
        r##"names"##,
        r##"dummy?"##,
        r##"ascii_compatible?"##,
        r##"replicate"##,
        r##"list"##,
        r##"name_list"##,
        r##"aliases"##,
        r##"find"##,
        r##"compatible?"##,
        r##"_dump"##,
        r##"_load"##,
        r##"default_external"##,
        r##"default_external="##,
        r##"default_internal"##,
        r##"default_internal="##,
        r##"locale_charmap"##,
        r##"Comparable"##,
        r##"between?"##,
        r##"clamp"##,
        r##"Enumerable"##,
        r##"entries"##,
        r##"sort"##,
        r##"sort_by"##,
        r##"grep"##,
        r##"grep_v"##,
        r##"count"##,
        r##"detect"##,
        r##"find_index"##,
        r##"find_all"##,
        r##"select"##,
        r##"filter"##,
        r##"reject"##,
        r##"collect"##,
        r##"map"##,
        r##"flat_map"##,
        r##"collect_concat"##,
        r##"inject"##,
        r##"reduce"##,
        r##"partition"##,
        r##"group_by"##,
        r##"first"##,
        r##"all?"##,
        r##"any?"##,
        r##"one?"##,
        r##"none?"##,
        r##"minmax"##,
        r##"min_by"##,
        r##"max_by"##,
        r##"minmax_by"##,
        r##"member?"##,
        r##"each_with_index"##,
        r##"reverse_each"##,
        r##"each_entry"##,
        r##"each_slice"##,
        r##"each_cons"##,
        r##"each_with_object"##,
        r##"zip"##,
        r##"take"##,
        r##"take_while"##,
        r##"drop"##,
        r##"drop_while"##,
        r##"cycle"##,
        r##"chunk"##,
        r##"slice_before"##,
        r##"slice_after"##,
        r##"slice_when"##,
        r##"chunk_while"##,
        r##"sum"##,
        r##"uniq"##,
        r##"next"##,
        r##"div"##,
        r##"try_convert"##,
        r##"casecmp"##,
        r##"casecmp?"##,
        r##"insert"##,
        r##"bytesize"##,
        r##"match"##,
        r##"match?"##,
        r##"succ!"##,
        r##"next!"##,
        r##"upto"##,
        r##"index"##,
        r##"rindex"##,
        r##"replace"##,
        r##"clear"##,
        r##"chr"##,
        r##"getbyte"##,
        r##"setbyte"##,
        r##"byteslice"##,
        r##"scrub"##,
        r##"scrub!"##,
        r##"dump"##,
        r##"undump"##,
        r##"ascii"##,
        r##"turkic"##,
        r##"lithuanian"##,
        r##"fold"##,
        r##"upcase"##,
        r##"downcase"##,
        r##"capitalize"##,
        r##"swapcase"##,
        r##"upcase!"##,
        r##"downcase!"##,
        r##"capitalize!"##,
        r##"swapcase!"##,
        r##"hex"##,
        r##"oct"##,
        r##"split"##,
        r##"lines"##,
        r##"bytes"##,
        r##"chars"##,
        r##"codepoints"##,
        r##"grapheme_clusters"##,
        r##"reverse"##,
        r##"reverse!"##,
        r##"concat"##,
        r##"prepend"##,
        r##"crypt"##,
        r##"ord"##,
        r##"start_with?"##,
        r##"end_with?"##,
        r##"scan"##,
        r##"ljust"##,
        r##"rjust"##,
        r##"center"##,
        r##"sub"##,
        r##"gsub"##,
        r##"chop"##,
        r##"chomp"##,
        r##"strip"##,
        r##"lstrip"##,
        r##"rstrip"##,
        r##"delete_prefix"##,
        r##"delete_suffix"##,
        r##"sub!"##,
        r##"gsub!"##,
        r##"chop!"##,
        r##"chomp!"##,
        r##"strip!"##,
        r##"lstrip!"##,
        r##"rstrip!"##,
        r##"delete_prefix!"##,
        r##"delete_suffix!"##,
        r##"tr"##,
        r##"tr_s"##,
        r##"delete"##,
        r##"squeeze"##,
        r##"tr!"##,
        r##"tr_s!"##,
        r##"delete!"##,
        r##"squeeze!"##,
        r##"each_line"##,
        r##"each_byte"##,
        r##"each_char"##,
        r##"each_codepoint"##,
        r##"each_grapheme_cluster"##,
        r##"slice"##,
        r##"slice!"##,
        r##"rpartition"##,
        r##"encoding"##,
        r##"force_encoding"##,
        r##"b"##,
        r##"valid_encoding?"##,
        r##"ascii_only?"##,
        r##"UnicodeNormalize"##,
        r##"normalize"##,
        r##"normalized?"##,
        r##"unicode_normalize"##,
        r##"unicode_normalize!"##,
        r##"unicode_normalized?"##,
        r##"$;"##,
        r##"$-F"##,
        r##"Symbol"##,
        r##"all_symbols"##,
        r##"id2name"##,
        r##"Exception"##,
        r##"to_tty?"##,
        r##"message"##,
        r##"full_message"##,
        r##"backtrace"##,
        r##"backtrace_locations"##,
        r##"set_backtrace"##,
        r##"cause"##,
        r##"SystemExit"##,
        r##"status"##,
        r##"success?"##,
        r##"fatal"##,
        r##"SignalException"##,
        r##"Interrupt"##,
        r##"StandardError"##,
        r##"TypeError"##,
        r##"ArgumentError"##,
        r##"IndexError"##,
        r##"KeyError"##,
        r##"receiver"##,
        r##"key"##,
        r##"RangeError"##,
        r##"ScriptError"##,
        r##"SyntaxError"##,
        r##"LoadError"##,
        r##"path"##,
        r##"@path"##,
        r##"NotImplementedError"##,
        r##"NameError"##,
        r##"local_variables"##,
        r##"NoMethodError"##,
        r##"args"##,
        r##"private_call?"##,
        r##"RuntimeError"##,
        r##"FrozenError"##,
        r##"SecurityError"##,
        r##"NoMemoryError"##,
        r##"EncodingError"##,
        r##"CompatibilityError"##,
        r##"SystemCallError"##,
        r##"errno"##,
        r##"Errno"##,
        r##"Warning"##,
        r##"warn"##,
        r##"buffer"##,
        r##"write"##,
        r##"top"##,
        r##"bottom"##,
        r##"$@"##,
        r##"$!"##,
        r##"raise"##,
        r##"fail"##,
        r##"global_variables"##,
        r##"__method__"##,
        r##"__callee__"##,
        r##"__dir__"##,
        r##"include"##,
        r##"refine"##,
        r##"using"##,
        r##"used_modules"##,
        r##"module_function"##,
        r##"eval"##,
        r##"iterator?"##,
        r##"block_given?"##,
        r##"catch"##,
        r##"throw"##,
        r##"loop"##,
        r##"instance_eval"##,
        r##"instance_exec"##,
        r##"public_send"##,
        r##"module_exec"##,
        r##"class_exec"##,
        r##"module_eval"##,
        r##"class_eval"##,
        r##"UncaughtThrowError"##,
        r##"tag"##,
        r##"value"##,
        r##"result"##,
        r##"remove_method"##,
        r##"undef_method"##,
        r##"alias_method"##,
        r##"public"##,
        r##"protected"##,
        r##"private"##,
        r##"method_defined?"##,
        r##"public_method_defined?"##,
        r##"private_method_defined?"##,
        r##"protected_method_defined?"##,
        r##"public_class_method"##,
        r##"private_class_method"##,
        r##"nesting"##,
        r##"extend"##,
        r##"trace_var"##,
        r##"untrace_var"##,
        r##"signo"##,
        r##"$SAFE"##,
        r##"at_exit"##,
        r##"coerce"##,
        r##"divmod"##,
        r##"ZeroDivisionError"##,
        r##"FloatDomainError"##,
        r##"Numeric"##,
        r##"i"##,
        r##"fdiv"##,
        r##"modulo"##,
        r##"remainder"##,
        r##"abs"##,
        r##"magnitude"##,
        r##"real?"##,
        r##"integer?"##,
        r##"zero?"##,
        r##"nonzero?"##,
        r##"finite?"##,
        r##"infinite?"##,
        r##"floor"##,
        r##"ceil"##,
        r##"round"##,
        r##"truncate"##,
        r##"step"##,
        r##"positive?"##,
        r##"negative?"##,
        r##"sqrt"##,
        r##"odd?"##,
        r##"even?"##,
        r##"allbits?"##,
        r##"anybits?"##,
        r##"nobits?"##,
        r##"downto"##,
        r##"times"##,
        r##"pred"##,
        r##"pow"##,
        r##"bit_length"##,
        r##"digits"##,
        r##"Fixnum"##,
        r##"ROUNDS"##,
        r##"RADIX"##,
        r##"MANT_DIG"##,
        r##"DIG"##,
        r##"MIN_EXP"##,
        r##"MAX_EXP"##,
        r##"MIN_10_EXP"##,
        r##"MAX_10_EXP"##,
        r##"MIN"##,
        r##"MAX"##,
        r##"EPSILON"##,
        r##"INFINITY"##,
        r##"NAN"##,
        r##"quo"##,
        r##"nan?"##,
        r##"next_float"##,
        r##"prev_float"##,
        r##"to"##,
        r##"by"##,
        r##"Bignum"##,
        r##"GMP_VERSION"##,
        r##"NOERROR"##,
        r##"E2BIG"##,
        r##"EACCES"##,
        r##"EADDRINUSE"##,
        r##"EADDRNOTAVAIL"##,
        r##"EADV"##,
        r##"EAFNOSUPPORT"##,
        r##"EAGAIN"##,
        r##"EALREADY"##,
        r##"EAUTH"##,
        r##"EBADARCH"##,
        r##"EBADE"##,
        r##"EBADEXEC"##,
        r##"EBADF"##,
        r##"EBADFD"##,
        r##"EBADMACHO"##,
        r##"EBADMSG"##,
        r##"EBADR"##,
        r##"EBADRPC"##,
        r##"EBADRQC"##,
        r##"EBADSLT"##,
        r##"EBFONT"##,
        r##"EBUSY"##,
        r##"ECANCELED"##,
        r##"ECAPMODE"##,
        r##"ECHILD"##,
        r##"ECHRNG"##,
        r##"ECOMM"##,
        r##"ECONNABORTED"##,
        r##"ECONNREFUSED"##,
        r##"ECONNRESET"##,
        r##"EDEADLK"##,
        r##"EDEADLOCK"##,
        r##"EDESTADDRREQ"##,
        r##"EDEVERR"##,
        r##"EDOM"##,
        r##"EDOOFUS"##,
        r##"EDOTDOT"##,
        r##"EDQUOT"##,
        r##"EEXIST"##,
        r##"EFAULT"##,
        r##"EFBIG"##,
        r##"EFTYPE"##,
        r##"EHOSTDOWN"##,
        r##"EHOSTUNREACH"##,
        r##"EHWPOISON"##,
        r##"EIDRM"##,
        r##"EILSEQ"##,
        r##"EINPROGRESS"##,
        r##"EINTR"##,
        r##"EINVAL"##,
        r##"EIO"##,
        r##"EIPSEC"##,
        r##"EISCONN"##,
        r##"EISDIR"##,
        r##"EISNAM"##,
        r##"EKEYEXPIRED"##,
        r##"EKEYREJECTED"##,
        r##"EKEYREVOKED"##,
        r##"EL2HLT"##,
        r##"EL2NSYNC"##,
        r##"EL3HLT"##,
        r##"EL3RST"##,
        r##"ELAST"##,
        r##"ELIBACC"##,
        r##"ELIBBAD"##,
        r##"ELIBEXEC"##,
        r##"ELIBMAX"##,
        r##"ELIBSCN"##,
        r##"ELNRNG"##,
        r##"ELOOP"##,
        r##"EMEDIUMTYPE"##,
        r##"EMFILE"##,
        r##"EMLINK"##,
        r##"EMSGSIZE"##,
        r##"EMULTIHOP"##,
        r##"ENAMETOOLONG"##,
        r##"ENAVAIL"##,
        r##"ENEEDAUTH"##,
        r##"ENETDOWN"##,
        r##"ENETRESET"##,
        r##"ENETUNREACH"##,
        r##"ENFILE"##,
        r##"ENOANO"##,
        r##"ENOATTR"##,
        r##"ENOBUFS"##,
        r##"ENOCSI"##,
        r##"ENODATA"##,
        r##"ENODEV"##,
        r##"ENOENT"##,
        r##"ENOEXEC"##,
        r##"ENOKEY"##,
        r##"ENOLCK"##,
        r##"ENOLINK"##,
        r##"ENOMEDIUM"##,
        r##"ENOMEM"##,
        r##"ENOMSG"##,
        r##"ENONET"##,
        r##"ENOPKG"##,
        r##"ENOPOLICY"##,
        r##"ENOPROTOOPT"##,
        r##"ENOSPC"##,
        r##"ENOSR"##,
        r##"ENOSTR"##,
        r##"ENOSYS"##,
        r##"ENOTBLK"##,
        r##"ENOTCAPABLE"##,
        r##"ENOTCONN"##,
        r##"ENOTDIR"##,
        r##"ENOTEMPTY"##,
        r##"ENOTNAM"##,
        r##"ENOTRECOVERABLE"##,
        r##"ENOTSOCK"##,
        r##"ENOTSUP"##,
        r##"ENOTTY"##,
        r##"ENOTUNIQ"##,
        r##"ENXIO"##,
        r##"EOPNOTSUPP"##,
        r##"EOVERFLOW"##,
        r##"EOWNERDEAD"##,
        r##"EPERM"##,
        r##"EPFNOSUPPORT"##,
        r##"EPIPE"##,
        r##"EPROCLIM"##,
        r##"EPROCUNAVAIL"##,
        r##"EPROGMISMATCH"##,
        r##"EPROGUNAVAIL"##,
        r##"EPROTO"##,
        r##"EPROTONOSUPPORT"##,
        r##"EPROTOTYPE"##,
        r##"EPWROFF"##,
        r##"EQFULL"##,
        r##"ERANGE"##,
        r##"EREMCHG"##,
        r##"EREMOTE"##,
        r##"EREMOTEIO"##,
        r##"ERESTART"##,
        r##"ERFKILL"##,
        r##"EROFS"##,
        r##"ERPCMISMATCH"##,
        r##"ESHLIBVERS"##,
        r##"ESHUTDOWN"##,
        r##"ESOCKTNOSUPPORT"##,
        r##"ESPIPE"##,
        r##"ESRCH"##,
        r##"ESRMNT"##,
        r##"ESTALE"##,
        r##"ESTRPIPE"##,
        r##"ETIME"##,
        r##"ETIMEDOUT"##,
        r##"ETOOMANYREFS"##,
        r##"ETXTBSY"##,
        r##"EUCLEAN"##,
        r##"EUNATCH"##,
        r##"EUSERS"##,
        r##"EWOULDBLOCK"##,
        r##"EXDEV"##,
        r##"EXFULL"##,
        r##"at"##,
        r##"fetch"##,
        r##"last"##,
        r##"union"##,
        r##"difference"##,
        r##"push"##,
        r##"append"##,
        r##"pop"##,
        r##"shift"##,
        r##"unshift"##,
        r##"each_index"##,
        r##"join"##,
        r##"rotate"##,
        r##"rotate!"##,
        r##"sort!"##,
        r##"sort_by!"##,
        r##"collect!"##,
        r##"map!"##,
        r##"select!"##,
        r##"filter!"##,
        r##"keep_if"##,
        r##"values_at"##,
        r##"delete_at"##,
        r##"delete_if"##,
        r##"reject!"##,
        r##"transpose"##,
        r##"fill"##,
        r##"assoc"##,
        r##"rassoc"##,
        r##"uniq!"##,
        r##"compact"##,
        r##"compact!"##,
        r##"flatten"##,
        r##"flatten!"##,
        r##"shuffle!"##,
        r##"shuffle"##,
        r##"sample"##,
        r##"permutation"##,
        r##"combination"##,
        r##"repeated_permutation"##,
        r##"repeated_combination"##,
        r##"product"##,
        r##"bsearch"##,
        r##"bsearch_index"##,
        r##"random"##,
        r##"yield"##,
        r##"default"##,
        r##"rehash"##,
        r##"store"##,
        r##"default="##,
        r##"default_proc"##,
        r##"default_proc="##,
        r##"each_value"##,
        r##"each_key"##,
        r##"each_pair"##,
        r##"transform_keys"##,
        r##"transform_keys!"##,
        r##"transform_values"##,
        r##"transform_values!"##,
        r##"keys"##,
        r##"values"##,
        r##"fetch_values"##,
        r##"invert"##,
        r##"update"##,
        r##"merge!"##,
        r##"merge"##,
        r##"has_key?"##,
        r##"has_value?"##,
        r##"key?"##,
        r##"value?"##,
        r##"compare_by_identity"##,
        r##"compare_by_identity?"##,
        r##"ENV"##,
        r##"__members__"##,
        r##"__members_back__"##,
        r##"__keyword_init__"##,
        r##"Struct"##,
        r##"members"##,
        r##"RegexpError"##,
        r##"$&"##,
        r##"$`"##,
        r##"$'"##,
        r##"$+"##,
        r##"$="##,
        r##"$KCODE"##,
        r##"$-K"##,
        r##"Regexp"##,
        r##"compile"##,
        r##"quote"##,
        r##"escape"##,
        r##"last_match"##,
        r##"source"##,
        r##"casefold?"##,
        r##"options"##,
        r##"fixed_encoding?"##,
        r##"named_captures"##,
        r##"IGNORECASE"##,
        r##"EXTENDED"##,
        r##"MULTILINE"##,
        r##"FIXEDENCODING"##,
        r##"NOENCODING"##,
        r##"MatchData"##,
        r##"regexp"##,
        r##"offset"##,
        r##"begin"##,
        r##"end"##,
        r##"captures"##,
        r##"pre_match"##,
        r##"post_match"##,
        r##"string"##,
        r##"pack"##,
        r##"unpack"##,
        r##"unpack1"##,
        r##"invalid"##,
        r##"undef"##,
        r##"fallback"##,
        r##"xml"##,
        r##"text"##,
        r##"invalid_byte_sequence"##,
        r##"undefined_conversion"##,
        r##"destination_buffer_full"##,
        r##"source_buffer_empty"##,
        r##"finished"##,
        r##"after_output"##,
        r##"incomplete_input"##,
        r##"universal_newline"##,
        r##"crlf_newline"##,
        r##"cr_newline"##,
        r##"partial_input"##,
        r##"newline"##,
        r##"universal"##,
        r##"crlf"##,
        r##"cr"##,
        r##"lf"##,
        r##"UndefinedConversionError"##,
        r##"InvalidByteSequenceError"##,
        r##"ConverterNotFoundError"##,
        r##"encode"##,
        r##"encode!"##,
        r##"Converter"##,
        r##"asciicompat_encoding"##,
        r##"search_convpath"##,
        r##"convpath"##,
        r##"source_encoding"##,
        r##"destination_encoding"##,
        r##"primitive_convert"##,
        r##"convert"##,
        r##"finish"##,
        r##"primitive_errinfo"##,
        r##"insert_output"##,
        r##"putback"##,
        r##"last_error"##,
        r##"replacement"##,
        r##"replacement="##,
        r##"INVALID_MASK"##,
        r##"INVALID_REPLACE"##,
        r##"UNDEF_MASK"##,
        r##"UNDEF_REPLACE"##,
        r##"UNDEF_HEX_CHARREF"##,
        r##"PARTIAL_INPUT"##,
        r##"AFTER_OUTPUT"##,
        r##"UNIVERSAL_NEWLINE_DECORATOR"##,
        r##"CRLF_NEWLINE_DECORATOR"##,
        r##"CR_NEWLINE_DECORATOR"##,
        r##"XML_TEXT_DECORATOR"##,
        r##"XML_ATTR_CONTENT_DECORATOR"##,
        r##"XML_ATTR_QUOTE_DECORATOR"##,
        r##"source_encoding_name"##,
        r##"destination_encoding_name"##,
        r##"error_char"##,
        r##"error_bytes"##,
        r##"readagain_bytes"##,
        r##"incomplete_input?"##,
        r##"Marshal"##,
        r##"marshal_dump"##,
        r##"marshal_load"##,
        r##"_dump_data"##,
        r##"_load_data"##,
        r##"_alloc"##,
        r##"read"##,
        r##"binmode"##,
        r##"load"##,
        r##"restore"##,
        r##"MAJOR_VERSION"##,
        r##"MINOR_VERSION"##,
        r##"excl"##,
        r##"Range"##,
        r##"exclude_end?"##,
        r##"cover?"##,
        r##"IOError"##,
        r##"EOFError"##,
        r##"getc"##,
        r##"flush"##,
        r##"readpartial"##,
        r##"set_encoding"##,
        r##"syscall"##,
        r##"open"##,
        r##"printf"##,
        r##"print"##,
        r##"putc"##,
        r##"puts"##,
        r##"readline"##,
        r##"readlines"##,
        r##"p"##,
        r##"display"##,
        r##"IO"##,
        r##"WaitReadable"##,
        r##"WaitWritable"##,
        r##"EAGAINWaitReadable"##,
        r##"EAGAINWaitWritable"##,
        r##"EWOULDBLOCKWaitReadable"##,
        r##"EWOULDBLOCKWaitWritable"##,
        r##"EINPROGRESSWaitReadable"##,
        r##"EINPROGRESSWaitWritable"##,
        r##"sysopen"##,
        r##"for_fd"##,
        r##"popen"##,
        r##"foreach"##,
        r##"binread"##,
        r##"binwrite"##,
        r##"pipe"##,
        r##"copy_stream"##,
        r##"$,"##,
        r##"$/"##,
        r##"$-0"##,
        r##"$\"##,
        r##"reopen"##,
        r##"syswrite"##,
        r##"sysread"##,
        r##"pread"##,
        r##"pwrite"##,
        r##"fileno"##,
        r##"fsync"##,
        r##"fdatasync"##,
        r##"sync"##,
        r##"sync="##,
        r##"lineno"##,
        r##"lineno="##,
        r##"__read_nonblock"##,
        r##"__write_nonblock"##,
        r##"readchar"##,
        r##"readbyte"##,
        r##"ungetbyte"##,
        r##"ungetc"##,
        r##"tell"##,
        r##"seek"##,
        r##"SEEK_SET"##,
        r##"SEEK_CUR"##,
        r##"SEEK_END"##,
        r##"SEEK_DATA"##,
        r##"SEEK_HOLE"##,
        r##"rewind"##,
        r##"pos"##,
        r##"pos="##,
        r##"eof"##,
        r##"eof?"##,
        r##"close_on_exec?"##,
        r##"close_on_exec="##,
        r##"close"##,
        r##"closed?"##,
        r##"close_read"##,
        r##"close_write"##,
        r##"isatty"##,
        r##"tty?"##,
        r##"binmode?"##,
        r##"sysseek"##,
        r##"advise"##,
        r##"ioctl"##,
        r##"fcntl"##,
        r##"pid"##,
        r##"external_encoding"##,
        r##"internal_encoding"##,
        r##"autoclose?"##,
        r##"autoclose="##,
        r##"$stdin"##,
        r##"$stdout"##,
        r##"$stderr"##,
        r##"$>"##,
        r##"STDIN"##,
        r##"STDOUT"##,
        r##"STDERR"##,
        r##"argv"##,
        r##"to_write_io"##,
        r##"read_nonblock"##,
        r##"filename"##,
        r##"file"##,
        r##"skip"##,
        r##"inplace_mode"##,
        r##"inplace_mode="##,
        r##"$<"##,
        r##"ARGF"##,
        r##"$."##,
        r##"$FILENAME"##,
        r##"$-i"##,
        r##"$*"##,
        r##"FileTest"##,
        r##"File"##,
        r##"directory?"##,
        r##"exist?"##,
        r##"exists?"##,
        r##"readable?"##,
        r##"readable_real?"##,
        r##"world_readable?"##,
        r##"writable?"##,
        r##"writable_real?"##,
        r##"world_writable?"##,
        r##"executable?"##,
        r##"executable_real?"##,
        r##"file?"##,
        r##"size?"##,
        r##"owned?"##,
        r##"grpowned?"##,
        r##"pipe?"##,
        r##"symlink?"##,
        r##"socket?"##,
        r##"blockdev?"##,
        r##"chardev?"##,
        r##"setuid?"##,
        r##"setgid?"##,
        r##"sticky?"##,
        r##"identical?"##,
        r##"stat"##,
        r##"lstat"##,
        r##"ftype"##,
        r##"atime"##,
        r##"mtime"##,
        r##"ctime"##,
        r##"birthtime"##,
        r##"utime"##,
        r##"chmod"##,
        r##"chown"##,
        r##"lchmod"##,
        r##"lchown"##,
        r##"lutime"##,
        r##"link"##,
        r##"symlink"##,
        r##"readlink"##,
        r##"unlink"##,
        r##"rename"##,
        r##"umask"##,
        r##"mkfifo"##,
        r##"expand_path"##,
        r##"absolute_path"##,
        r##"realpath"##,
        r##"realdirpath"##,
        r##"basename"##,
        r##"dirname"##,
        r##"extname"##,
        r##"Separator"##,
        r##"SEPARATOR"##,
        r##"ALT_SEPARATOR"##,
        r##"PATH_SEPARATOR"##,
        r##"flock"##,
        r##"Constants"##,
        r##"RDONLY"##,
        r##"WRONLY"##,
        r##"RDWR"##,
        r##"APPEND"##,
        r##"CREAT"##,
        r##"EXCL"##,
        r##"NONBLOCK"##,
        r##"TRUNC"##,
        r##"NOCTTY"##,
        r##"BINARY"##,
        r##"SHARE_DELETE"##,
        r##"SYNC"##,
        r##"DSYNC"##,
        r##"NOFOLLOW"##,
        r##"LOCK_SH"##,
        r##"LOCK_EX"##,
        r##"LOCK_UN"##,
        r##"LOCK_NB"##,
        r##"NULL"##,
        r##"to_path"##,
        r##"test"##,
        r##"Stat"##,
        r##"dev"##,
        r##"dev_major"##,
        r##"dev_minor"##,
        r##"ino"##,
        r##"mode"##,
        r##"nlink"##,
        r##"uid"##,
        r##"gid"##,
        r##"rdev"##,
        r##"rdev_major"##,
        r##"rdev_minor"##,
        r##"blksize"##,
        r##"blocks"##,
        r##"perm"##,
        r##"flags"##,
        r##"open_args"##,
        r##"textmode"##,
        r##"autoclose"##,
        r##"normal"##,
        r##"sequential"##,
        r##"willneed"##,
        r##"dontneed"##,
        r##"noreuse"##,
        r##"SET"##,
        r##"CUR"##,
        r##"END"##,
        r##"DATA"##,
        r##"HOLE"##,
        r##"wait_readable"##,
        r##"wait_writable"##,
        r##"Dir"##,
        r##"each_child"##,
        r##"children"##,
        r##"chdir"##,
        r##"getwd"##,
        r##"pwd"##,
        r##"chroot"##,
        r##"mkdir"##,
        r##"rmdir"##,
        r##"home"##,
        r##"glob"##,
        r##"fnmatch"##,
        r##"fnmatch?"##,
        r##"FNM_NOESCAPE"##,
        r##"FNM_PATHNAME"##,
        r##"FNM_DOTMATCH"##,
        r##"FNM_CASEFOLD"##,
        r##"FNM_EXTGLOB"##,
        r##"FNM_SYSCASE"##,
        r##"FNM_SHORTNAME"##,
        r##"submicro"##,
        r##"nano_num"##,
        r##"nano_den"##,
        r##"zone"##,
        r##"nanosecond"##,
        r##"microsecond"##,
        r##"millisecond"##,
        r##"nsec"##,
        r##"usec"##,
        r##"local_to_utc"##,
        r##"utc_to_local"##,
        r##"year"##,
        r##"mon"##,
        r##"mday"##,
        r##"hour"##,
        r##"sec"##,
        r##"isdst"##,
        r##"find_timezone"##,
        r##"Time"##,
        r##"now"##,
        r##"utc"##,
        r##"gm"##,
        r##"local"##,
        r##"mktime"##,
        r##"localtime"##,
        r##"gmtime"##,
        r##"getlocal"##,
        r##"getgm"##,
        r##"getutc"##,
        r##"asctime"##,
        r##"day"##,
        r##"month"##,
        r##"wday"##,
        r##"yday"##,
        r##"dst?"##,
        r##"gmtoff"##,
        r##"gmt_offset"##,
        r##"utc_offset"##,
        r##"utc?"##,
        r##"gmt?"##,
        r##"sunday?"##,
        r##"monday?"##,
        r##"tuesday?"##,
        r##"wednesday?"##,
        r##"thursday?"##,
        r##"friday?"##,
        r##"saturday?"##,
        r##"tv_sec"##,
        r##"tv_usec"##,
        r##"tv_nsec"##,
        r##"subsec"##,
        r##"strftime"##,
        r##"tm"##,
        r##"to_time"##,
        r##"from_time"##,
        r##"rand"##,
        r##"srand"##,
        r##"Random"##,
        r##"seed"##,
        r##"state"##,
        r##"left"##,
        r##"DEFAULT"##,
        r##"new_seed"##,
        r##"urandom"##,
        r##"Formatter"##,
        r##"random_number"##,
        r##"Signal"##,
        r##"trap"##,
        r##"signame"##,
        r##"signm"##,
        r##"$:"##,
        r##"$-I"##,
        r##"$LOAD_PATH"##,
        r##"$""##,
        r##"$LOADED_FEATURES"##,
        r##"require"##,
        r##"require_relative"##,
        r##"autoload"##,
        r##"autoload?"##,
        r##"Proc"##,
        r##"arity"##,
        r##"lambda?"##,
        r##"binding"##,
        r##"curry"##,
        r##"source_location"##,
        r##"parameters"##,
        r##"LocalJumpError"##,
        r##"exit_value"##,
        r##"reason"##,
        r##"SystemStackError"##,
        r##"Method"##,
        r##"original_name"##,
        r##"owner"##,
        r##"unbind"##,
        r##"super_method"##,
        r##"method"##,
        r##"public_method"##,
        r##"singleton_method"##,
        r##"UnboundMethod"##,
        r##"bind"##,
        r##"instance_method"##,
        r##"public_instance_method"##,
        r##"define_method"##,
        r##"define_singleton_method"##,
        r##"Binding"##,
        r##"local_variable_get"##,
        r##"local_variable_set"##,
        r##"local_variable_defined?"##,
        r##"Math"##,
        r##"DomainError"##,
        r##"PI"##,
        r##"E"##,
        r##"atan2"##,
        r##"cos"##,
        r##"sin"##,
        r##"tan"##,
        r##"acos"##,
        r##"asin"##,
        r##"atan"##,
        r##"cosh"##,
        r##"sinh"##,
        r##"tanh"##,
        r##"acosh"##,
        r##"asinh"##,
        r##"atanh"##,
        r##"exp"##,
        r##"log"##,
        r##"log2"##,
        r##"log10"##,
        r##"cbrt"##,
        r##"frexp"##,
        r##"ldexp"##,
        r##"hypot"##,
        r##"erf"##,
        r##"erfc"##,
        r##"gamma"##,
        r##"lgamma"##,
        r##"GC"##,
        r##"start"##,
        r##"enable"##,
        r##"disable"##,
        r##"stress"##,
        r##"stress="##,
        r##"latest_gc_info"##,
        r##"garbage_collect"##,
        r##"RVALUE_SIZE"##,
        r##"HEAP_PAGE_OBJ_LIMIT"##,
        r##"HEAP_PAGE_BITMAP_SIZE"##,
        r##"HEAP_PAGE_BITMAP_PLANES"##,
        r##"INTERNAL_CONSTANTS"##,
        r##"Profiler"##,
        r##"enabled?"##,
        r##"raw_data"##,
        r##"report"##,
        r##"total_time"##,
        r##"ObjectSpace"##,
        r##"each_object"##,
        r##"define_finalizer"##,
        r##"undefine_finalizer"##,
        r##"_id2ref"##,
        r##"__id__"##,
        r##"count_objects"##,
        r##"WeakMap"##,
        r##"finalize"##,
        r##"verify_internal_consistency"##,
        r##"verify_transient_heap_internal_consistency"##,
        r##"OPTS"##,
        r##"arguments"##,
        r##"memo"##,
        r##"force"##,
        r##"to_enum"##,
        r##"exclude_end"##,
        r##"enum_for"##,
        r##"Enumerator"##,
        r##"with_index"##,
        r##"with_object"##,
        r##"next_values"##,
        r##"peek_values"##,
        r##"peek"##,
        r##"feed"##,
        r##"chain"##,
        r##"Lazy"##,
        r##"lazy"##,
        r##"StopIteration"##,
        r##"Generator"##,
        r##"Yielder"##,
        r##"Chain"##,
        r##"ArithmeticSequence"##,
        r##"RubyVM"##,
        r##"MJIT"##,
        r##"pause"##,
        r##"resume"##,
        r##"Thread"##,
        r##"INSTRUCTION_NAMES"##,
        r##"thread_vm_stack_size"##,
        r##"thread_machine_stack_size"##,
        r##"fiber_vm_stack_size"##,
        r##"fiber_machine_stack_size"##,
        r##"DEFAULT_PARAMS"##,
        r##"translate"##,
        r##"locals"##,
        r##"TOPLEVEL_BINDING"##,
        r##"Backtrace"##,
        r##"Location"##,
        r##"label"##,
        r##"base_label"##,
        r##"caller"##,
        r##"caller_locations"##,
        r##"resolve_feature_path"##,
        r##"InstructionSequence"##,
        r##"disasm"##,
        r##"disassemble"##,
        r##"to_binary"##,
        r##"load_from_binary"##,
        r##"load_from_binary_extra_data"##,
        r##"first_lineno"##,
        r##"trace_points"##,
        r##"compile_file"##,
        r##"compile_option"##,
        r##"compile_option="##,
        r##"of"##,
        r##"load_iseq"##,
        r##"never"##,
        r##"immediate"##,
        r##"on_blocking"##,
        r##"fork"##,
        r##"main"##,
        r##"current"##,
        r##"stop"##,
        r##"kill"##,
        r##"exit"##,
        r##"pass"##,
        r##"abort_on_exception"##,
        r##"abort_on_exception="##,
        r##"report_on_exception"##,
        r##"report_on_exception="##,
        r##"handle_interrupt"##,
        r##"pending_interrupt?"##,
        r##"terminate"##,
        r##"run"##,
        r##"wakeup"##,
        r##"priority"##,
        r##"priority="##,
        r##"thread_variable_get"##,
        r##"thread_variable_set"##,
        r##"thread_variables"##,
        r##"thread_variable?"##,
        r##"alive?"##,
        r##"stop?"##,
        r##"safe_level"##,
        r##"group"##,
        r##"name="##,
        r##"ThreadGroup"##,
        r##"enclose"##,
        r##"enclosed?"##,
        r##"add"##,
        r##"Default"##,
        r##"__recursive_key__"##,
        r##"ThreadError"##,
        r##"Mutex"##,
        r##"locked?"##,
        r##"try_lock"##,
        r##"lock"##,
        r##"unlock"##,
        r##"sleep"##,
        r##"synchronize"##,
        r##"Queue"##,
        r##"ClosedQueueError"##,
        r##"num_waiting"##,
        r##"enq"##,
        r##"deq"##,
        r##"SizedQueue"##,
        r##"max="##,
        r##"ConditionVariable"##,
        r##"wait"##,
        r##"signal"##,
        r##"broadcast"##,
        r##"in"##,
        r##"out"##,
        r##"err"##,
        r##"child"##,
        r##"pgroup"##,
        r##"unsetenv_others"##,
        r##"close_others"##,
        r##"second"##,
        r##"float_microsecond"##,
        r##"float_millisecond"##,
        r##"float_second"##,
        r##"GETTIMEOFDAY_BASED_CLOCK_REALTIME"##,
        r##"TIME_BASED_CLOCK_REALTIME"##,
        r##"TIMES_BASED_CLOCK_MONOTONIC"##,
        r##"TIMES_BASED_CLOCK_PROCESS_CPUTIME_ID"##,
        r##"GETRUSAGE_BASED_CLOCK_PROCESS_CPUTIME_ID"##,
        r##"CLOCK_BASED_CLOCK_PROCESS_CPUTIME_ID"##,
        r##"MACH_ABSOLUTE_TIME_BASED_CLOCK_MONOTONIC"##,
        r##"hertz"##,
        r##"$?"##,
        r##"$$"##,
        r##"exec"##,
        r##"exit!"##,
        r##"system"##,
        r##"spawn"##,
        r##"abort"##,
        r##"Process"##,
        r##"WNOHANG"##,
        r##"WUNTRACED"##,
        r##"last_status"##,
        r##"wait2"##,
        r##"waitpid"##,
        r##"waitpid2"##,
        r##"waitall"##,
        r##"detach"##,
        r##"Waiter"##,
        r##"Status"##,
        r##"stopped?"##,
        r##"stopsig"##,
        r##"signaled?"##,
        r##"termsig"##,
        r##"exited?"##,
        r##"exitstatus"##,
        r##"coredump?"##,
        r##"ppid"##,
        r##"getpgrp"##,
        r##"setpgrp"##,
        r##"getpgid"##,
        r##"setpgid"##,
        r##"getsid"##,
        r##"setsid"##,
        r##"getpriority"##,
        r##"setpriority"##,
        r##"PRIO_PROCESS"##,
        r##"PRIO_PGRP"##,
        r##"PRIO_USER"##,
        r##"getrlimit"##,
        r##"setrlimit"##,
        r##"RLIM_SAVED_MAX"##,
        r##"RLIM_INFINITY"##,
        r##"RLIM_SAVED_CUR"##,
        r##"RLIMIT_AS"##,
        r##"RLIMIT_CORE"##,
        r##"RLIMIT_CPU"##,
        r##"RLIMIT_DATA"##,
        r##"RLIMIT_FSIZE"##,
        r##"RLIMIT_MEMLOCK"##,
        r##"RLIMIT_NOFILE"##,
        r##"RLIMIT_NPROC"##,
        r##"RLIMIT_RSS"##,
        r##"RLIMIT_STACK"##,
        r##"uid="##,
        r##"gid="##,
        r##"euid"##,
        r##"euid="##,
        r##"egid"##,
        r##"egid="##,
        r##"initgroups"##,
        r##"groups"##,
        r##"groups="##,
        r##"maxgroups"##,
        r##"maxgroups="##,
        r##"daemon"##,
        r##"CLOCK_REALTIME"##,
        r##"CLOCK_MONOTONIC"##,
        r##"CLOCK_PROCESS_CPUTIME_ID"##,
        r##"CLOCK_THREAD_CPUTIME_ID"##,
        r##"CLOCK_MONOTONIC_RAW"##,
        r##"CLOCK_MONOTONIC_RAW_APPROX"##,
        r##"CLOCK_UPTIME_RAW"##,
        r##"CLOCK_UPTIME_RAW_APPROX"##,
        r##"clock_gettime"##,
        r##"clock_getres"##,
        r##"stime"##,
        r##"cutime"##,
        r##"cstime"##,
        r##"Tms"##,
        r##"utime="##,
        r##"stime="##,
        r##"cutime="##,
        r##"cstime="##,
        r##"UID"##,
        r##"GID"##,
        r##"rid"##,
        r##"eid"##,
        r##"change_privilege"##,
        r##"grant_privilege"##,
        r##"eid="##,
        r##"re_exchange"##,
        r##"re_exchangeable?"##,
        r##"sid_available?"##,
        r##"switch"##,
        r##"from_name"##,
        r##"Sys"##,
        r##"getuid"##,
        r##"geteuid"##,
        r##"getgid"##,
        r##"getegid"##,
        r##"setuid"##,
        r##"setgid"##,
        r##"setruid"##,
        r##"setrgid"##,
        r##"seteuid"##,
        r##"setegid"##,
        r##"setreuid"##,
        r##"setregid"##,
        r##"setresuid"##,
        r##"setresgid"##,
        r##"issetugid"##,
        r##"Fiber"##,
        r##"FiberError"##,
        r##"@numerator"##,
        r##"@denominator"##,
        r##"Rational"##,
        r##"numerator"##,
        r##"denominator"##,
        r##"rationalize"##,
        r##"compatible"##,
        r##"gcd"##,
        r##"lcm"##,
        r##"gcdlcm"##,
        r##"arg"##,
        r##"@real"##,
        r##"@image"##,
        r##"Complex"##,
        r##"rectangular"##,
        r##"rect"##,
        r##"polar"##,
        r##"real"##,
        r##"imaginary"##,
        r##"imag"##,
        r##"abs2"##,
        r##"angle"##,
        r##"phase"##,
        r##"conjugate"##,
        r##"conj"##,
        r##"to_c"##,
        r##"I"##,
        r##"RUBY_VERSION"##,
        r##"RUBY_RELEASE_DATE"##,
        r##"RUBY_PLATFORM"##,
        r##"RUBY_PATCHLEVEL"##,
        r##"RUBY_REVISION"##,
        r##"RUBY_COPYRIGHT"##,
        r##"RUBY_ENGINE"##,
        r##"RUBY_ENGINE_VERSION"##,
        r##"set_trace_func"##,
        r##"add_trace_func"##,
        r##"TracePoint"##,
        r##"trace"##,
        r##"__enable"##,
        r##"event"##,
        r##"method_id"##,
        r##"callee_id"##,
        r##"defined_class"##,
        r##"self"##,
        r##"return_value"##,
        r##"raised_exception"##,
        r##"eval_script"##,
        r##"instruction_sequence"##,
        r##"AbstractSyntaxTree"##,
        r##"Node"##,
        r##"parse"##,
        r##"parse_file"##,
        r##"type"##,
        r##"first_column"##,
        r##"last_lineno"##,
        r##"last_column"##,
        r##"$VERBOSE"##,
        r##"$-v"##,
        r##"$-w"##,
        r##"$-W"##,
        r##"$DEBUG"##,
        r##"$-d"##,
        r##"$0"##,
        r##"$PROGRAM_NAME"##,
        r##"argv0"##,
        r##"setproctitle"##,
        r##"ARGV"##,
        r##"@gem_prelude_index"##,
        r##"TMP_RUBY_PREFIX"##,
        r##"RUBY_DESCRIPTION"##,
        r##"resolving"##,
        r##"ASCII_8BIT"##,
        r##"UTF_8"##,
        r##"US_ASCII"##,
        r##"Big5"##,
        r##"BIG5"##,
        r##"Big5_HKSCS"##,
        r##"BIG5_HKSCS"##,
        r##"Big5_UAO"##,
        r##"BIG5_UAO"##,
        r##"CP949"##,
        r##"Emacs_Mule"##,
        r##"EMACS_MULE"##,
        r##"EUC_JP"##,
        r##"EUC_KR"##,
        r##"EUC_TW"##,
        r##"GB2312"##,
        r##"GB18030"##,
        r##"GBK"##,
        r##"ISO_8859_1"##,
        r##"ISO_8859_2"##,
        r##"ISO_8859_3"##,
        r##"ISO_8859_4"##,
        r##"ISO_8859_5"##,
        r##"ISO_8859_6"##,
        r##"ISO_8859_7"##,
        r##"ISO_8859_8"##,
        r##"ISO_8859_9"##,
        r##"ISO_8859_10"##,
        r##"ISO_8859_11"##,
        r##"ISO_8859_13"##,
        r##"ISO_8859_14"##,
        r##"ISO_8859_15"##,
        r##"ISO_8859_16"##,
        r##"KOI8_R"##,
        r##"KOI8_U"##,
        r##"Shift_JIS"##,
        r##"SHIFT_JIS"##,
        r##"UTF_16BE"##,
        r##"UTF_16LE"##,
        r##"UTF_32BE"##,
        r##"UTF_32LE"##,
        r##"Windows_31J"##,
        r##"WINDOWS_31J"##,
        r##"Windows_1250"##,
        r##"WINDOWS_1250"##,
        r##"Windows_1251"##,
        r##"WINDOWS_1251"##,
        r##"Windows_1252"##,
        r##"WINDOWS_1252"##,
        r##"Windows_1253"##,
        r##"WINDOWS_1253"##,
        r##"Windows_1254"##,
        r##"WINDOWS_1254"##,
        r##"Windows_1257"##,
        r##"WINDOWS_1257"##,
        r##"IBM437"##,
        r##"CP437"##,
        r##"IBM737"##,
        r##"CP737"##,
        r##"IBM775"##,
        r##"CP775"##,
        r##"CP850"##,
        r##"IBM850"##,
        r##"IBM852"##,
        r##"CP852"##,
        r##"IBM855"##,
        r##"CP855"##,
        r##"IBM857"##,
        r##"CP857"##,
        r##"IBM860"##,
        r##"CP860"##,
        r##"IBM861"##,
        r##"CP861"##,
        r##"IBM862"##,
        r##"CP862"##,
        r##"IBM863"##,
        r##"CP863"##,
        r##"IBM864"##,
        r##"CP864"##,
        r##"IBM865"##,
        r##"CP865"##,
        r##"IBM866"##,
        r##"CP866"##,
        r##"IBM869"##,
        r##"CP869"##,
        r##"Windows_1258"##,
        r##"WINDOWS_1258"##,
        r##"CP1258"##,
        r##"GB1988"##,
        r##"MacCentEuro"##,
        r##"MACCENTEURO"##,
        r##"MacCroatian"##,
        r##"MACCROATIAN"##,
        r##"MacCyrillic"##,
        r##"MACCYRILLIC"##,
        r##"MacGreek"##,
        r##"MACGREEK"##,
        r##"MacIceland"##,
        r##"MACICELAND"##,
        r##"MacRoman"##,
        r##"MACROMAN"##,
        r##"MacRomania"##,
        r##"MACROMANIA"##,
        r##"MacThai"##,
        r##"MACTHAI"##,
        r##"MacTurkish"##,
        r##"MACTURKISH"##,
        r##"MacUkraine"##,
        r##"MACUKRAINE"##,
        r##"CP950"##,
        r##"Big5_HKSCS_2008"##,
        r##"BIG5_HKSCS_2008"##,
        r##"CP951"##,
        r##"IBM037"##,
        r##"EBCDIC_CP_US"##,
        r##"Stateless_ISO_2022_JP"##,
        r##"STATELESS_ISO_2022_JP"##,
        r##"EucJP"##,
        r##"EUCJP"##,
        r##"EucJP_ms"##,
        r##"EUCJP_MS"##,
        r##"EUC_JP_MS"##,
        r##"CP51932"##,
        r##"EUC_JIS_2004"##,
        r##"EUC_JISX0213"##,
        r##"EucKR"##,
        r##"EUCKR"##,
        r##"EucTW"##,
        r##"EUCTW"##,
        r##"EUC_CN"##,
        r##"EucCN"##,
        r##"EUCCN"##,
        r##"GB12345"##,
        r##"CP936"##,
        r##"ISO_2022_JP"##,
        r##"ISO2022_JP"##,
        r##"ISO_2022_JP_2"##,
        r##"ISO2022_JP2"##,
        r##"CP50220"##,
        r##"CP50221"##,
        r##"ISO8859_1"##,
        r##"ISO8859_2"##,
        r##"ISO8859_3"##,
        r##"ISO8859_4"##,
        r##"ISO8859_5"##,
        r##"ISO8859_6"##,
        r##"Windows_1256"##,
        r##"WINDOWS_1256"##,
        r##"CP1256"##,
        r##"ISO8859_7"##,
        r##"ISO8859_8"##,
        r##"Windows_1255"##,
        r##"WINDOWS_1255"##,
        r##"CP1255"##,
        r##"ISO8859_9"##,
        r##"ISO8859_10"##,
        r##"ISO8859_11"##,
        r##"TIS_620"##,
        r##"Windows_874"##,
        r##"WINDOWS_874"##,
        r##"CP874"##,
        r##"ISO8859_13"##,
        r##"ISO8859_14"##,
        r##"ISO8859_15"##,
        r##"ISO8859_16"##,
        r##"CP878"##,
        r##"MacJapanese"##,
        r##"MACJAPANESE"##,
        r##"MacJapan"##,
        r##"MACJAPAN"##,
        r##"ASCII"##,
        r##"ANSI_X3_4_1968"##,
        r##"UTF_7"##,
        r##"CP65000"##,
        r##"CP65001"##,
        r##"UTF8_MAC"##,
        r##"UTF_8_MAC"##,
        r##"UTF_8_HFS"##,
        r##"UTF_16"##,
        r##"UTF_32"##,
        r##"UCS_2BE"##,
        r##"UCS_4BE"##,
        r##"UCS_4LE"##,
        r##"CP932"##,
        r##"CsWindows31J"##,
        r##"CSWINDOWS31J"##,
        r##"SJIS"##,
        r##"PCK"##,
        r##"CP1250"##,
        r##"CP1251"##,
        r##"CP1252"##,
        r##"CP1253"##,
        r##"CP1254"##,
        r##"CP1257"##,
        r##"UTF8_DoCoMo"##,
        r##"UTF8_DOCOMO"##,
        r##"SJIS_DoCoMo"##,
        r##"SJIS_DOCOMO"##,
        r##"UTF8_KDDI"##,
        r##"SJIS_KDDI"##,
        r##"ISO_2022_JP_KDDI"##,
        r##"Stateless_ISO_2022_JP_KDDI"##,
        r##"STATELESS_ISO_2022_JP_KDDI"##,
        r##"UTF8_SoftBank"##,
        r##"UTF8_SOFTBANK"##,
        r##"SJIS_SoftBank"##,
        r##"SJIS_SOFTBANK"##,
        r##"SCRIPT_LINES__"##,
        r##"exclusive"##,
        r##"block"##,
        r##"mutex"##,
        r##"len"##,
        r##"buf"##,
        r##"write_nonblock"##,
        r##"target"##,
        r##"target_line"##,
        r##"blk"##,
        r##"irb"##,
        r##"pp"##,
        r##"objs"##,
        r##"coverage_enabled"##,
        r##"inline_const_cache"##,
        r##"peephole_optimization"##,
        r##"tailcall_optimization"##,
        r##"specialized_instruction"##,
        r##"operands_unification"##,
        r##"instructions_unification"##,
        r##"stack_caching"##,
        r##"frozen_string_literal"##,
        r##"debug_frozen_string_literal"##,
        r##"debug_level"##,
        r##"singletonclass"##,
        r##"Gem"##,
        r##"gem"##,
        r##"DidYouMean"##,
        r##"s"##,
        r##"$-p"##,
        r##"$-l"##,
        r##"$-a"##,
    ];

    #[test]
    fn mri_symbol_idents() {
        for &sym in IDENTS {
            assert!(
                sym.parse::<IdentifierType>().is_ok(),
                "Failed to validate {} as a valid identifier",
                sym
            );
        }
    }
}
