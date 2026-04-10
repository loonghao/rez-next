//! Satisfiability checking for version range BoundSets.

use super::types::{Bound, BoundSet};
use super::super::Version;

/// Check if a single BoundSet is satisfiable (not trivially empty due to conflicting bounds)
pub(super) fn is_bound_set_satisfiable(bs: &BoundSet) -> bool {
    // Check for None bounds
    if bs.bounds.iter().any(|b| matches!(b, Bound::None)) {
        return false;
    }
    // Extract lower and upper bounds to check for contradiction
    let mut lower: Option<(&Version, bool)> = None; // (version, inclusive)
    let mut upper: Option<(&Version, bool)> = None; // (version, inclusive)

    for bound in &bs.bounds {
        match bound {
            Bound::Any => {}
            Bound::None => return false,
            Bound::Ge(v) => match lower {
                None => lower = Some((v, true)),
                Some((lv, linc)) => {
                    if v > lv || (v == lv && !linc) {
                        lower = Some((v, true));
                    }
                }
            },
            Bound::Gt(v) => match lower {
                None => lower = Some((v, false)),
                Some((lv, _)) => {
                    if v >= lv {
                        lower = Some((v, false));
                    }
                }
            },
            Bound::Le(v) => match upper {
                None => upper = Some((v, true)),
                Some((uv, uinc)) => {
                    if v < uv || (v == uv && !uinc) {
                        upper = Some((v, true));
                    }
                }
            },
            Bound::Lt(v) => match upper {
                None => upper = Some((v, false)),
                Some((uv, _)) => {
                    if v <= uv {
                        upper = Some((v, false));
                    }
                }
            },
            Bound::Eq(v) => {
                // Equality constraint acts as both lower and upper bound
                match lower {
                    None => lower = Some((v, true)),
                    Some((lv, linc)) => {
                        if v > lv || (v == lv && !linc) {
                            lower = Some((v, true));
                        } else if v < lv {
                            // v must be >= lv but eq requires v exactly — contradiction
                            return false;
                        }
                    }
                }
                match upper {
                    None => upper = Some((v, true)),
                    Some((uv, uinc)) => {
                        if v < uv || (v == uv && !uinc) {
                            upper = Some((v, true));
                        } else if v > uv {
                            return false;
                        }
                    }
                }
            }
            Bound::Ne(_) | Bound::Compatible(_) => {}
        }
    }

    // Check if lower and upper bounds are compatible
    if let (Some((lv, linc)), Some((uv, uinc))) = (lower, upper) {
        match lv.cmp(uv) {
            std::cmp::Ordering::Greater => return false,
            std::cmp::Ordering::Equal => {
                if !linc || !uinc {
                    return false;
                }
            }
            std::cmp::Ordering::Less => {}
        }
    }

    true
}

/// Check if two BoundSets can simultaneously be satisfied (have intersection)
pub(super) fn bound_sets_intersect(a: &BoundSet, b: &BoundSet) -> bool {
    // Quick check: if either is Any, they intersect
    let a_any = a.bounds.iter().all(|bnd| matches!(bnd, Bound::Any));
    let b_any = b.bounds.iter().all(|bnd| matches!(bnd, Bound::Any));
    if a_any || b_any {
        return true;
    }

    // Combine all bounds and check for structural impossibilities
    let combined_bounds: Vec<&Bound> = a.bounds.iter().chain(b.bounds.iter()).collect();

    // Check for obvious impossibilities: Eq(v) and Eq(w) where v != w
    let eq_versions: Vec<&Version> = combined_bounds
        .iter()
        .filter_map(|b| if let Bound::Eq(v) = b { Some(v) } else { None })
        .collect();
    if eq_versions.len() > 1 {
        let first = eq_versions[0];
        if eq_versions.iter().any(|v| *v != first) {
            return false;
        }
    }

    // Check Eq(v) against inequality bounds: Eq(v) ∩ Ge(w) = ∅ when v < w,
    // Eq(v) ∩ Gt(w) = ∅ when v <= w, Eq(v) ∩ Le(w) = ∅ when v > w,
    // Eq(v) ∩ Lt(w) = ∅ when v >= w.
    for eq_v in &eq_versions {
        for bound in &combined_bounds {
            match bound {
                Bound::Ge(w) if *eq_v < w => return false,
                Bound::Gt(w) if *eq_v <= w => return false,
                Bound::Le(w) if *eq_v > w => return false,
                Bound::Lt(w) if *eq_v >= w => return false,
                _ => {}
            }
        }
    }

    // Compute effective lower and upper bounds from combined set
    // Lower bound: maximum of all lower bounds (most restrictive)
    // Upper bound: minimum of all upper bounds (most restrictive)
    let mut lower: Option<(&Version, bool)> = None; // (version, inclusive)
    let mut upper: Option<(&Version, bool)> = None; // (version, inclusive)

    for bound in &combined_bounds {
        match bound {
            Bound::Ge(v) => match lower {
                None => lower = Some((v, true)),
                Some((lv, linc)) => {
                    if v > lv || (v == lv && !linc) {
                        lower = Some((v, true));
                    }
                }
            },
            Bound::Gt(v) => match lower {
                None => lower = Some((v, false)),
                Some((lv, _linc)) => {
                    if v >= lv {
                        lower = Some((v, false));
                    }
                }
            },
            Bound::Le(v) => match upper {
                None => upper = Some((v, true)),
                Some((uv, uinc)) => {
                    if v < uv || (v == uv && !uinc) {
                        upper = Some((v, true));
                    }
                }
            },
            Bound::Lt(v) => match upper {
                None => upper = Some((v, false)),
                Some((uv, _uinc)) => {
                    if v <= uv {
                        upper = Some((v, false));
                    }
                }
            },
            _ => {}
        }
    }

    // Check if lower bound and upper bound are compatible
    if let (Some((lv, linc)), Some((uv, uinc))) = (lower, upper) {
        match lv.cmp(uv) {
            std::cmp::Ordering::Greater => return false,
            std::cmp::Ordering::Equal => {
                // Equal bounds only feasible if both inclusive
                if !linc || !uinc {
                    return false;
                }
            }
            std::cmp::Ordering::Less => {} // feasible
        }
    }

    true
}
