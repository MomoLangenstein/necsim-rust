use mpi::request::{CancelGuard, LocalScope, Request};

#[expect(clippy::module_name_repetitions)]
pub struct DataOrRequest<'a, T: ?Sized, R: ?Sized> {
    value: &'a mut T,
    scope: &'a LocalScope<'a>,
    request: Box<Option<Request<'a, R, &'a LocalScope<'a>>>>,
}

impl<'a, T: ?Sized, R: ?Sized> DataOrRequest<'a, T, R> {
    #[must_use]
    pub fn new(value: &'a mut T, scope: &'a LocalScope<'a>) -> Self {
        Self {
            value,
            scope,
            request: Box::new(None),
        }
    }

    #[must_use]
    pub fn get_data(&self) -> Option<&T> {
        match &*self.request {
            None => Some(self.value),
            Some(_) => None,
        }
    }

    #[must_use]
    pub fn test_for_data_mut(&mut self) -> Option<&mut T> {
        match self.request.take().map(Request::test) {
            None | Some(Ok(_)) => Some(self.value),
            Some(Err(request)) => {
                *self.request = Some(request);

                None
            },
        }
    }

    pub fn request_if_data<
        F: for<'r> FnOnce(&'r mut T, &'r LocalScope<'r>) -> Request<'r, R, &'r LocalScope<'r>>,
    >(
        &mut self,
        do_request: F,
    ) {
        if self.request.is_none() {
            let request: Request<R, &LocalScope> = do_request(self.value, reduce_scope(self.scope));

            // Safety: upgrade of request scope back to the scope 'a
            let self_request: &mut Option<Request<R, &LocalScope>> = unsafe {
                &mut *(std::ptr::addr_of_mut!(*self.request)
                    .cast::<Option<Request<R, &LocalScope>>>())
            };

            *self_request = Some(request);
        }
    }
}

impl<'a, T: ?Sized, R: ?Sized> Drop for DataOrRequest<'a, T, R> {
    fn drop(&mut self) {
        if let Some(request) = self.request.take() {
            std::mem::drop(CancelGuard::from(request));
        }
    }
}

pub fn reduce_scope<'s, 'p: 's>(scope: &'s LocalScope<'p>) -> &'s LocalScope<'s> {
    // Safety: 'p outlives 's, so reducing the scope from 'p to 's is safe
    unsafe { std::mem::transmute(scope) }
}
