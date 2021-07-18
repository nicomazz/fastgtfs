#[cfg(not(target_arch = "wasm32"))]
use rayon::iter::{IntoParallelIterator, ParallelIterator};

#[cfg(not(target_arch = "wasm32"))]
pub trait IntoParallelIteratorIfPossible {
    type Iter: ParallelIterator<Item = Self::Item>;
    type Item: Send;

    fn into_par_iter_if_possible(self) -> Self::Iter;
}

#[cfg(not(target_arch = "wasm32"))]
impl<T: Send> IntoParallelIteratorIfPossible for Vec<T> {
    type Iter = rayon::vec::IntoIter<T>;
    type Item = T;

    fn into_par_iter_if_possible(self) -> Self::Iter {
        self.into_par_iter()
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub trait ParallelIteratorIfPossible<'data> {
    type Iter: ParallelIterator<Item = Self::Item>;
    type Item: Send + 'data;
    fn par_iter_if_possible(&'data self) -> Self::Iter;
}

#[cfg(not(target_arch = "wasm32"))]
impl<'data, I: 'data + ?Sized> ParallelIteratorIfPossible<'data> for I
where
    &'data I: IntoParallelIterator,
{
    type Iter = <&'data I as IntoParallelIterator>::Iter;
    type Item = <&'data I as IntoParallelIterator>::Item;

    fn par_iter_if_possible(&'data self) -> Self::Iter {
        self.into_par_iter()
    }
}

#[cfg(target_arch = "wasm32")]
pub trait IntoParallelIteratorIfPossible {
    type Iter: Iterator<Item = Self::Item>;
    type Item;

    fn into_par_iter_if_possible(self) -> Self::Iter;
}

#[cfg(target_arch = "wasm32")]
impl<T> IntoParallelIteratorIfPossible for Vec<T> {
    type Iter = std::vec::IntoIter<T>;
    type Item = T;

    fn into_par_iter_if_possible(self) -> Self::Iter {
        self.into_iter()
    }
}

#[cfg(target_arch = "wasm32")]
pub trait ParallelIteratorIfPossible<'data> {
    type Iter: Iterator<Item = Self::Item>;
    type Item: 'data;
    fn par_iter_if_possible(&'data self) -> Self::Iter;
}

#[cfg(target_arch = "wasm32")]
impl<'data, I: 'data> ParallelIteratorIfPossible<'data> for I
where
    &'data I: IntoIterator,
{
    type Iter = <&'data I as IntoIterator>::IntoIter; //core::slice::Iter<'data, T>;
    type Item = <&'data I as IntoIterator>::Item;
    fn par_iter_if_possible(&'data self) -> Self::Iter {
        self.into_iter()
    }
}
