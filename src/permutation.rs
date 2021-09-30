use std::convert::TryInto;

use itertools::Itertools;
use num::Integer;

#[derive(Debug)]
pub struct IncompatiblePermutationSizes;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Permutation<T>
where
    T: TryInto<usize> + Clone + PartialEq + Default,
    <T as TryInto<usize>>::Error: std::fmt::Debug,
{
    pub raw: Vec<T>,
    cycles: Option<Vec<Vec<T>>>,
}

impl<T> Permutation<T>
where
    T: TryInto<usize> + Clone + PartialEq + Default,
    <T as TryInto<usize>>::Error: std::fmt::Debug,
{
    /// Builds a new permutation from the given vector but doesn't
    /// compute the cycles explicitly.
    pub fn new(raw: Vec<T>) -> Self {
        Permutation { raw, cycles: None }
    }

    /// Builds a new permutation from the given vector and
    /// compute the cycles explicitly.
    pub fn new_with_cycles(raw: Vec<T>) -> Self {
        let mut new = Permutation { raw, cycles: None };
        new.compute_cycles();
        new
    }

    /// Builds a new permutation form the given cycles.
    /// The cycles need to contain single element cycles as well.
    pub fn _from_cycles(cycles: Vec<Vec<T>>, size: usize) -> Self {
        let mut raw = vec![T::default(); size];

        for cycle in cycles.iter() {
            Self::_from_cycle(cycle, &mut raw);
        }

        let cycles = cycles.into_iter().filter(|cycle| cycle.len() > 1).collect();

        Permutation {
            raw,
            cycles: Some(cycles),
        }
    }

    /// Standard composition of permutations where the right
    /// permutation is applied first (i.e. the inner one).
    ///
    /// (x . y)(a) = x(y(a))
    pub fn _compose(left: &Self, right: &Self) -> Result<Self, IncompatiblePermutationSizes> {
        if left.len() != right.len() {
            return Err(IncompatiblePermutationSizes);
        }

        let mut compositum = Permutation {
            cycles: None,
            ..right.clone()
        };
        compositum._compose_with(left)?;

        Ok(compositum)
    }

    /// Composes another permutation with this permutation.
    /// The other permutation is the subsequent one.
    ///
    /// In-place version of [`Permutation::compose`].
    pub fn _compose_with(
        &mut self,
        subsequent_perm: &Self,
    ) -> Result<(), IncompatiblePermutationSizes> {
        if self.len() != subsequent_perm.len() {
            return Err(IncompatiblePermutationSizes);
        }

        for value in self.raw.iter_mut() {
            let self_index: usize = value.clone().try_into().unwrap();
            let subsequent_value = subsequent_perm.get(self_index).unwrap();
            *value = subsequent_value.clone();
        }

        Ok(())
    }

    pub fn _nth_power(&mut self, n: usize) {
        let self_copy = self.clone();

        for _ in 1..n {
            self._compose_with(&self_copy).unwrap();
        }
    }

    pub fn _nth_power_of(&self, n: usize) -> Self {
        let mut self_copy = self.clone();

        for _ in 1..n {
            self_copy._compose_with(self).unwrap();
        }

        self_copy
    }

    pub fn _nth_power_mod(&mut self, mut n: usize) {
        let order = self._get_order();
        n %= order;
        self._nth_power(n);
    }

    pub fn _nth_power_of_mod(&self, mut n: usize) -> Self {
        let mut self_copy = self.clone();

        let order = self_copy._get_order();
        n %= order;

        for _ in 1..n {
            self_copy._compose_with(self).unwrap();
        }

        self_copy
    }

    pub fn len(&self) -> usize {
        self.raw.len()
    }

    /// Computes the order of the permutation. This is the same as the size
    /// of the subgroup generated by this permutation.
    /// The order is the least common multiple of all cycle lengths as each
    /// cycle has a period of its own length.
    pub fn _get_order(&mut self) -> usize {
        if self.cycles.is_none() {
            self.compute_cycles();
        }

        match &self.cycles {
            Some(cycles) => {
                let mut size = 1;

                for cycle in cycles.iter() {
                    size = size.lcm(&cycle.len());
                }

                size
            }
            None => unreachable!(),
        }
    }

    /// Evaluate the permutation for a single value.
    pub fn _evaluate(&self, in_value: &T) -> Option<T> {
        self.get(in_value.clone().try_into().unwrap()).map(T::clone)
    }

    /// Apply the permutation to the given iterator.
    pub fn _apply<I>(self, iterator: I) -> impl Iterator<Item = T>
    where
        I: Iterator<Item = T>,
    {
        iterator.map(move |value| self._evaluate(&value.clone()).unwrap_or(value))
    }

    pub fn get_cycles(&mut self) -> Vec<Vec<T>> {
        if self.cycles.is_none() {
            self.compute_cycles();
        }

        self.cycles.clone().unwrap()
    }

    fn get(&self, index: usize) -> Option<&T> {
        self.raw.get(index)
    }

    fn _from_cycle(cycle: &[T], raw: &mut [T]) {
        let first = cycle.get(0).unwrap().clone();
        let mut last = first.clone();

        for current in cycle[1..].iter() {
            *raw.get_mut(last.try_into().unwrap()).unwrap() = current.clone();
            last = current.clone();
        }
        *raw.get_mut(last.try_into().unwrap()).unwrap() = first;
    }

    fn normalize_cycle(cycle: &mut [T]) {
        let min_index = cycle
            .iter()
            .position_min_by_key(|&x| x.clone().try_into().unwrap())
            .unwrap();
        cycle.rotate_left(min_index);
    }

    fn get_cycle(&self, from: T) -> Vec<T> {
        let mut cycle = vec![from.clone()];

        let mut value = self.get(from.clone().try_into().unwrap()).unwrap();

        loop {
            if value != &from {
                cycle.push(value.clone());
                value = self.get(value.clone().try_into().unwrap()).unwrap();
            } else {
                break;
            }
        }

        Self::normalize_cycle(&mut cycle);
        cycle
    }

    fn compute_cycles(&mut self) {
        let mut cycles = Vec::new();

        for (index, value) in self.raw.iter().enumerate() {
            if index != value.clone().try_into().unwrap()
                && !cycles.iter().any(|cycle: &Vec<T>| cycle.contains(value))
            {
                cycles.push(self.get_cycle(value.clone()));
            }
        }

        self.cycles = Some(cycles);
    }
}

impl<T> From<Vec<T>> for Permutation<T>
where
    T: TryInto<usize> + Clone + PartialEq + Default,
    <T as TryInto<usize>>::Error: std::fmt::Debug,
{
    fn from(raw: Vec<T>) -> Self {
        Permutation::new(raw)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn compose_test() {
        let perm1 = vec![1usize, 2, 0].into();
        let perm2 = vec![2, 1, 0].into();
        let comp = Permutation::_compose(&perm1, &perm2).unwrap();
        assert_eq!(Permutation::new(vec![0usize, 2, 1]), comp);

        let perm3 = vec![0, 1, 2, 3].into();
        let comp_error = Permutation::_compose(&perm1, &perm3);
        assert!(comp_error.is_err());
    }

    #[test]
    fn compose_with_test() {
        let perm1 = vec![1usize, 2, 0].into();
        let mut perm2: Permutation<usize> = vec![2, 1, 0].into();
        assert!(perm2._compose_with(&perm1).is_ok());
        assert_eq!(Permutation::new(vec![0usize, 2, 1]), perm2);

        let perm3 = vec![0, 1, 2, 3].into();
        let comp_error = perm2._compose_with(&perm3);
        assert!(comp_error.is_err());
    }

    #[test]
    fn normalize_cycle_test() {
        let mut cycle = vec![3, 5, 4, 2];
        let normalized = vec![2, 3, 5, 4];
        Permutation::<usize>::normalize_cycle(&mut cycle);
        assert_eq!(normalized, cycle);
    }

    #[test]
    fn compute_cycles_test() {
        let raw = vec![0usize, 1, 3, 2];
        let mut perm = Permutation::new(raw.clone());
        perm.compute_cycles();
        assert_eq!(raw, perm.raw);
        assert_eq!(vec![vec![2usize, 3]], perm.cycles.unwrap());
    }

    #[test]
    fn get_subgroup_size() {
        let mut perm = Permutation::new_with_cycles(vec![4usize, 0, 1, 5, 7, 3, 2, 6]); //(0 4 7 6 2 1) (3 5)
        let subgroup_size = perm._get_order();
        assert_eq!(6, subgroup_size);

        let mut perm2 = Permutation::new(vec![1usize, 2, 0, 4, 3]); // (0 1 2) (3 4)
        let subgroup_size2 = perm2._get_order();
        assert_eq!(6, subgroup_size2);

        let mut perm3 = Permutation::new(vec![1usize, 2, 0, 4, 3, 8, 5, 6, 7]); // (0 1 2) (3 4) (5 8 7 6)
        let subgroup_size3 = perm3._get_order();
        assert_eq!(12, subgroup_size3);
    }

    #[test]
    fn evaluate_test() {
        let perm = Permutation::new(vec![0usize, 2, 1]);
        assert_eq!(0, perm._evaluate(&0).unwrap());
        assert_eq!(2, perm._evaluate(&1).unwrap());
        assert_eq!(1, perm._evaluate(&2).unwrap());
        assert!(perm._evaluate(&3).is_none());
    }

    #[test]
    fn apply_test() {
        let perm = Permutation::new(vec![4usize, 2, 1, 0, 3]);
        let data = vec![0usize, 2, 3, 4, 5, 1];
        let permuted_data: Vec<usize> = perm._apply(data.into_iter()).collect();
        assert_eq!(vec![4usize, 1, 0, 3, 5, 2], permuted_data);
    }

    #[test]
    fn from_cycles_test() {
        let cycles = vec![vec![1u8, 2, 3], vec![0], vec![5, 6], vec![4]];
        let perm = Permutation::_from_cycles(cycles, 7);
        let expected_perm = Permutation::new_with_cycles(vec![0, 2, 3, 1, 4, 6, 5]);
        assert_eq!(expected_perm, perm);
    }
}
