// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Shared A/B/B/A measurement harness for request-encoding benchmarks.

use std::{fmt::Debug, hint::black_box, time::Instant};

pub fn elapsed(iterations: usize, operation: &mut impl FnMut()) -> f64 {
    let start = Instant::now();
    for _ in 0..iterations {
        operation();
    }
    start.elapsed().as_nanos() as f64 / iterations as f64
}

pub fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    (values[(values.len() - 1) / 2] + values[values.len() / 2]) / 2.
}

/// One benchmark family lives in its own bench binary: unrelated measured
/// code shifts monomorphization and codegen-unit placement enough to move
/// other cases by double-digit percentages, which the drift gate cannot see.
pub fn compare<T: PartialEq + Debug>(
    name: &str,
    iterations: usize,
    mut old: impl FnMut() -> T,
    mut new: impl FnMut() -> T,
) {
    assert_eq!(old(), new(), "wire mismatch: {name}");
    for _ in 0..100 {
        black_box(old());
        black_box(new());
    }
    let mut a1 = Vec::new();
    let mut a2 = Vec::new();
    let mut b = Vec::new();
    for _ in 0..9 {
        a1.push(elapsed(iterations, &mut || {
            black_box(old());
        }));
        b.push(elapsed(iterations, &mut || {
            black_box(new());
        }));
        b.push(elapsed(iterations, &mut || {
            black_box(new());
        }));
        a2.push(elapsed(iterations, &mut || {
            black_box(old());
        }));
    }
    let drift = (median(&mut a2) / median(&mut a1) - 1.) * 100.;
    let mut a = a1;
    a.extend(a2);
    let old = median(&mut a);
    let new = median(&mut b);
    println!(
        "{name}: baseline {old:.0} ns/op, current {new:.0} ns/op, {:.1}% less time, baseline drift {drift:+.1}%",
        (1. - new / old) * 100.
    );
    assert!(
        drift.abs() <= 10.,
        "baseline drift exceeds 10%; rerun before interpreting {name}"
    );
}
