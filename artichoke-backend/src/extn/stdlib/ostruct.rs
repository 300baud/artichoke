use artichoke_core::load::LoadSources;

use crate::class;
use crate::{Artichoke, ArtichokeError};

pub fn init(interp: &Artichoke) -> Result<(), ArtichokeError> {
    let spec = class::Spec::new("OpenStruct", None, None);
    interp.0.borrow_mut().def_class::<OpenStruct>(spec);
    interp.def_rb_source_file(b"ostruct.rb", &include_bytes!("ostruct.rb")[..])?;
    Ok(())
}

pub struct OpenStruct;