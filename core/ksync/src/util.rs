//! Utility types and functions for synchronization primitives.

use axtask::yield_now;

/// Spin configuration for blocking synchronization primitives.
#[derive(Debug, Clone, Copy)]
pub struct SpinConfig {
    /// Maximum number of spin iterations before blocking
    pub max_spins: u32,
    /// Number of spins before yielding
    pub spin_before_yield: u32,
}

impl Default for SpinConfig {
    fn default() -> Self {
        Self {
            max_spins: 10,
            spin_before_yield: 3,
        }
    }
}

/// Helper for adaptive spinning with configurable strategy.
pub(crate) struct Spin {
    count: u32,
    config: SpinConfig,
}

impl Spin {
    #[inline]
    pub(crate) fn new(config: SpinConfig) -> Self {
        Self { count: 0, config }
    }

    /// Perform one spin iteration.
    /// Returns `true` if more spins should be attempted, `false` if should block.
    #[inline]
    pub(crate) fn spin(&mut self) -> bool {
        if self.count >= self.config.max_spins {
            return false;
        }
        self.count += 1;
        if self.count <= self.config.spin_before_yield {
            for _ in 0..(1 << self.count) {
                core::hint::spin_loop();
            }
        } else {
            yield_now();
        }
        true
    }
}
