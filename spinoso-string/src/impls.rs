use alloc::borrow::Cow;
use alloc::vec::Vec;
use core::borrow::{Borrow, BorrowMut};
use core::iter::{FromIterator, FusedIterator};
use core::ops::{Deref, DerefMut, Index, IndexMut};
use core::slice::SliceIndex;

use crate::{Bytes, Center, IntoIter, Iter, IterMut, String};

impl<'a> AsRef<[u8]> for Iter<'a> {
    fn as_ref(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a u8;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }

    #[inline]
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.0.nth(n)
    }

    #[inline]
    fn count(self) -> usize {
        self.0.count()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }

    #[inline]
    fn last(self) -> Option<Self::Item> {
        self.0.last()
    }
}

impl<'a> DoubleEndedIterator for Iter<'a> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }

    #[inline]
    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        self.0.nth_back(n)
    }
}

impl<'a> FusedIterator for Iter<'a> {}

impl<'a> ExactSizeIterator for Iter<'a> {}

impl<'a> Iterator for IterMut<'a> {
    type Item = &'a mut u8;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }

    #[inline]
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.0.nth(n)
    }

    #[inline]
    fn count(self) -> usize {
        self.0.count()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }

    #[inline]
    fn last(self) -> Option<Self::Item> {
        self.0.last()
    }
}

impl<'a> DoubleEndedIterator for IterMut<'a> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }

    #[inline]
    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        self.0.nth_back(n)
    }
}

impl<'a> FusedIterator for IterMut<'a> {}

impl<'a> ExactSizeIterator for IterMut<'a> {}

impl AsRef<[u8]> for IntoIter {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl Iterator for IntoIter {
    type Item = u8;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }

    #[inline]
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.0.nth(n)
    }

    #[inline]
    fn count(self) -> usize {
        self.0.count()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }

    #[inline]
    fn last(self) -> Option<Self::Item> {
        self.0.last()
    }
}

impl DoubleEndedIterator for IntoIter {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }

    #[inline]
    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        self.0.nth_back(n)
    }
}

impl FusedIterator for IntoIter {}

impl ExactSizeIterator for IntoIter {}

impl<'a> Iterator for Bytes<'a> {
    type Item = u8;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().copied()
    }

    #[inline]
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.0.nth(n).copied()
    }

    #[inline]
    fn count(self) -> usize {
        self.0.count()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }

    #[inline]
    fn last(self) -> Option<Self::Item> {
        self.0.last().copied()
    }
}

impl<'a> DoubleEndedIterator for Bytes<'a> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back().copied()
    }

    #[inline]
    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        self.0.nth_back(n).copied()
    }
}

impl<'a> FusedIterator for Bytes<'a> {}

impl<'a> ExactSizeIterator for Bytes<'a> {}

impl<'a, 'b> Iterator for Center<'a, 'b> {
    type Item = u8;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(&next) = self.left.next() {
            return Some(next);
        }
        if let Some(next) = self.next.take() {
            if let Some((&first, tail)) = next.split_first() {
                self.next = Some(tail);
                return Some(first);
            }
        }
        if let Some(next) = self.s.next() {
            if let Some((&first, tail)) = next.split_first() {
                if !tail.is_empty() {
                    self.next = Some(tail);
                }
                return Some(first);
            }
        }
        self.right.next().copied()
    }
}

impl<'a, 'b> FusedIterator for Center<'a, 'b> {}

impl<'a, 'b> ExactSizeIterator for Center<'a, 'b> {}

impl IntoIterator for String {
    type Item = u8;
    type IntoIter = IntoIter;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self.buf.into_iter())
    }
}

impl Extend<u8> for String {
    #[inline]
    fn extend<I: IntoIterator<Item = u8>>(&mut self, iter: I) {
        self.buf.extend(iter.into_iter())
    }
}

impl<'a> Extend<&'a u8> for String {
    #[inline]
    fn extend<I: IntoIterator<Item = &'a u8>>(&mut self, iter: I) {
        self.buf.extend(iter.into_iter().copied())
    }
}

impl<'a> Extend<&'a mut u8> for String {
    #[inline]
    fn extend<I: IntoIterator<Item = &'a mut u8>>(&mut self, iter: I) {
        self.buf.extend(iter.into_iter().map(|byte| *byte))
    }
}

impl FromIterator<u8> for String {
    #[inline]
    fn from_iter<I: IntoIterator<Item = u8>>(iter: I) -> Self {
        let mut s = String::new();
        s.extend(iter.into_iter());
        s
    }
}

impl<'a> FromIterator<&'a u8> for String {
    #[inline]
    fn from_iter<I: IntoIterator<Item = &'a u8>>(iter: I) -> Self {
        let mut s = String::new();
        s.extend(iter.into_iter());
        s
    }
}

impl<'a> FromIterator<&'a mut u8> for String {
    #[inline]
    fn from_iter<I: IntoIterator<Item = &'a mut u8>>(iter: I) -> Self {
        let mut s = String::new();
        s.extend(iter.into_iter());
        s
    }
}

impl FromIterator<char> for String {
    #[inline]
    fn from_iter<I: IntoIterator<Item = char>>(iter: I) -> Self {
        let mut s = String::new();
        let mut buf = [0; 4];
        for ch in iter {
            let encoded = ch.encode_utf8(&mut buf);
            s.buf.extend_from_slice(encoded.as_bytes());
        }
        s
    }
}

impl<'a> FromIterator<&'a char> for String {
    #[inline]
    fn from_iter<I: IntoIterator<Item = &'a char>>(iter: I) -> Self {
        let mut s = String::new();
        let mut buf = [0; 4];
        for ch in iter {
            let encoded = ch.encode_utf8(&mut buf);
            s.buf.extend_from_slice(encoded.as_bytes());
        }
        s
    }
}

impl<'a> FromIterator<&'a mut char> for String {
    #[inline]
    fn from_iter<I: IntoIterator<Item = &'a mut char>>(iter: I) -> Self {
        let mut s = String::new();
        let mut buf = [0; 4];
        for ch in iter {
            let encoded = ch.encode_utf8(&mut buf);
            s.buf.extend_from_slice(encoded.as_bytes());
        }
        s
    }
}

impl From<Vec<u8>> for String {
    #[inline]
    fn from(content: Vec<u8>) -> Self {
        Self::utf8(content)
    }
}

impl<'a> From<&'a [u8]> for String {
    #[inline]
    fn from(content: &'a [u8]) -> Self {
        Self::utf8(content.to_vec())
    }
}

impl<'a> From<&'a mut [u8]> for String {
    #[inline]
    fn from(content: &'a mut [u8]) -> Self {
        Self::utf8(content.to_vec())
    }
}

impl<'a> From<Cow<'a, [u8]>> for String {
    #[inline]
    fn from(content: Cow<'a, [u8]>) -> Self {
        Self::utf8(content.into_owned())
    }
}

impl From<alloc::string::String> for String {
    #[inline]
    fn from(s: alloc::string::String) -> Self {
        Self::utf8(s.into_bytes())
    }
}

impl From<&str> for String {
    #[inline]
    fn from(s: &str) -> Self {
        Self::utf8(s.as_bytes().to_vec())
    }
}

impl From<String> for Vec<u8> {
    #[inline]
    fn from(s: String) -> Self {
        s.buf
    }
}

impl AsRef<[u8]> for String {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.buf.as_slice()
    }
}

impl AsMut<[u8]> for String {
    #[inline]
    fn as_mut(&mut self) -> &mut [u8] {
        self.buf.as_mut_slice()
    }
}

impl AsRef<Vec<u8>> for String {
    #[inline]
    fn as_ref(&self) -> &Vec<u8> {
        &self.buf
    }
}

impl AsMut<Vec<u8>> for String {
    #[inline]
    fn as_mut(&mut self) -> &mut Vec<u8> {
        &mut self.buf
    }
}

impl Deref for String {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &[u8] {
        &*self.buf
    }
}

impl DerefMut for String {
    #[inline]
    fn deref_mut(&mut self) -> &mut [u8] {
        &mut *self.buf
    }
}

impl Borrow<[u8]> for String {
    #[inline]
    fn borrow(&self) -> &[u8] {
        self.buf.as_slice()
    }
}

impl BorrowMut<[u8]> for String {
    #[inline]
    fn borrow_mut(&mut self) -> &mut [u8] {
        self.buf.as_mut_slice()
    }
}

impl Borrow<Vec<u8>> for String {
    #[inline]
    fn borrow(&self) -> &Vec<u8> {
        &self.buf
    }
}

impl BorrowMut<Vec<u8>> for String {
    #[inline]
    fn borrow_mut(&mut self) -> &mut Vec<u8> {
        &mut self.buf
    }
}

impl<I: SliceIndex<[u8]>> Index<I> for String {
    type Output = I::Output;

    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        Index::index(&self.buf, index)
    }
}

impl<I: SliceIndex<[u8]>> IndexMut<I> for String {
    #[inline]
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        IndexMut::index_mut(&mut self.buf, index)
    }
}
