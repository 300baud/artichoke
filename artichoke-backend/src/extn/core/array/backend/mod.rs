use std::any::Any;

use crate::extn::prelude::*;

pub mod aggregate;
pub mod buffer;
pub mod fixed;
pub mod integer_range;
pub mod repeated;

pub trait ArrayType: Any {
    fn box_clone(&self) -> Box<dyn ArrayType>;

    fn gc_mark(&self, interp: &Artichoke);

    fn real_children(&self) -> usize;

    fn len(&self) -> usize;

    fn is_empty(&self) -> bool;

    fn get(&self, interp: &Artichoke, index: usize) -> Result<Value, Exception>;

    fn slice(
        &self,
        interp: &Artichoke,
        start: usize,
        len: usize,
    ) -> Result<Box<dyn ArrayType>, Exception>;

    fn set(
        &mut self,
        interp: &Artichoke,
        index: usize,
        elem: Value,
        realloc: &mut Option<Vec<Box<dyn ArrayType>>>,
    ) -> Result<(), Exception>;

    fn set_with_drain(
        &mut self,
        interp: &Artichoke,
        start: usize,
        drain: usize,
        with: Value,
        realloc: &mut Option<Vec<Box<dyn ArrayType>>>,
    ) -> Result<usize, Exception>;

    fn set_slice(
        &mut self,
        interp: &Artichoke,
        start: usize,
        drain: usize,
        with: Box<dyn ArrayType>,
        realloc: &mut Option<Vec<Box<dyn ArrayType>>>,
    ) -> Result<usize, Exception>;

    fn concat(
        &mut self,
        interp: &Artichoke,
        other: Box<dyn ArrayType>,
        realloc: &mut Option<Vec<Box<dyn ArrayType>>>,
    ) -> Result<(), Exception>;

    fn pop(
        &mut self,
        interp: &Artichoke,
        realloc: &mut Option<Vec<Box<dyn ArrayType>>>,
    ) -> Result<Value, Exception>;

    fn reverse(&mut self, interp: &Artichoke) -> Result<(), Exception>;
}

#[allow(clippy::missing_safety_doc)]
mod internal {
    downcast!(dyn super::ArrayType);
}
