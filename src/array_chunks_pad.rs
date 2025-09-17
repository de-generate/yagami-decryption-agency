use std::iter::ArrayChunks;

pub trait ArrayChunksPadExtension: Iterator
where
    Self: Sized,
    Self::Item: Copy,
{
    fn array_chunks_pad<const N: usize>(self, filler: Self::Item) -> ArrayChunksPad<Self, N>;
}

impl<I> ArrayChunksPadExtension for I
where
    I: Iterator,
    I::Item: Copy,
{
    fn array_chunks_pad<const N: usize>(self, filler: Self::Item) -> ArrayChunksPad<Self, N>
    where
        Self: Sized,
        Self::Item: Copy,
    {
        ArrayChunksPad::new(self, filler)
    }
}

pub struct ArrayChunksPad<I, const N: usize>
where
    I: Iterator,
    I::Item: Copy,
{
    iter: Option<ArrayChunks<I, N>>,
    filler: I::Item,
}

impl<I, const N: usize> ArrayChunksPad<I, N>
where
    I: Iterator,
    I::Item: Copy,
{
    pub fn new(iter: I, filler: I::Item) -> Self {
        Self {
            iter: Some(iter.array_chunks::<N>()),
            filler,
        }
    }
}

impl<I, const N: usize> Iterator for ArrayChunksPad<I, N>
where
    I: Iterator,
    I::Item: Copy,
{
    type Item = [I::Item; N];

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(iter) = &mut self.iter {
            match iter.next() {
                None => {
                    let mut remainder = self
                        .iter
                        .take()
                        .unwrap()
                        .into_remainder()
                        .unwrap()
                        .peekable();

                    if remainder.peek().is_some() {
                        let mut result = [self.filler; N];

                        for (i, remains) in remainder.enumerate() {
                            result[i] = remains;
                        }

                        Some(result)
                    } else {
                        None
                    }
                }
                el => el,
            }
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test(input: Vec<u8>, expected: Vec<[u8; 4]>) {
        assert_eq!(
            input
                .into_iter()
                .array_chunks_pad::<4>(0)
                .collect::<Vec<_>>(),
            expected
        );
    }

    #[test]
    fn test_0() {
        test(vec![1, 2, 3, 4], vec![[1, 2, 3, 4]]);
    }

    #[test]
    fn test_1() {
        test(vec![1, 2, 3, 4, 5], vec![[1, 2, 3, 4], [5, 0, 0, 0]]);
    }

    #[test]
    fn test_2() {
        test(vec![1, 2, 3, 4, 5, 6], vec![[1, 2, 3, 4], [5, 6, 0, 0]]);
    }

    #[test]
    fn test_3() {
        test(vec![1, 2, 3, 4, 5, 6, 7], vec![[1, 2, 3, 4], [5, 6, 7, 0]]);
    }

    #[test]
    fn test_4() {
        test(
            vec![1, 2, 3, 4, 5, 6, 7, 8],
            vec![[1, 2, 3, 4], [5, 6, 7, 8]],
        );
    }

    #[test]
    fn test_5() {
        test(
            vec![1, 2, 3, 4, 5, 6, 7, 8, 9],
            vec![[1, 2, 3, 4], [5, 6, 7, 8], [9, 0, 0, 0]],
        );
    }
}
