//! Log evaluation mode that allows to
//! evaluate the logs of the quotientPlanning
//! tool run as experiments.

use std::{
    io::{BufRead, Lines},
    iter::Peekable,
    str::FromStr,
};

use crate::{
    parser::{Input, ParseError},
    MetricUsed,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanResult {
    ValidPlan(usize),
    NotSolved,
}

#[derive(Debug, PartialEq, Eq)]
pub enum QuotientResult {
    QuotientConcretePlans(PlanResult, PlanResult),
    NoSymmetry,
    Nondescriptive,
    TimedOut,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Log {
    metric: MetricUsed,
    default_result: PlanResult,
    quotient_result: QuotientResult,
}

fn evaluate_plan_result<'a>(line: &'a str) -> Option<PlanResult> {
    use nom::{
        branch::alt, bytes::complete::tag, character::complete::digit1, combinator::map,
        sequence::preceded,
    };
    let valid_tag =
        tag::<Input<'a>, Input<'a>, ParseError<'a>>("Plan is valid and it is of length ");
    let valid_parser = map(preceded(valid_tag, digit1), |length: &str| {
        PlanResult::ValidPlan(length.parse().unwrap())
    });
    let not_solved_parser = map(
        tag("The problem was not solved! Plan can't be valid!"),
        |_| PlanResult::NotSolved,
    );
    alt((valid_parser, not_solved_parser))(line)
        .ok()
        .map(|(_, plan_result)| plan_result)
}

fn evaluate_log<B: BufRead>(peekable: &mut Peekable<&mut Lines<B>>) -> Option<Log> {
    let metric = peekable.find_map(|line| {
        line.unwrap()
            .strip_suffix(':')
            .map(|line| MetricUsed::from_str(line).ok())
            .flatten()
    })?;
    let default_result = peekable.find_map(|line| evaluate_plan_result(line.unwrap().as_str()))?;

    let mut quotient_result = QuotientResult::TimedOut;
    let mut quotient_next = false;

    loop {
        if peekable
            .next_if(|line| line.as_ref().unwrap() == "No symmetries found, exiting!!")
            .is_some()
        {
            quotient_result = QuotientResult::NoSymmetry;
            break;
        } else if peekable
            .next_if(|line| line.as_ref().unwrap() == "No covering instantiations, exiting!!")
            .is_some()
        {
            quotient_result = QuotientResult::Nondescriptive;
            break;
        } else if peekable
            .next_if(|line| line.as_ref().unwrap() == "Quotient problem plan:")
            .is_some()
        {
            quotient_result =
                QuotientResult::QuotientConcretePlans(PlanResult::NotSolved, PlanResult::NotSolved);
            quotient_next = true;
        } else if peekable
            .next_if(|line| line.as_ref().unwrap() == "Concrete problem plan:")
            .is_some()
        {
            quotient_next = false;
        } else if peekable
            .peek()?
            .as_ref()
            .unwrap()
            .strip_suffix(':')
            .map(|line| MetricUsed::from_str(line).ok())
            .flatten()
            .is_some()
        {
            quotient_result = QuotientResult::TimedOut;
            break;
        } else if let Some(plan_result) =
            evaluate_plan_result(peekable.peek().unwrap().as_ref().unwrap().as_str())
        {
            if quotient_next {
                quotient_result = QuotientResult::QuotientConcretePlans(
                    plan_result.clone(),
                    PlanResult::NotSolved,
                );
                peekable.next();
                if matches!(plan_result, PlanResult::NotSolved) {
                    break;
                } else {
                    continue;
                }
            } else if let QuotientResult::QuotientConcretePlans(quotient, _) = quotient_result {
                quotient_result = QuotientResult::QuotientConcretePlans(quotient, plan_result);
                peekable.next();
                break;
            }
            unreachable!();
        } else {
            peekable.next();
        }
    }

    Some(Log {
        metric,
        default_result,
        quotient_result,
    })
}

#[cfg(not(tarpaulin_include))]
pub fn evaluate_log_file<B: BufRead>(file_as_lines: &mut Lines<B>) -> Vec<Log> {
    let mut logs = Vec::new();
    let mut peekable = file_as_lines.peekable();

    while let Some(log) = evaluate_log(&mut peekable) {
        logs.push(log);
    }

    logs
}

#[cfg(test)]
mod test {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn test_evaluate_plan_result() {
        let plan_result1 = "Plan is valid and it is of length 36";
        assert_eq!(
            Some(PlanResult::ValidPlan(36)),
            evaluate_plan_result(plan_result1),
        );

        let plan_result2 = "The problem was not solved! Plan can't be valid!";
        assert_eq!(
            Some(PlanResult::NotSolved),
            evaluate_plan_result(plan_result2),
        );

        let plan_result3 = "Covering instantiation set size is: 2";
        assert_eq!(None, evaluate_plan_result(plan_result3));
    }

    #[test]
    fn test_evaluate_log_nondescriptive() {
        let raw = "standard:
The causal graph is not acyclic.
51 variables of 51 necessary
Plan is valid and it is of length 36
Initial size of primary cover 9.
Initial size of secondary cover 432.
Number of variable orbits is 0
Number of actions = 3114
Number of variables = 432
Printing action edges
Printing variable edges (only to variables)
Printing initial state edges
Printing goal edges
Number of action orbits = 918
Number of variable orbits = 179
Number of actions in the problem = 3114
Number of action orbits added to the quotient problem = 918
Number of var in the problem initial state = 51
Number of var orbits added to the quotient problem initial state = 31
Number of var in the problem goal state = 9
Number of var orbits added to the quotient problem goal state = 3
Primary cover size: 9
No covering instantiations, exiting!!";
        let mut lines = Cursor::new(raw).lines();
        let mut peekable = (&mut lines).peekable();
        let log = evaluate_log(&mut peekable);
        let expected_log = Some(Log {
            metric: MetricUsed::Standard,
            default_result: PlanResult::ValidPlan(36),
            quotient_result: QuotientResult::Nondescriptive,
        });
        assert_eq!(expected_log, log);
    }

    #[test]
    fn test_evaluate_log_nosymmetry() {
        let raw = "biggest_orbit:
The causal graph is not acyclic.

23 variables of 23 necessary

Current action is 34
Current action is 77
Current action is 88
Current action is 93
Current action is 55
Plan is valid and it is of length 5
Initial size of primary cover 2.
Initial size of secondary cover 66.
Number of variable orbits is 0
Number of actions = 104
Number of variables = 66

Printing action edges

Printing variable edges (only to variables)

Printing initial state edges

Printing goal edges

Number of action orbits = 104
Number of variable orbits = 66
Number of actions in the problem = 104
Number of action orbits added to the quotient problem = 104
Number of var in the problem initial state = 23
Number of var orbits added to the quotient problem initial state = 23
Number of var in the problem goal state = 2
Number of var orbits added to the quotient problem goal state = 2
No symmetries found, exiting!!";
        let mut lines = Cursor::new(raw).lines();
        let mut peekable = (&mut lines).peekable();
        let log = evaluate_log(&mut peekable);
        let expected_log = Some(Log {
            metric: MetricUsed::BiggestOrbits,
            default_result: PlanResult::ValidPlan(5),
            quotient_result: QuotientResult::NoSymmetry,
        });
        assert_eq!(expected_log, log);
    }

    #[test]
    fn test_evaluate_log_quotient_notsolved() {
        let raw = "least_orbits:
The causal graph is not acyclic.
408 variables of 408 necessary
Plan is valid and it is of length 194
Initial size of primary cover 15.
Initial size of secondary cover 849.
Number of variable orbits is 0
Number of actions = 2814
Number of variables = 849
Printing action edges
Printing variable edges (only to variables)
Printing initial state edges
Printing goal edges
Number of action orbits = 2474
Number of variable orbits = 749
Number of actions in the problem = 2814
Number of action orbits added to the quotient problem = 2474
Number of var in the problem initial state = 408
Number of var orbits added to the quotient problem initial state = 360
Number of var in the problem goal state = 15
Number of var orbits added to the quotient problem goal state = 13
Primary cover size: 15
Primary cover size: 2
Primary cover size: 1
Covering instantiation set size is: 3
The number of common resources is 749 and the size of common resource orbits is 749
Adding orbit 0 to the quotient goal
Adding orbit 720 to the quotient goal
Quotient problem plan:
The problem was not solved! Plan can't be valid!
Concrete problem plan:
Plan is valid and it is of length 36";
        let mut lines = Cursor::new(raw).lines();
        let mut peekable = (&mut lines).peekable();
        let log = evaluate_log(&mut peekable);
        let expected_log = Some(Log {
            metric: MetricUsed::LeastOrbits,
            default_result: PlanResult::ValidPlan(194),
            quotient_result: QuotientResult::QuotientConcretePlans(
                PlanResult::NotSolved,
                PlanResult::NotSolved,
            ),
        });
        assert_eq!(expected_log, log);
    }

    #[test]
    fn test_evaluate_log_concrete_notsolved() {
        let raw = "least_orbits:
The causal graph is not acyclic.
408 variables of 408 necessary
The problem was not solved! Plan can't be valid!
Initial size of primary cover 15.
Initial size of secondary cover 849.
Number of variable orbits is 0
Number of actions = 2814
Number of variables = 849
Printing action edges
Printing variable edges (only to variables)
Printing initial state edges
Printing goal edges
Number of action orbits = 2474
Number of variable orbits = 749
Number of actions in the problem = 2814
Number of action orbits added to the quotient problem = 2474
Number of var in the problem initial state = 408
Number of var orbits added to the quotient problem initial state = 360
Number of var in the problem goal state = 15
Number of var orbits added to the quotient problem goal state = 13
Primary cover size: 15
Primary cover size: 2
Primary cover size: 1
Covering instantiation set size is: 3
The number of common resources is 749 and the size of common resource orbits is 749
Adding orbit 0 to the quotient goal
Adding orbit 720 to the quotient goal
Quotient problem plan:
Plan is valid and it is of length 36
Concrete problem plan:
The problem was not solved! Plan can't be valid!";
        let mut lines = Cursor::new(raw).lines();
        let mut peekable = (&mut lines).peekable();
        let log = evaluate_log(&mut peekable);
        let expected_log = Some(Log {
            metric: MetricUsed::LeastOrbits,
            default_result: PlanResult::NotSolved,
            quotient_result: QuotientResult::QuotientConcretePlans(
                PlanResult::ValidPlan(36),
                PlanResult::NotSolved,
            ),
        });
        assert_eq!(expected_log, log);
    }

    #[test]
    fn test_evaluate_log_valid() {
        let raw = "sparsity:
The causal graph is not acyclic.
51 variables of 51 necessary
Plan is valid and it is of length 36
Initial size of primary cover 9.
Initial size of secondary cover 432.
Number of variable orbits is 0
Number of actions = 3114
Number of variables = 432
Printing action edges
Printing variable edges (only to variables)
Printing initial state edges
Printing goal edges
Number of action orbits = 2772
Number of variable orbits = 404
Number of actions in the problem = 3114
Number of action orbits added to the quotient problem = 2772
Number of var in the problem initial state = 51
Number of var orbits added to the quotient problem initial state = 49
Number of var in the problem goal state = 9
Number of var orbits added to the quotient problem goal state = 9
Primary cover size: 9
Covering instantiation set size is: 1
The number of common resources is 0 and the size of common resource orbits is 0
Common resources are Common resource orbits before removing non-preconditions are 
Common resource orbits after removing non-preconditions are 

Quotient problem plan:
Plan is valid and it is of length 48000
Concrete problem plan:
Current action is 1527
Current action is 2502
Current action is 993
Current action is 2667
Current action is 1170
Current action is 2832
Current action is 1344
Current action is 2557
Current action is 2034
Current action is 495
Current action is 1047
Current action is 2612
Current action is 2088
Current action is 1101
Current action is 2994
Current action is 554
Current action is 2700
Current action is 1492
Current action is 1162
Current action is 1528
Current action is 2722
Current action is 1492
Current action is 1220
Current action is 1528
Current action is 2777
Current action is 1278
Current action is 2874
Current action is 1493
Current action is 1336
Current action is 1529
Current action is 2887
Current action is 1493
Current action is 1400
Current action is 1529
Current action is 2942
Current action is 1452
Plan is valid and it is of length 12";
        let mut lines = Cursor::new(raw).lines();
        let mut peekable = (&mut lines).peekable();
        let log = evaluate_log(&mut peekable);
        let expected_log = Some(Log {
            metric: MetricUsed::Sparsity,
            default_result: PlanResult::ValidPlan(36),
            quotient_result: QuotientResult::QuotientConcretePlans(
                PlanResult::ValidPlan(48000),
                PlanResult::ValidPlan(12),
            ),
        });
        assert_eq!(expected_log, log);
    }

    #[test]
    fn test_evaluate_log_concrete_timeout() {
        let raw = "least_orbits:
The causal graph is not acyclic.
408 variables of 408 necessary
The problem was not solved! Plan can't be valid!
Initial size of primary cover 15.
Initial size of secondary cover 849.
Number of variable orbits is 0
Number of actions = 2814
Number of variables = 849
Printing action edges
Printing variable edges (only to variables)
Printing initial state edges
Printing goal edges
Number of action orbits = 2474
Number of variable orbits = 749
Number of actions in the problem = 2814
Number of action orbits added to the quotient problem = 2474
Number of var in the problem initial state = 408
Number of var orbits added to the quotient problem initial state = 360
Number of var in the problem goal state = 15
Number of var orbits added to the quotient problem goal state = 13
Primary cover size: 15
Primary cover size: 2
Primary cover size: 1
Covering instantiation set size is: 3
The number of common resources is 749 and the size of common resource orbits is 749
Adding orbit 0 to the quotient goal
Adding orbit 720 to the quotient goal
Quotient problem plan:
Plan is valid and it is of length 36

sparsity:";
        let mut lines = Cursor::new(raw).lines();
        let mut peekable = (&mut lines).peekable();
        let log = evaluate_log(&mut peekable);
        let expected_log = Some(Log {
            metric: MetricUsed::LeastOrbits,
            default_result: PlanResult::NotSolved,
            quotient_result: QuotientResult::TimedOut,
        });
        assert_eq!(expected_log, log);
    }
}
