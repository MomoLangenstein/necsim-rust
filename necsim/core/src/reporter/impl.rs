#[macro_export]
#[expect(clippy::module_name_repetitions)]
macro_rules! impl_report {
    // Special case: Ignored = MaybeUsed<False>
    ($(#[$metas:meta])* $([$default:tt])? $target:ident(&mut $this:ident, $value:ident: Ignored) {}) =>
    {
        impl_report!{
            $(#[$metas])*
            $([$default])? $target(&mut $this, $value: MaybeUsed<$crate::reporter::boolean::False>) {}
        }
    };
    // Special case: Used = MaybeUsed<True>
    ($(#[$metas:meta])* $([$default:tt])? $target:ident(&mut $this:ident, $value:ident: Used) $code:block) =>
    {
        impl_report!{
            $(#[$metas])*
            $([$default])? $target(&mut $this, $value: MaybeUsed<$crate::reporter::boolean::True>)
            $code
        }
    };
    // Dispatch case: MaybeUsed + speciation
    ($(#[$metas:meta])* $([$default:tt])? speciation(&mut $this:ident, $value:ident: MaybeUsed<$Usage:ty>)
        $code:block) =>
    {
        impl_report!{
            $(#[$metas])*
            $([$default])? fn report_speciation(&mut $this, $value: MaybeUsed<
                $crate::event::SpeciationEvent, ReportSpeciation = $Usage
            >) $code
        }
    };
    // Dispatch case: MaybeUsed + dispersal
    ($(#[$metas:meta])* $([$default:tt])? dispersal(&mut $this:ident, $value:ident: MaybeUsed<$Usage:ty>)
        $code:block) =>
    {
        impl_report!{
            $(#[$metas])*
            $([$default])? fn report_dispersal(&mut $this, $value: MaybeUsed<
                $crate::event::DispersalEvent, ReportDispersal = $Usage
            >) $code
        }
    };
    // Dispatch case: MaybeUsed + progress
    ($(#[$metas:meta])* $([$default:tt])? progress(&mut $this:ident, $value:ident: MaybeUsed<$Usage:ty>)
        $code:block) =>
    {
        impl_report!{
            $(#[$metas])*
            $([$default])? fn report_progress(&mut $this, $value: MaybeUsed<u64, ReportProgress = $Usage>) $code
        }
    };
    // Impl case: MaybeUsed
    ($(#[$metas:meta])* $([$default:tt])? fn $target:ident(&mut $this:ident, $value:ident: MaybeUsed<
        $EventTy:ty, $UsageIdent:ident = $UsageTy:ty
    >) $code:block) =>
    {
        $(#[$metas])*
        $($default)? fn $target(
            &mut $this,
            $value: &$crate::reporter::used::MaybeUsed<$EventTy, Self::$UsageIdent>,
        ) {
            $value.maybe_use_in(|$value| $code)
        }

        $($default)? type $UsageIdent = $UsageTy;
    };
}

#[macro_export]
#[expect(clippy::module_name_repetitions)]
macro_rules! impl_finalise {
    ($(#[$metas:meta])* ($self:ident) $code:block) => {
        $(#[$metas])*
        fn finalise($self) where Self:Sized {
            $code
        }

        $(#[$metas])*
        unsafe fn finalise_boxed($self: $crate::alloc::boxed::Box<Self>) {
            $code
        }
    };
    ($(#[$metas:meta])* (mut $self:ident) $code:block) => {
        $(#[$metas])*
        fn finalise(mut $self) where Self:Sized {
            $code
        }

        $(#[$metas])*
        unsafe fn finalise_boxed(mut $self: $crate::alloc::boxed::Box<Self>) {
            $code
        }
    };
}
