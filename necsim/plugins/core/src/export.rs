use std::mem::ManuallyDrop;

use necsim_core::reporter::{
    boolean::{Boolean, True},
    Reporter,
};

pub trait SerializeableReporter: Reporter + erased_serde::Serialize {
    fn reporter_name(&self) -> &'static str;
}

pub struct ReporterPluginDeclaration {
    pub rustc_version: &'static str,
    pub core_version: &'static str,

    #[expect(improper_ctypes_definitions)]
    pub init: unsafe extern "C" fn(&'static dyn log::Log, log::LevelFilter),

    #[expect(improper_ctypes_definitions)]
    pub deserialise:
        unsafe extern "C" fn(
            &mut dyn erased_serde::Deserializer,
        )
            -> Result<ManuallyDrop<UnsafeReporterPlugin>, erased_serde::Error>,

    #[expect(improper_ctypes_definitions)]
    pub library_path: unsafe extern "C" fn() -> Option<::std::path::PathBuf>,

    #[expect(improper_ctypes_definitions)]
    pub drop: unsafe extern "C" fn(ManuallyDrop<UnsafeReporterPlugin>),
}

#[derive(Copy, Clone)]
pub struct ReporterPluginFilter {
    pub(crate) report_speciation: bool,
    pub(crate) report_dispersal: bool,
    pub(crate) report_progress: bool,
}

impl ReporterPluginFilter {
    #[must_use]
    pub fn from_reporter<R: SerializeableReporter>() -> Self {
        Self {
            report_speciation: R::ReportSpeciation::VALUE,
            report_dispersal: R::ReportDispersal::VALUE,
            report_progress: R::ReportProgress::VALUE,
        }
    }
}

pub type DynReporterPlugin = dyn SerializeableReporter<
    ReportSpeciation = True,
    ReportDispersal = True,
    ReportProgress = True,
>;

#[repr(C)]
pub struct UnsafeReporterPlugin {
    pub(crate) reporter: Box<DynReporterPlugin>,
    pub(crate) filter: ReporterPluginFilter,
}

impl<R: SerializeableReporter> From<R> for UnsafeReporterPlugin {
    fn from(reporter: R) -> Self {
        let boxed_reporter: Box<
            dyn SerializeableReporter<
                ReportSpeciation = R::ReportSpeciation,
                ReportDispersal = R::ReportDispersal,
                ReportProgress = R::ReportProgress,
            >,
        > = Box::new(reporter);

        Self {
            reporter: unsafe {
                std::mem::transmute::<
                    Box<
                        dyn SerializeableReporter<
                            ReportDispersal = R::ReportDispersal,
                            ReportProgress = R::ReportProgress,
                            ReportSpeciation = R::ReportSpeciation,
                        >,
                    >,
                    Box<
                        dyn SerializeableReporter<
                            ReportDispersal = True,
                            ReportProgress = True,
                            ReportSpeciation = True,
                        >,
                    >,
                >(boxed_reporter)
            },
            filter: ReporterPluginFilter::from_reporter::<R>(),
        }
    }
}

#[macro_export]
#[expect(clippy::module_name_repetitions)]
macro_rules! export_plugin {
    ($($name:ident => $plugin:ty),+$(,)?) => {
        #[doc(hidden)]
        extern "C" fn __necsim_reporter_plugin_init(
            log: &'static dyn $crate::log::Log,
            max_level: $crate::log::LevelFilter,
        ) {
            let _ = $crate::log::set_logger(log);

            $crate::log::set_max_level(max_level);
        }

        #[doc(hidden)]
        extern "C" fn __necsim_reporter_plugin_deserialise<'de>(
            deserializer: &mut dyn $crate::erased_serde::Deserializer<'de>,
        ) -> Result<
            ::std::mem::ManuallyDrop<$crate::export::UnsafeReporterPlugin>,
            $crate::erased_serde::Error,
        > {
            #[expect(clippy::enum_variant_names)]
            #[derive($crate::serde::Deserialize)]
            #[serde(crate = "::necsim_plugins_core::serde")]
            enum Reporters {
                $($name($plugin)),*
            }

            $crate::erased_serde::deserialize::<Reporters>(deserializer).map(|reporter| {
                match reporter {
                    $(Reporters::$name(reporter) => reporter.into()),*
                }
            }).map(::std::mem::ManuallyDrop::new)
        }

        $(impl $crate::export::SerializeableReporter for $plugin {
            fn reporter_name(&self) -> &'static str {
                stringify!($name)
            }
        })*

        #[doc(hidden)]
        extern "C" fn __necsim_reporter_plugin_library_path() -> Option<::std::path::PathBuf> {
            ::necsim_plugins_core::process_path::get_dylib_path()
        }

        #[doc(hidden)]
        extern "C" fn __necsim_reporter_plugin_drop(
            plugin: ::std::mem::ManuallyDrop<$crate::export::UnsafeReporterPlugin>,
        ) {
            ::std::mem::drop(::std::mem::ManuallyDrop::into_inner(plugin))
        }

        #[doc(hidden)]
        #[no_mangle]
        pub static NECSIM_REPORTER_PLUGIN_DECLARATION: $crate::export::ReporterPluginDeclaration =
            $crate::export::ReporterPluginDeclaration {
                rustc_version: $crate::RUSTC_VERSION,
                core_version: $crate::CORE_VERSION,

                init: __necsim_reporter_plugin_init,
                deserialise: __necsim_reporter_plugin_deserialise,
                library_path: __necsim_reporter_plugin_library_path,
                drop: __necsim_reporter_plugin_drop,
            };
    };
}

pub enum Reporters<'r> {
    DynReporter(&'r DynReporterPlugin),
}

impl<'r> serde::Serialize for Reporters<'r> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        struct Reporter<'r>(&'r DynReporterPlugin);

        impl<'r> serde::Serialize for Reporter<'r> {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                erased_serde::serialize(self.0, serializer)
            }
        }

        let Self::DynReporter(reporter) = self;

        serializer.serialize_newtype_variant(
            "Reporters",
            0,
            reporter.reporter_name(),
            &Reporter(*reporter),
        )
    }
}
