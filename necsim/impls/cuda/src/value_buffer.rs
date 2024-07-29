#[cfg(not(target_os = "cuda"))]
use core::iter::Iterator;

use const_type_layout::TypeGraphLayout;
use rust_cuda::{
    safety::{PortableBitSemantics, SafeMutableAliasing, StackOnly},
    utils::{
        aliasing::SplitSliceOverCudaThreadsConstStride,
        exchange::buffer::{CudaExchangeBuffer, CudaExchangeItem},
    },
};

#[cfg(not(target_os = "cuda"))]
use rust_cuda::deps::rustacuda::{
    error::CudaResult,
    function::{BlockSize, GridSize},
};

use super::utils::MaybeSome;

#[derive(rust_cuda::lend::LendRustToCuda)]
#[cuda(free = "T")]
#[allow(clippy::module_name_repetitions)] // FIXME: use expect
pub struct ValueBuffer<T, const M2D: bool, const M2H: bool>
where
    T: StackOnly + PortableBitSemantics + TypeGraphLayout,
{
    #[cuda(embed)]
    mask: SplitSliceOverCudaThreadsConstStride<CudaExchangeBuffer<bool, true, true>, 1_usize>,
    #[cuda(embed)]
    buffer:
        SplitSliceOverCudaThreadsConstStride<CudaExchangeBuffer<MaybeSome<T>, M2D, M2H>, 1_usize>,
}

// Safety:
// - no mutable aliasing occurs since all parts implement SafeMutableAliasing
// - dropping does not trigger (de)alloc since ValueBuffer doesn't impl Drop and
//   all parts implement SafeMutableAliasing
// - ValueBuffer has no shallow mutable state
unsafe impl<T: StackOnly + PortableBitSemantics + TypeGraphLayout, const M2D: bool, const M2H: bool>
    SafeMutableAliasing for ValueBuffer<T, M2D, M2H>
where
    SplitSliceOverCudaThreadsConstStride<CudaExchangeBuffer<bool, true, true>, 1_usize>:
        SafeMutableAliasing,
    SplitSliceOverCudaThreadsConstStride<CudaExchangeBuffer<MaybeSome<T>, M2D, M2H>, 1_usize>:
        SafeMutableAliasing,
{
}

#[cfg(not(target_os = "cuda"))]
impl<T: StackOnly + PortableBitSemantics + TypeGraphLayout, const M2D: bool, const M2H: bool>
    ValueBuffer<T, M2D, M2H>
{
    /// # Errors
    /// Returns a `rustacuda::errors::CudaError` iff an error occurs inside CUDA
    pub fn new(block_size: &BlockSize, grid_size: &GridSize) -> CudaResult<Self> {
        let block_size = (block_size.x * block_size.y * block_size.z) as usize;
        let grid_size = (grid_size.x * grid_size.y * grid_size.z) as usize;
        let total_capacity = block_size * grid_size;

        let mut buffer = alloc::vec::Vec::with_capacity(total_capacity);
        buffer.resize_with(total_capacity, || MaybeSome::None);

        Ok(Self {
            mask: SplitSliceOverCudaThreadsConstStride::new(CudaExchangeBuffer::new(
                &false,
                total_capacity,
            )?),
            buffer: SplitSliceOverCudaThreadsConstStride::new(CudaExchangeBuffer::from_vec(
                buffer,
            )?),
        })
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }
}

#[cfg(not(target_os = "cuda"))]
impl<T: StackOnly + PortableBitSemantics + TypeGraphLayout, const M2D: bool>
    ValueBuffer<T, M2D, true>
{
    pub fn iter(&self) -> impl Iterator<Item = Option<&T>> {
        self.mask
            .iter()
            .zip(self.buffer.iter())
            .map(|(mask, maybe)| {
                if *mask.read() {
                    Some(unsafe { maybe.read().assume_some_ref() })
                } else {
                    None
                }
            })
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = ValueRefMut<T, M2D>> {
        self.mask
            .iter_mut()
            .zip(self.buffer.iter_mut())
            .map(|(mask, value)| ValueRefMut { mask, value })
    }
}

#[cfg(target_os = "cuda")]
impl<T: StackOnly + PortableBitSemantics + TypeGraphLayout> ValueBuffer<T, true, true> {
    pub fn with_value_for_core<F: FnOnce(Option<T>) -> Option<T>>(&mut self, inner: F) {
        let value = if self
            .mask
            .first()
            .map(CudaExchangeItem::read)
            .copied()
            .unwrap_or(false)
        {
            Some(unsafe { self.buffer.get_unchecked(0).read().assume_some_read() })
        } else {
            None
        };

        let result = inner(value);

        if let Some(mask) = self.mask.get_mut(0) {
            mask.write(result.is_some());

            if let Some(result) = result {
                unsafe { self.buffer.get_unchecked_mut(0) }.write(MaybeSome::Some(result));
            }
        }
    }
}

#[cfg(target_os = "cuda")]
impl<T: StackOnly + PortableBitSemantics + TypeGraphLayout, const M2H: bool>
    ValueBuffer<T, true, M2H>
{
    pub fn take_value_for_core(&mut self) -> Option<T> {
        if let Some(mask) = self.mask.get_mut(0) {
            mask.write(false);

            if *mask.read() {
                Some(unsafe { self.buffer.get_unchecked(0).read().assume_some_read() })
            } else {
                None
            }
        } else {
            None
        }
    }
}

#[cfg(target_os = "cuda")]
impl<T: StackOnly + PortableBitSemantics + TypeGraphLayout, const M2D: bool>
    ValueBuffer<T, M2D, true>
{
    pub fn put_value_for_core(&mut self, value: Option<T>) {
        if let Some(mask) = self.mask.get_mut(0) {
            mask.write(value.is_some());

            if let Some(value) = value {
                unsafe { self.buffer.get_unchecked_mut(0) }.write(MaybeSome::Some(value));
            }
        }
    }
}

#[cfg(not(target_os = "cuda"))]
pub struct ValueRefMut<'v, T: StackOnly + PortableBitSemantics + TypeGraphLayout, const M2D: bool> {
    mask: &'v mut CudaExchangeItem<bool, true, true>,
    value: &'v mut CudaExchangeItem<MaybeSome<T>, M2D, true>,
}

#[cfg(not(target_os = "cuda"))]
impl<'v, T: StackOnly + PortableBitSemantics + TypeGraphLayout, const M2D: bool>
    ValueRefMut<'v, T, M2D>
{
    pub fn take(&mut self) -> Option<T> {
        if *self.mask.read() {
            self.mask.write(false);

            Some(unsafe { self.value.read().assume_some_read() })
        } else {
            None
        }
    }

    #[must_use]
    pub fn as_ref(&self) -> Option<&T> {
        if *self.mask.read() {
            Some(unsafe { self.value.read().assume_some_ref() })
        } else {
            None
        }
    }
}

#[cfg(not(target_os = "cuda"))]
impl<'v, T: StackOnly + PortableBitSemantics + TypeGraphLayout> ValueRefMut<'v, T, true> {
    #[must_use]
    pub fn as_mut(&mut self) -> Option<&mut T> {
        if *self.mask.read() {
            Some(unsafe { self.value.as_mut().assume_some_mut() })
        } else {
            None
        }
    }

    pub fn replace(&mut self, value: Option<T>) -> Option<T> {
        let old = if *self.mask.read() {
            Some(unsafe { self.value.read().assume_some_read() })
        } else {
            None
        };

        self.mask.write(value.is_some());

        if let Some(value) = value {
            self.value.write(MaybeSome::Some(value));
        }

        old
    }
}
