//! Simple combinatorial helper functions
//! that allow to search the powerset of
//! generators for some tha induce descriptive quotients.

fn is_active(n: usize, index: usize) -> bool {
    (n & (1 << index)) > 0
}

pub fn iterate_powerset<T, F>(set: &Vec<T>, f: F)
where
    T: Clone,
    F: Fn(&mut Vec<T>),
{
    let number_of_elements = set.len();

    // I don't really care about more than 64 generators for now.
    // Change after 1.53.0 to usize::BITS (currently unstable after regressions)
    if number_of_elements > 64 {
        unimplemented!()
    }

    let mut subset = Vec::with_capacity(number_of_elements);

    // If `elements_numbers` would be bigger than 64 we would run into trouble here:
    for counter in 1..(2usize.pow(number_of_elements as u32)) {
        for element_index in 0..number_of_elements {
            if is_active(counter, element_index) {
                subset.push(set[element_index].clone());
            }
        }

        f(&mut subset);
        subset.clear();
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_is_active() {
        let x = 0b01;
        assert!(is_active(x, 0));
        assert!(!is_active(x, 1));

        let y = 0b1010101;
        assert!(is_active(y, 0));
        assert!(!is_active(y, 1));
        assert!(is_active(y, 2));
        assert!(!is_active(y, 3));
        assert!(is_active(y, 4));
        assert!(!is_active(y, 5));
        assert!(is_active(y, 6));
    }

    #[test]
    fn test_iterate() {
        let set: Vec<i32> = vec![1, 2];
        let f = |xs: &mut Vec<i32>| {
            println!("{:?}", xs);
            for x in xs[..].iter() {
                assert!(*x > 0);
            }
        };

        iterate_powerset(&set, f);
    }
}
