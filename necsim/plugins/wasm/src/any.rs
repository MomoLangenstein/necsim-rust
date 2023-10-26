use core::{any, mem, mem::MaybeUninit, ptr};

pub struct Any {
    value: Value,
    drop: unsafe fn(&mut Value),
    fingerprint: Fingerprint,
}

union Value {
    ptr: *mut (),
    inline: [MaybeUninit<usize>; 2],
}

fn is_small<T>() -> bool {
    cfg!(not(no_maybe_uninit))
        && mem::size_of::<T>() <= mem::size_of::<Value>()
        && mem::align_of::<T>() <= mem::align_of::<Value>()
}

impl Any {
    // This is unsafe -- caller must not hold on to the Any beyond the lifetime
    // of T.
    //
    // Example of bad code:
    //
    //    let s = "bad".to_owned();
    //    let a = Any::new(&s);
    //    drop(s);
    //
    // Now `a.view()` and `a.take()` return references to a dead String.
    pub(crate) unsafe fn new<T>(t: T) -> Self {
        let value: Value;
        let drop: unsafe fn(&mut Value);
        let fingerprint = Fingerprint::of::<T>();

        if is_small::<T>() {
            let mut inline = [MaybeUninit::uninit(); 2];
            unsafe { ptr::write(inline.as_mut_ptr() as *mut T, t) };
            value = Value { inline };
            unsafe fn inline_drop<T>(value: &mut Value) {
                unsafe { ptr::drop_in_place(value.inline.as_mut_ptr() as *mut T) }
            }
            drop = inline_drop::<T>;
        } else {
            let ptr = Box::into_raw(Box::new(t)) as *mut ();
            value = Value { ptr };
            unsafe fn ptr_drop<T>(value: &mut Value) {
                mem::drop(unsafe { Box::from_raw(value.ptr as *mut T) });
            }
            drop = ptr_drop::<T>;
        };

        Any {
            value,
            drop,
            fingerprint,
        }
    }

    // This is unsafe -- caller is responsible that T is the correct type.
    pub(crate) unsafe fn view<T>(&mut self) -> &mut T {
        if cfg!(not(miri)) && self.fingerprint != Fingerprint::of::<T>() {
            self.invalid_cast_to::<T>();
        }

        let ptr = if is_small::<T>() {
            unsafe { self.value.inline.as_mut_ptr() as *mut T }
        } else {
            unsafe { self.value.ptr as *mut T }
        };

        unsafe { &mut *ptr }
    }

    // This is unsafe -- caller is responsible that T is the correct type.
    pub(crate) unsafe fn take<T>(mut self) -> T {
        if cfg!(not(miri)) && self.fingerprint != Fingerprint::of::<T>() {
            self.invalid_cast_to::<T>();
        }

        if is_small::<T>() {
            let ptr = unsafe { self.value.inline.as_mut_ptr() as *mut T };
            let value = unsafe { ptr::read(ptr) };
            mem::forget(self);
            value
        } else {
            let ptr = unsafe { self.value.ptr as *mut T };
            let box_t = unsafe { Box::from_raw(ptr) };
            mem::forget(self);
            *box_t
        }
    }

    fn invalid_cast_to<T>(&self) -> ! {
        let from = self.fingerprint.type_name;
        let to = any::type_name::<T>();
        panic!("invalid cast: {} to {}", from, to);
    }
}

impl Drop for Any {
    fn drop(&mut self) {
        unsafe { (self.drop)(&mut self.value) }
    }
}

#[derive(Debug, Eq, PartialEq)]
struct Fingerprint {
    size: usize,
    align: usize,
    type_name: &'static str,
}

impl Fingerprint {
    fn of<T>() -> Fingerprint {
        Fingerprint {
            size: mem::size_of::<T>(),
            align: mem::align_of::<T>(),
            type_name: any::type_name::<T>(),
        }
    }
}

#[test]
fn test_fingerprint() {
    assert_eq!(Fingerprint::of::<usize>(), Fingerprint::of::<usize>());
    assert_eq!(Fingerprint::of::<&str>(), Fingerprint::of::<&'static str>());

    assert_ne!(Fingerprint::of::<u32>(), Fingerprint::of::<[u8; 4]>());
    assert_ne!(Fingerprint::of::<u32>(), Fingerprint::of::<[u32; 2]>());

    if cfg!(all(include_fnptr_in_fingerprint, not(miri))) {
        assert_ne!(Fingerprint::of::<usize>(), Fingerprint::of::<isize>());
        assert_ne!(Fingerprint::of::<usize>(), Fingerprint::of::<&usize>());
        assert_ne!(Fingerprint::of::<&usize>(), Fingerprint::of::<&&usize>());
        assert_ne!(Fingerprint::of::<&usize>(), Fingerprint::of::<&mut usize>());

        struct A;
        struct B;
        assert_ne!(Fingerprint::of::<A>(), Fingerprint::of::<B>());
    }
}
