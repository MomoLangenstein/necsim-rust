#[macro_export]
#[expect(clippy::module_name_repetitions)]
macro_rules! ReporterGroup {
    () => {
        $crate::reporter::NullReporter
    };
    ($first_reporter:ident $(,$reporter_tail:ident)*) => {
        {
            unsafe { $crate::reporter::ReporterCombinator::new(
                $first_reporter,
                $crate::ReporterGroup![$($reporter_tail),*],
            ) }
        }
    }
}

#[macro_export]
#[expect(clippy::module_name_repetitions)]
macro_rules! ReporterUnGroup {
    ($reporter:expr => []) => {};
    ($reporter:expr => [$first_reporter:ident $(,$reporter_tail:ident)*]) => {
        {
            let (reporter_front, reporter_tail) = unsafe {
                $reporter.wen()
            };

            $first_reporter = reporter_front;

            $crate::ReporterUnGroup!{reporter_tail => [$($reporter_tail),*]}
        }
    }
}

#[macro_export]
macro_rules! ReporterGroupType {
    () => {
        $crate::reporter::NullReporter
    };
    ($first_reporter:ty $(,$reporter_tail:ty)*) => {
        $crate::reporter::ReporterCombinator<
            $first_reporter,
            $crate::ReporterGroupType![$($reporter_tail),*],
        >
    }
}
