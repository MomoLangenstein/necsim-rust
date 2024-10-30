#![allow(clippy::doc_markdown)]
/// no-std fixed-size two-dimensional array with `RustToCuda` support
///
/// Based on a subset of Harrison McCullough's MIT-licensed [`array2d`] crate.
///
/// [`array2d`]: https://github.com/HarrisonMc555/array2d
use alloc::{boxed::Box, rc::Rc, sync::Arc, vec::Vec};

use core::{
    marker::PhantomData,
    ops::{Deref, DerefMut, Index, IndexMut},
};

pub trait ArrayBackend<T>: From<Vec<T>> + Deref<Target = [T]> {}
impl<T, B: From<Vec<T>> + Deref<Target = [T]>> ArrayBackend<T> for B {}

/// A fixed sized two-dimensional array.
#[cfg_attr(feature = "cuda", derive(rust_cuda::lend::LendRustToCuda))]
#[cfg_attr(
    feature = "cuda",
    cuda(
        free = "T",
        bound = "T: rust_cuda::safety::PortableBitSemantics + const_type_layout::TypeGraphLayout"
    )
)]
pub struct Array2D<T, B: ArrayBackend<T> = Box<[T]>> {
    #[cfg_attr(feature = "cuda", cuda(embed))]
    array: B,
    num_rows: usize,
    marker: PhantomData<T>,
}

impl<T, B: ArrayBackend<T> + Clone> Clone for Array2D<T, B> {
    fn clone(&self) -> Self {
        Self {
            array: self.array.clone(),
            num_rows: self.num_rows,
            marker: PhantomData::<T>,
        }
    }
}

impl<T, B: ArrayBackend<T> + PartialEq> PartialEq for Array2D<T, B> {
    fn eq(&self, other: &Self) -> bool {
        (self.array == other.array) && (self.num_rows == other.num_rows)
    }
}
impl<T, B: ArrayBackend<T> + Eq> Eq for Array2D<T, B> {}

pub type ArcArray2D<T> = Array2D<T, Arc<[T]>>;
pub type BoxArray2D<T> = Array2D<T, Box<[T]>>;
pub type RcArray2D<T> = Array2D<T, Rc<[T]>>;
pub type VecArray2D<T> = Array2D<T, Vec<T>>;

impl<T, B: ArrayBackend<T>> core::fmt::Debug for Array2D<T, B> {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(
            fmt,
            "Array2D<{}; WxH = {}x{}>",
            core::any::type_name::<T>(),
            self.num_columns(),
            self.num_rows()
        )
    }
}

/// An error that can arise during the use of an [`Array2D`].
///
/// [`Array2D`]: struct.Array2D.html
#[derive(Debug, Eq, PartialEq)]
pub enum Error {
    /// The given indices were out of bounds.
    IndicesOutOfBounds(usize, usize),
    /// The dimensions given did not match the elements provided
    DimensionMismatch,
    /// There were not enough elements to fill the array.
    NotEnoughElements,
}

impl<T, B: ArrayBackend<T>> Array2D<T, B> {
    /// Creates a new [`Array2D`] from a slice of rows, each of which is a
    /// [`Vec`] of elements.
    ///
    /// # Errors
    ///
    /// Returns an `DimensionMismatch` error if the rows are not all the same
    /// size.
    ///
    /// # Examples
    ///
    /// ```
    /// # use necsim_impls_no_std::array2d::{Array2D, Error};
    /// # fn main() -> Result<(), Error> {
    /// let rows = vec![vec![1, 2, 3], vec![4, 5, 6]];
    /// let array = Array2D::<_>::from_rows(&rows)?;
    /// assert_eq!(array[(1, 2)], 6);
    /// assert_eq!(array.as_rows(), rows);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`Array2D`]: struct.Array2D.html
    /// [`Vec`]: https://doc.rust-lang.org/std/vec/struct.Vec.html
    pub fn from_rows(elements: &[Vec<T>]) -> Result<Self, Error>
    where
        T: Clone,
    {
        let row_len = elements.first().map_or(0, Vec::len);
        if !elements.iter().all(|row| row.len() == row_len) {
            return Err(Error::DimensionMismatch);
        }
        Ok(Array2D {
            array: elements
                .iter()
                .flat_map(Vec::clone)
                .collect::<Vec<_>>()
                .into(),
            num_rows: elements.len(),
            marker: PhantomData::<T>,
        })
    }

    /// Creates a new [`Array2D`] from the given flat slice in [row major
    /// order].
    ///
    /// Returns an error if the number of elements in `elements` is not the
    /// product of `num_rows` and `num_columns`, i.e. the dimensions do not
    /// match.
    ///
    /// # Errors
    ///
    /// Returns a `DimensionMismatch` error if
    ///  `elements.len() != num_rows * num_columns`
    ///
    /// # Examples
    ///
    /// ```
    /// # use necsim_impls_no_std::array2d::{Array2D, Error};
    /// # fn main() -> Result<(), Error> {
    /// let row_major = vec![1, 2, 3, 4, 5, 6];
    /// let array = Array2D::<_>::from_row_major(&row_major, 2, 3)?;
    /// assert_eq!(array[(1, 2)], 6);
    /// assert_eq!(array.as_rows(), vec![vec![1, 2, 3], vec![4, 5, 6]]);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`Array2D`]: struct.Array2D.html
    /// [row major order]: https://en.wikipedia.org/wiki/Row-_and_column-major_order
    pub fn from_row_major(
        elements: &[T],
        num_rows: usize,
        num_columns: usize,
    ) -> Result<Self, Error>
    where
        T: Clone,
    {
        let total_len = num_rows * num_columns;
        if total_len != elements.len() {
            return Err(Error::DimensionMismatch);
        }
        Ok(Array2D {
            array: elements.to_vec().into(),
            num_rows,
            marker: PhantomData::<T>,
        })
    }

    /// Creates a new [`Array2D`] with the specified number of rows and columns
    /// that contains `element` in every location.
    ///
    /// # Examples
    ///
    /// ```
    /// # use necsim_impls_no_std::array2d::{Array2D, Error};
    /// let array = Array2D::<_>::filled_with(42, 2, 3);
    /// assert_eq!(array.as_rows(), vec![vec![42, 42, 42], vec![42, 42, 42]]);
    /// ```
    ///
    /// [`Array2D`]: struct.Array2D.html
    pub fn filled_with(element: T, num_rows: usize, num_columns: usize) -> Self
    where
        T: Clone,
    {
        let total_len = num_rows * num_columns;
        let array = alloc::vec![element; total_len];
        Array2D {
            array: array.into(),
            num_rows,
            marker: PhantomData::<T>,
        }
    }

    /// Creates a new [`Array2D`] with the specified number of rows and columns
    /// and fills each element with the elements produced from the provided
    /// iterator. If the iterator produces more than enough elements, the
    /// remaining are unused. Returns an error if the iterator does not produce
    /// enough elements.
    ///
    /// The elements are inserted into the array in [row major order].
    ///
    /// # Errors
    ///
    /// Returns a `NotEnoughElements` error if
    ///  `iterator.len() < num_rows * num_columns`
    ///
    /// # Examples
    ///
    /// ```
    /// # use necsim_impls_no_std::array2d::{Array2D, Error};
    /// # fn main() -> Result<(), Error> {
    /// let iterator = (1..);
    /// let array = Array2D::<_>::from_iter_row_major(iterator, 2, 3)?;
    /// assert_eq!(array.as_rows(), vec![vec![1, 2, 3], vec![4, 5, 6]]);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`Array2D`]: struct.Array2D.html
    /// [row major order]: https://en.wikipedia.org/wiki/Row-_and_column-major_order
    pub fn from_iter_row_major<I>(
        iterator: I,
        num_rows: usize,
        num_columns: usize,
    ) -> Result<Self, Error>
    where
        I: Iterator<Item = T>,
    {
        let total_len = num_rows * num_columns;
        let array = iterator.take(total_len).collect::<Vec<_>>();
        if array.len() != total_len {
            return Err(Error::NotEnoughElements);
        }
        Ok(Array2D {
            array: array.into(),
            num_rows,
            marker: PhantomData::<T>,
        })
    }

    /// The number of rows.
    #[must_use]
    pub fn num_rows(&self) -> usize {
        self.num_rows
    }

    /// The number of columns.
    #[must_use]
    pub fn num_columns(&self) -> usize {
        self.array.len() / self.num_rows
    }

    /// The total number of elements, i.e. the product of `num_rows` and
    /// `num_columns`.
    #[must_use]
    pub fn num_elements(&self) -> usize {
        self.array.len()
    }

    /// The number of elements in each row, i.e. the number of columns.
    #[must_use]
    pub fn row_len(&self) -> usize {
        self.num_columns()
    }

    /// The number of elements in each column, i.e. the number of rows.
    #[must_use]
    pub fn column_len(&self) -> usize {
        self.num_rows()
    }

    /// Returns a reference to the element at the given `row` and `column` if
    /// the index is in bounds (wrapped in [`Some`]). Returns [`None`] if
    /// the index is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// # use necsim_impls_no_std::array2d::{Array2D, Error};
    /// let array = Array2D::<_>::filled_with(42, 2, 3);
    /// assert_eq!(array.get(0, 0), Some(&42));
    /// assert_eq!(array.get(10, 10), None);
    /// ```
    ///
    /// [`Some`]: https://doc.rust-lang.org/std/option/enum.Option.html#variant.Some
    /// [`None`]: https://doc.rust-lang.org/std/option/enum.Option.html#variant.None
    #[must_use]
    pub fn get(&self, row: usize, column: usize) -> Option<&T> {
        self.get_index(row, column).map(|index| &self.array[index])
    }

    /// Returns a mutable reference to the element at the given `row` and
    /// `column` if the index is in bounds (wrapped in [`Some`]). Returns
    /// [`None`] if the index is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// # use necsim_impls_no_std::array2d::{Array2D, Error};
    /// let mut array = Array2D::<_>::filled_with(42, 2, 3);
    ///
    /// assert_eq!(array.get_mut(0, 0), Some(&mut 42));
    /// assert_eq!(array.get_mut(10, 10), None);
    ///
    /// array.get_mut(0, 0).map(|x| *x = 100);
    /// assert_eq!(array.get(0, 0), Some(&100));
    ///
    /// array.get_mut(10, 10).map(|x| *x = 200);
    /// assert_eq!(array.get(10, 10), None);
    /// ```
    ///
    /// [`Some`]: https://doc.rust-lang.org/std/option/enum.Option.html#variant.Some
    /// [`None`]: https://doc.rust-lang.org/std/option/enum.Option.html#variant.None
    pub fn get_mut(&mut self, row: usize, column: usize) -> Option<&mut T>
    where
        B: DerefMut,
    {
        self.get_index(row, column)
            .map(move |index| &mut self.array[index])
    }

    /// Returns an [`Iterator`] over references to all elements in the given
    /// row. Returns an error if the index is out of bounds.
    ///
    /// # Errors
    ///
    /// Returns a `IndicesOutOfBounds(row_index, 0)` error
    ///  if `row_index >= self.num_rows()`
    ///
    /// # Examples
    ///
    /// ```
    /// # use necsim_impls_no_std::array2d::{Array2D, Error};
    /// # fn main() -> Result<(), Error> {
    /// let rows = vec![vec![1, 2, 3], vec![4, 5, 6]];
    /// let array = Array2D::<_>::from_rows(&rows)?;
    /// let mut row_iter = array.row_iter(1)?;
    /// assert_eq!(row_iter.next(), Some(&4));
    /// assert_eq!(row_iter.next(), Some(&5));
    /// assert_eq!(row_iter.next(), Some(&6));
    /// assert_eq!(row_iter.next(), None);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`Iterator`]: https://doc.rust-lang.org/std/iter/trait.Iterator.html
    pub fn row_iter(&self, row_index: usize) -> Result<impl DoubleEndedIterator<Item = &T>, Error> {
        let start = self
            .get_index(row_index, 0)
            .ok_or(Error::IndicesOutOfBounds(row_index, 0))?;
        let end = start + self.row_len();
        Ok(self.array[start..end].iter())
    }

    #[expect(clippy::missing_panics_doc)]
    /// Returns an [`Iterator`] over all rows. Each [`Item`] is itself another
    /// [`Iterator`] over references to the elements in that row.
    ///
    /// # Examples
    ///
    /// ```
    /// # use necsim_impls_no_std::array2d::{Array2D, Error};
    /// # fn main() -> Result<(), Error> {
    /// let rows = vec![vec![1, 2, 3], vec![4, 5, 6]];
    /// let array = Array2D::<_>::from_rows(&rows)?;
    /// for row_iter in array.rows_iter() {
    ///     for element in row_iter {
    ///         print!("{} ", element);
    ///     }
    ///     println!();
    /// }
    ///
    /// let mut rows_iter = array.rows_iter();
    ///
    /// let mut first_row_iter = rows_iter.next().unwrap();
    /// assert_eq!(first_row_iter.next(), Some(&1));
    /// assert_eq!(first_row_iter.next(), Some(&2));
    /// assert_eq!(first_row_iter.next(), Some(&3));
    /// assert_eq!(first_row_iter.next(), None);
    ///
    /// let mut second_row_iter = rows_iter.next().unwrap();
    /// assert_eq!(second_row_iter.next(), Some(&4));
    /// assert_eq!(second_row_iter.next(), Some(&5));
    /// assert_eq!(second_row_iter.next(), Some(&6));
    /// assert_eq!(second_row_iter.next(), None);
    ///
    /// assert!(rows_iter.next().is_none());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`Iterator`]: https://doc.rust-lang.org/std/iter/trait.Iterator.html
    /// [`Item`]: https://doc.rust-lang.org/std/iter/trait.Iterator.html#associatedtype.Item
    #[must_use]
    pub fn rows_iter(
        &self,
    ) -> impl DoubleEndedIterator<Item = impl DoubleEndedIterator<Item = &T>> {
        (0..self.num_rows()).map(move |row_index| {
            self.row_iter(row_index)
                .expect("rows_iter should never fail")
        })
    }

    /// Collects the [`Array2D`] into a [`Vec`] of rows, each of which contains
    /// a [`Vec`] of elements.
    ///
    /// # Examples
    ///
    /// ```
    /// # use necsim_impls_no_std::array2d::{Array2D, Error};
    /// # fn main() -> Result<(), Error> {
    /// let rows = vec![vec![1, 2, 3], vec![4, 5, 6]];
    /// let array = Array2D::<_>::from_rows(&rows)?;
    /// assert_eq!(array.as_rows(), rows);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`Array2D`]: struct.Array2D.html
    /// [`Vec`]: https://doc.rust-lang.org/std/vec/struct.Vec.html
    #[must_use]
    pub fn as_rows(&self) -> Vec<Vec<T>>
    where
        T: Clone,
    {
        self.rows_iter()
            .map(|row_iter| row_iter.cloned().collect())
            .collect()
    }

    /// Converts the [`Array2D`] into a [`Vec`] of elements in [row major
    /// order].
    ///
    /// # Examples
    ///
    /// ```
    /// # use necsim_impls_no_std::array2d::{Array2D, Error};
    /// # fn main() -> Result<(), Error> {
    /// let rows = vec![vec![1, 2, 3], vec![4, 5, 6]];
    /// let array = Array2D::<_>::from_rows(&rows)?;
    /// assert_eq!(array.into_row_major(), vec![1, 2, 3, 4, 5, 6]);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`Array2D`]: struct.Array2D.html
    /// [`Vec`]: https://doc.rust-lang.org/std/vec/struct.Vec.html
    /// [row major order]: https://en.wikipedia.org/wiki/Row-_and_column-major_order
    #[must_use]
    pub fn into_row_major(self) -> Vec<T>
    where
        B: Into<Vec<T>>,
    {
        self.array.into()
    }

    /// Converts the [`Array2D`] into its inner [`ArrayBackend`] `B` of
    /// elements in [row major order].
    ///
    /// # Examples
    ///
    /// ```
    /// # use necsim_impls_no_std::array2d::{Error, VecArray2D};
    /// # fn main() -> Result<(), Error> {
    /// let rows = vec![vec![1, 2, 3], vec![4, 5, 6]];
    /// let array = VecArray2D::from_rows(&rows)?;
    /// assert_eq!(array.into_row_major_backend(), vec![1, 2, 3, 4, 5, 6]);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`Array2D`]: struct.Array2D.html
    /// [row major order]: https://en.wikipedia.org/wiki/Row-_and_column-major_order
    #[must_use]
    pub fn into_row_major_backend(self) -> B {
        self.array
    }

    /// Converts the [`Array2D`] into from its current into a new
    /// [`ArrayBackend`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use necsim_impls_no_std::array2d::{BoxArray2D, Error, VecArray2D};
    /// # fn main() -> Result<(), Error> {
    /// let rows = vec![vec![1, 2, 3], vec![4, 5, 6]];
    /// let array = VecArray2D::from_rows(&rows)?;
    /// let array: BoxArray2D<_> = array.switch_backend();
    /// assert_eq!(array.into_row_major(), vec![1, 2, 3, 4, 5, 6]);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`Array2D`]: struct.Array2D.html
    /// [row major order]: https://en.wikipedia.org/wiki/Row-_and_column-major_order
    #[must_use]
    pub fn switch_backend<B2: ArrayBackend<T> + From<B>>(self) -> Array2D<T, B2> {
        Array2D {
            array: self.array.into(),
            num_rows: self.num_rows,
            marker: PhantomData::<T>,
        }
    }

    /// Returns an [`Iterator`] over references to all elements in
    /// [row major order].
    ///
    /// # Examples
    ///
    /// ```
    /// # use necsim_impls_no_std::array2d::{Array2D, Error};
    /// # fn main() -> Result<(), Error> {
    /// let rows = vec![vec![1, 2, 3], vec![4, 5, 6]];
    /// let elements = vec![1, 2, 3, 4, 5, 6];
    /// let array = Array2D::<_>::from_rows(&rows)?;
    /// let row_major = array.elements_row_major_iter();
    /// assert_eq!(row_major.cloned().collect::<Vec<_>>(), elements);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`Iterator`]: https://doc.rust-lang.org/std/iter/trait.Iterator.html
    /// [row major order]: https://en.wikipedia.org/wiki/Row-_and_column-major_order
    #[must_use]
    pub fn elements_row_major_iter(&self) -> impl DoubleEndedIterator<Item = &T> {
        self.array.iter()
    }

    fn get_index(&self, row: usize, column: usize) -> Option<usize> {
        if row < self.num_rows && column < self.num_columns() {
            Some(row * self.row_len() + column)
        } else {
            None
        }
    }
}

impl<T, B: ArrayBackend<T>> Index<(usize, usize)> for Array2D<T, B> {
    type Output = T;

    /// Returns the element at the given indices, given as `(row, column)`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use necsim_impls_no_std::array2d::{Array2D, Error};
    /// let array = Array2D::<_>::filled_with(42, 2, 3);
    /// assert_eq!(array[(0, 0)], 42);
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the indices are out of bounds.
    ///
    /// ```rust,should_panic
    /// # use necsim_impls_no_std::array2d::Array2D;
    /// let array = Array2D::<_>::filled_with(42, 2, 3);
    /// let element = array[(10, 10)];
    /// ```
    fn index(&self, (row, column): (usize, usize)) -> &Self::Output {
        self.get(row, column)
            .unwrap_or_else(|| panic!("Index indices {}, {} out of bounds", row, column))
    }
}

impl<T, B: ArrayBackend<T> + DerefMut> IndexMut<(usize, usize)> for Array2D<T, B> {
    /// Returns a mutable version of the element at the given indices, given as
    /// `(row, column)`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use necsim_impls_no_std::array2d::{Array2D, Error};
    /// let mut array = Array2D::<_>::filled_with(42, 2, 3);
    /// array[(0, 0)] = 100;
    /// assert_eq!(array[(0, 0)], 100);
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the indices are out of bounds.
    ///
    /// ```rust,should_panic
    /// # use necsim_impls_no_std::array2d::Array2D;
    /// let mut array = Array2D::<_>::filled_with(42, 2, 3);
    /// array[(10, 10)] = 7;
    /// ```
    fn index_mut(&mut self, (row, column): (usize, usize)) -> &mut Self::Output {
        self.get_mut(row, column)
            .unwrap_or_else(|| panic!("Index mut indices {}, {} out of bounds", row, column))
    }
}
