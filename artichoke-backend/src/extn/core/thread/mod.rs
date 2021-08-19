use std::ffi::CStr;

use crate::extn::prelude::*;

const THREAD_CSTR: &CStr = cstr::cstr!("Thread");
const MUTEX_CSTR: &CStr = cstr::cstr!("Mutex");

pub fn init(interp: &mut Artichoke) -> InitializeResult<()> {
    if interp.is_class_defined::<Thread>() {
        return Ok(());
    }
    if interp.is_class_defined::<Mutex>() {
        return Ok(());
    }
    let spec = class::Spec::new("Thread", THREAD_CSTR, None, None)?;
    interp.def_class::<Thread>(spec)?;
    let spec = class::Spec::new("Mutex", MUTEX_CSTR, None, None)?;
    interp.def_class::<Mutex>(spec)?;
    // TODO: Don't add a source file and don't add an explicit require below.
    // Instead, have thread be a default loaded feature in `mezzaluna-feature-loader`.
    interp.def_rb_source_file("thread.rb", &include_bytes!("thread.rb")[..])?;
    // Thread is loaded by default, so eval it on interpreter initialization
    // https://www.rubydoc.info/gems/rubocop/RuboCop/Cop/Lint/UnneededRequireStatement
    let _ = interp.eval(&b"require 'thread'"[..])?;
    trace!("Patched Thread onto interpreter");
    trace!("Patched Mutex onto interpreter");
    Ok(())
}

#[derive(Debug, Clone, Copy)]
pub struct Thread;

#[derive(Debug, Clone, Copy)]
pub struct Mutex;

#[cfg(test)]
mod tests {
    use crate::test::prelude::*;
    use bstr::ByteSlice;

    const SUBJECT: &str = "Thread";
    const FUNCTIONAL_TEST: &[u8] = include_bytes!("thread_test.rb");

    #[test]
    fn functional() {
        let mut interp = interpreter().unwrap();
        let _ = interp.eval(FUNCTIONAL_TEST).unwrap();
        let result = interp.eval(b"spec");
        if let Err(exc) = result {
            let backtrace = exc.vm_backtrace(&mut interp);
            let backtrace = bstr::join("\n", backtrace.unwrap_or_default());
            panic!(
                "{} tests failed with message: {:?} and backtrace:\n{:?}",
                SUBJECT,
                exc.message().as_bstr(),
                backtrace.as_bstr()
            );
        }
    }
}
