use itertools::Itertools;
use std::{
    fs::File,
    io::{BufWriter, Write},
};

use super::GAP_IN_FILE;
use crate::{permutation::Permutation, Error};

#[cfg(not(tarpaulin_include))]
fn write_permutation_gap(
    writer: &mut impl Write,
    permutation: &mut Permutation,
) -> Result<(), Error> {
    let cycles = permutation.get_cycles();
    if cycles.is_empty() {
        return write!(writer, "()").map_err(Error::from);
    }

    for cycle in cycles {
        write!(writer, "(")?;
        Itertools::intersperse(cycle.into_iter().map(|vertex| Some(vertex + 1)), None)
            .map(|element| {
                if let Some(i) = element {
                    write!(writer, "{}", i)
                } else {
                    write!(writer, ",")
                }
            })
            .try_collect()?;
        write!(writer, ")")?;
    }

    Ok(())
}

#[cfg(not(tarpaulin_include))]
pub fn write_gap_input(permutations: Vec<Permutation>) -> Result<(), Error> {
    let mut gap_in_file = BufWriter::new(File::create(GAP_IN_FILE)?);

    write!(gap_in_file, "g:=Group([")?;
    for mut permutation in permutations {
        write_permutation_gap(&mut gap_in_file, &mut permutation)?;
        writeln!(gap_in_file, ",")?;
    }
    writeln!(gap_in_file, "]);;")?;
    writeln!(
        gap_in_file,
        r#"
c:=ConjugacyClassesSubgroups(g);;
c_length:=Length(c);;
for i in [2..c_length] do
    Print(GeneratorsOfGroup(Representative(c[i])));
    Print("\n");
od;;"#
    )
    .map_err(Error::from)
}
