use artichoke_core::eval::Eval;
use artichoke_core::load::LoadSources;

use crate::class;
use crate::{Artichoke, ArtichokeError};

pub fn init(interp: &Artichoke) -> Result<(), ArtichokeError> {
    if interp.0.borrow().class_spec::<Thread>().is_some() {
        return Ok(());
    }
    if interp.0.borrow().class_spec::<Mutex>().is_some() {
        return Ok(());
    }
    let spec = class::Spec::new("Thread", None, None);
    interp.0.borrow_mut().def_class::<Thread>(spec);
    let spec = class::Spec::new("Mutex", None, None);
    interp.0.borrow_mut().def_class::<Mutex>(spec);
    interp.def_rb_source_file(b"thread.rb", &include_bytes!("thread.rb")[..])?;
    // Thread is loaded by default, so eval it on interpreter initialization
    // https://www.rubydoc.info/gems/rubocop/RuboCop/Cop/Lint/UnneededRequireStatement
    interp.eval(&b"require 'thread'"[..])?;
    trace!("Patched Thread onto interpreter");
    trace!("Patched Mutex onto interpreter");
    Ok(())
}

pub struct Thread;
pub struct Mutex;

#[cfg(test)]
mod tests {
    #![allow(clippy::shadow_unrelated)]

    use artichoke_core::eval::Eval;
    use artichoke_core::value::Value as _;

    use crate::ArtichokeError;

    #[test]
    fn thread_required_by_default() {
        let interp = crate::interpreter().expect("init");
        let result = interp
            .eval(b"Object.const_defined?(:Thread)")
            .expect("thread");
        assert!(result.try_into::<bool>().expect("convert"));
    }

    #[test]
    fn thread_current_is_main() {
        let interp = crate::interpreter().expect("init");
        let spec = b"Thread.current == Thread.main";
        let result = interp.eval(spec).expect("spec");
        assert!(result.try_into::<bool>().expect("convert"));
        let spec = b"Thread.new { Thread.current == Thread.main }.join.value == false";
        let result = interp.eval(spec).expect("spec");
        assert!(result.try_into::<bool>().expect("convert"));
    }

    #[test]
    fn thread_join_value() {
        let interp = crate::interpreter().expect("init");
        let spec = b"Thread.new { 2 + 3 }.join.value == 5";
        let result = interp.eval(spec).expect("spec");
        assert!(result.try_into::<bool>().expect("convert"));
        let spec = b"Thread.new { 2 + Thread.new { 3 }.join.value }.join.value == 5";
        let result = interp.eval(spec).expect("spec");
        assert!(result.try_into::<bool>().expect("convert"));
    }

    #[test]
    fn thread_main_is_running() {
        let interp = crate::interpreter().expect("init");
        let spec = b"Thread.current.status == 'run'";
        let result = interp.eval(spec).expect("spec");
        assert!(result.try_into::<bool>().expect("convert"));
        let spec = b"Thread.current.alive?";
        let result = interp.eval(spec).expect("spec");
        assert!(result.try_into::<bool>().expect("convert"));
    }

    #[test]
    fn thread_spawn() {
        let interp = crate::interpreter().expect("init");
        let spec = b"Thread.new { Thread.current }.join.value != Thread.current";
        let result = interp.eval(spec).expect("spec");
        assert!(result.try_into::<bool>().expect("convert"));
        let spec = b"Thread.new { Thread.current.name }.join.value != Thread.current.name";
        let result = interp.eval(spec).expect("spec");
        assert!(result.try_into::<bool>().expect("convert"));
        let spec = b"Thread.new { Thread.current }.join.value.alive? == false";
        let result = interp.eval(spec).expect("spec");
        assert!(result.try_into::<bool>().expect("convert"));
        let spec = b"Thread.new { Thread.current }.join.value.status == false";
        let result = interp.eval(spec).expect("spec");
        assert!(result.try_into::<bool>().expect("convert"));
    }

    #[test]
    fn thread_locals() {
        let interp = crate::interpreter().expect("init");
        let spec = br#"
Thread.current[:local] = 42
Thread.new { Thread.current.keys.empty? }.join.value
"#;
        let result = interp.eval(spec).expect("spec");
        assert!(result.try_into::<bool>().expect("convert"));
        let spec = br#"
Thread.current[:local] = 42
Thread.new { Thread.current[:local] = 96 }.join
Thread.current[:local] == 42
"#;
        let result = interp.eval(spec).expect("spec");
        assert!(result.try_into::<bool>().expect("convert"));
        let spec = br#"
Thread.current.thread_variable_set(:local, 42)
Thread.new { Thread.current.thread_variables.empty? }.join.value
"#;
        let result = interp.eval(spec).expect("spec");
        assert!(result.try_into::<bool>().expect("convert"));
        let spec = br#"
Thread.current.thread_variable_set(:local, 42)
Thread.new { Thread.current.thread_variable_set(:local, 96) }.join
Thread.current.thread_variable_get(:local) == 42
"#;
        let result = interp.eval(spec).expect("spec");
        assert!(result.try_into::<bool>().expect("convert"));
    }

    #[test]
    fn thread_abort_on_exception() {
        let interp = crate::interpreter().expect("init");
        let spec = br#"
Thread.abort_on_exception = true
Thread.new { raise 'failboat' }.join
"#;
        let result = interp.eval(spec);
        assert!(result.is_err());
        let spec = br#"
Thread.abort_on_exception = true
Thread.new do
  begin
    Thread.new { raise 'failboat' }.join
  rescue StandardError
    # swallow errors
  end
end.join
"#;
        let result = interp.eval(spec);
        assert!(result.is_err());
        let spec = br#"
Thread.abort_on_exception = false
Thread.new do
  begin
    Thread.new do
      Thread.current.abort_on_exception = true
      raise 'failboat'
    end.join
  rescue StandardError
    # swallow errors
  end
end.join
"#;
        let result = interp.eval(spec);
        assert!(result.is_err());

        let spec = r#"
Thread.abort_on_exception = false
Thread.new do
  begin
    Thread.new do
      begin
        Thread.new do
          Thread.current.abort_on_exception = true
          raise 'inner'
        end.join
        raise 'outer'
      rescue StandardError
        # swallow errors
      end
    end.join
    raise 'failboat'
  rescue StandardError
    # swallow errors
  end
end.join
        "#;
        let result = interp.eval(spec.trim().as_bytes()).map(|_| ());
        let expected_backtrace = r#"
(eval):8: inner (RuntimeError)
(eval):8:in call
/src/lib/thread.rb:122:in initialize
(eval):6:in call
/src/lib/thread.rb:122:in initialize
(eval):4:in call
/src/lib/thread.rb:122:in initialize
(eval):2
        "#;
        assert_eq!(
            result,
            Err(ArtichokeError::Exec(expected_backtrace.trim().to_owned()))
        );
    }
}