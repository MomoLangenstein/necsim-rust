use std::{fmt, mem::ManuallyDrop, rc::Rc};

use crate::{
    export::{DynReporterPlugin, ReporterPluginFilter, UnsafeReporterPlugin},
    import::serde::PluginLibrary,
};

#[expect(clippy::module_name_repetitions)]
pub struct ReporterPlugin {
    pub(crate) library: Rc<PluginLibrary>,

    pub(crate) reporter: ManuallyDrop<Box<DynReporterPlugin>>,
    pub(crate) filter: ReporterPluginFilter,

    pub(crate) finalised: bool,
}

impl ReporterPlugin {
    pub(crate) fn finalise(mut self) {
        self.finalised = true;

        std::mem::drop(self);
    }
}

impl Drop for ReporterPlugin {
    fn drop(&mut self) {
        if self.finalised {
            unsafe {
                ManuallyDrop::take(&mut self.reporter).finalise_boxed();
            }
        } else {
            unsafe {
                (self.library.declaration.drop)(ManuallyDrop::new(UnsafeReporterPlugin {
                    reporter: ManuallyDrop::take(&mut self.reporter),
                    filter: self.filter,
                }));
            }
        }
    }
}

impl fmt::Debug for ReporterPlugin {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&*self.reporter, fmt)
    }
}
