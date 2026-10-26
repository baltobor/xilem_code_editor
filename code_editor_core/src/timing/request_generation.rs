// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

/// Cancellation by generation: each new request bumps the generation, and
/// a response is applied only if it carries the current one. This is how
/// superseded provider results are dropped (zed drops the task instead).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RequestGeneration(u64);

impl RequestGeneration {
    /// Start a new request; returns its generation.
    pub fn next(&mut self) -> u64 {
        self.0 += 1;
        self.0
    }

    /// Invalidate any in-flight request without starting a new one.
    pub fn cancel(&mut self) {
        self.0 += 1;
    }

    pub fn is_current(&self, generation: u64) -> bool {
        self.0 == generation
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_latest_is_current() {
        let mut g = RequestGeneration::default();
        let a = g.next();
        let b = g.next();
        assert!(!g.is_current(a));
        assert!(g.is_current(b));
        g.cancel();
        assert!(!g.is_current(b));
    }
}
