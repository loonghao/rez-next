//! SolverCallbackReturn enum for solver callback return values.
//!
//! This module provides the `SolverCallbackReturn` enum that is returned by
//! the callback callable passed to a `Solver` instance.

/// Enum returned by the `callback` callable passed to a `Solver` instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolverCallbackReturn {
    /// Continue the solve
    KeepGoing,
    /// Abort the solve
    Abort,
    /// Stop the solve and set to most recent failure
    Fail,
}

impl SolverCallbackReturn {
    /// Get the description of the callback return value.
    ///
    /// # Returns
    ///
    /// A string describing the callback return value.
    pub fn description(&self) -> &'static str {
        match self {
            SolverCallbackReturn::KeepGoing => "Continue the solve",
            SolverCallbackReturn::Abort => "Abort the solve",
            SolverCallbackReturn::Fail => "Stop the solve and set to most recent failure",
        }
    }
}

impl std::fmt::Display for SolverCallbackReturn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.description())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solver_callback_return_keep_going() {
        let value = SolverCallbackReturn::KeepGoing;
        assert_eq!(value.description(), "Continue the solve");
        assert_eq!(format!("{}", value), "Continue the solve");
    }

    #[test]
    fn test_solver_callback_return_abort() {
        let value = SolverCallbackReturn::Abort;
        assert_eq!(value.description(), "Abort the solve");
        assert_eq!(format!("{}", value), "Abort the solve");
    }

    #[test]
    fn test_solver_callback_return_fail() {
        let value = SolverCallbackReturn::Fail;
        assert_eq!(value.description(), "Stop the solve and set to most recent failure");
        assert_eq!(
            format!("{}", value),
            "Stop the solve and set to most recent failure"
        );
    }

    #[test]
    fn test_solver_callback_return_partial_eq() {
        assert_eq!(
            SolverCallbackReturn::KeepGoing,
            SolverCallbackReturn::KeepGoing
        );
        assert_ne!(SolverCallbackReturn::KeepGoing, SolverCallbackReturn::Abort);
        assert_ne!(SolverCallbackReturn::KeepGoing, SolverCallbackReturn::Fail);
    }

    #[test]
    fn test_solver_callback_return_clone() {
        let value = SolverCallbackReturn::KeepGoing;
        let cloned = value;
        assert_eq!(value, cloned);
    }
}
