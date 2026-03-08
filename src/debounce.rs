//! Debounce and throttle timing primitives.
//!
//! Pure Rust — no nvim-oxi dependency. These helpers are useful for limiting
//! how often a callback fires in response to rapid events (typing, cursor
//! movement, window resize, etc.).
//!
//! # Examples
//!
//! ```
//! use std::time::Duration;
//! use ishizue::debounce::Debounce;
//!
//! let debounce = Debounce::new(Duration::from_millis(100));
//! // First call always passes through.
//! assert!(debounce.should_fire());
//! ```

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Debounce gate — delays action until a quiet period has elapsed.
///
/// Call [`Debounce::should_fire`] each time an event arrives. It returns
/// `true` only when at least `delay` has passed since the last call.
/// This implements a **trailing-edge** debounce: the first call after a
/// quiet period fires immediately.
#[derive(Debug)]
pub struct Debounce {
    delay: Duration,
    last: Mutex<Option<Instant>>,
}

impl Debounce {
    /// Create a new debounce gate with the given delay.
    #[must_use]
    pub fn new(delay: Duration) -> Self {
        Self {
            delay,
            last: Mutex::new(None),
        }
    }

    /// Returns `true` if enough time has elapsed since the last accepted call.
    ///
    /// When `true` is returned the internal timer is reset, so the next call
    /// must again wait for `delay` to elapse.
    pub fn should_fire(&self) -> bool {
        let mut last = self.last.lock().expect("debounce lock poisoned");
        let now = Instant::now();

        match *last {
            None => {
                *last = Some(now);
                true
            }
            Some(prev) if now.duration_since(prev) >= self.delay => {
                *last = Some(now);
                true
            }
            Some(_) => false,
        }
    }

    /// Reset the debounce timer so the next call to [`Debounce::should_fire`]
    /// will return `true`.
    pub fn reset(&self) {
        let mut last = self.last.lock().expect("debounce lock poisoned");
        *last = None;
    }

    /// Return the configured delay duration.
    #[must_use]
    pub fn delay(&self) -> Duration {
        self.delay
    }
}

/// Throttle gate — limits execution to at most once per `interval`.
///
/// Unlike [`Debounce`], throttle fires on the **leading edge**: the very
/// first call fires immediately, and subsequent calls within `interval`
/// are suppressed.
#[derive(Debug)]
pub struct Throttle {
    interval: Duration,
    last_fired: Mutex<Option<Instant>>,
}

impl Throttle {
    /// Create a new throttle with the given minimum interval between fires.
    #[must_use]
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            last_fired: Mutex::new(None),
        }
    }

    /// Returns `true` if the action should fire now.
    ///
    /// Fires immediately on the first call, then suppresses subsequent calls
    /// until `interval` has elapsed since the last fire.
    pub fn should_fire(&self) -> bool {
        let mut last = self.last_fired.lock().expect("throttle lock poisoned");
        let now = Instant::now();

        match *last {
            None => {
                *last = Some(now);
                true
            }
            Some(prev) if now.duration_since(prev) >= self.interval => {
                *last = Some(now);
                true
            }
            Some(_) => false,
        }
    }

    /// Reset the throttle so the next call fires immediately.
    pub fn reset(&self) {
        let mut last = self.last_fired.lock().expect("throttle lock poisoned");
        *last = None;
    }

    /// Return the configured interval.
    #[must_use]
    pub fn interval(&self) -> Duration {
        self.interval
    }
}

/// A value that is initialized exactly once (like `std::sync::OnceLock` but
/// with a simpler builder-style API and `Clone`-free retrieval via reference).
///
/// This is a thin wrapper around [`std::sync::OnceLock`] provided for API
/// consistency within ishizue.
#[derive(Debug)]
pub struct OnceCell<T> {
    inner: std::sync::OnceLock<T>,
}

impl<T> OnceCell<T> {
    /// Create a new, empty once-cell.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            inner: std::sync::OnceLock::new(),
        }
    }

    /// Get the value if it has been initialized.
    #[must_use]
    pub fn get(&self) -> Option<&T> {
        self.inner.get()
    }

    /// Initialize the cell with `value`. Returns `Ok(&value)` on success or
    /// `Err(value)` if the cell was already initialized.
    pub fn set(&self, value: T) -> Result<&T, T> {
        self.inner.set(value)?;
        // Safety: we just set it, so `get()` will succeed.
        Ok(self.inner.get().unwrap())
    }

    /// Get the value, initializing it with `f` if empty.
    pub fn get_or_init(&self, f: impl FnOnce() -> T) -> &T {
        self.inner.get_or_init(f)
    }

    /// Returns `true` if the cell has been initialized.
    #[must_use]
    pub fn is_initialized(&self) -> bool {
        self.inner.get().is_some()
    }
}

impl<T> Default for OnceCell<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// A simple flag that can be set exactly once. Useful for one-shot
/// initialization guards.
#[derive(Debug)]
pub struct OnceFlag {
    fired: AtomicBool,
}

impl OnceFlag {
    /// Create a new, unfired flag.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            fired: AtomicBool::new(false),
        }
    }

    /// Try to fire the flag. Returns `true` the first time, `false` on all
    /// subsequent calls.
    pub fn fire(&self) -> bool {
        !self.fired.swap(true, Ordering::AcqRel)
    }

    /// Returns `true` if the flag has been fired.
    #[must_use]
    pub fn has_fired(&self) -> bool {
        self.fired.load(Ordering::Acquire)
    }

    /// Reset the flag so it can fire again.
    pub fn reset(&self) {
        self.fired.store(false, Ordering::Release);
    }
}

impl Default for OnceFlag {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    // -- Debounce -----------------------------------------------------------

    #[test]
    fn debounce_first_call_fires() {
        let d = Debounce::new(Duration::from_millis(50));
        assert!(d.should_fire());
    }

    #[test]
    fn debounce_rapid_calls_suppressed() {
        let d = Debounce::new(Duration::from_millis(200));
        assert!(d.should_fire()); // first fires
        assert!(!d.should_fire()); // immediate second suppressed
        assert!(!d.should_fire()); // third suppressed
    }

    #[test]
    fn debounce_fires_after_delay() {
        let d = Debounce::new(Duration::from_millis(20));
        assert!(d.should_fire());
        thread::sleep(Duration::from_millis(30));
        assert!(d.should_fire()); // enough time passed
    }

    #[test]
    fn debounce_reset_allows_immediate_fire() {
        let d = Debounce::new(Duration::from_millis(200));
        assert!(d.should_fire());
        assert!(!d.should_fire());
        d.reset();
        assert!(d.should_fire()); // reset allowed it
    }

    #[test]
    fn debounce_delay_accessor() {
        let d = Debounce::new(Duration::from_secs(5));
        assert_eq!(d.delay(), Duration::from_secs(5));
    }

    // -- Throttle -----------------------------------------------------------

    #[test]
    fn throttle_first_call_fires() {
        let t = Throttle::new(Duration::from_millis(50));
        assert!(t.should_fire());
    }

    #[test]
    fn throttle_rapid_calls_suppressed() {
        let t = Throttle::new(Duration::from_millis(200));
        assert!(t.should_fire());
        assert!(!t.should_fire());
        assert!(!t.should_fire());
    }

    #[test]
    fn throttle_fires_after_interval() {
        let t = Throttle::new(Duration::from_millis(20));
        assert!(t.should_fire());
        thread::sleep(Duration::from_millis(30));
        assert!(t.should_fire());
    }

    #[test]
    fn throttle_reset_allows_immediate_fire() {
        let t = Throttle::new(Duration::from_millis(200));
        assert!(t.should_fire());
        assert!(!t.should_fire());
        t.reset();
        assert!(t.should_fire());
    }

    #[test]
    fn throttle_interval_accessor() {
        let t = Throttle::new(Duration::from_secs(3));
        assert_eq!(t.interval(), Duration::from_secs(3));
    }

    // -- OnceCell -----------------------------------------------------------

    #[test]
    fn once_cell_empty_initially() {
        let cell: OnceCell<i32> = OnceCell::new();
        assert!(cell.get().is_none());
        assert!(!cell.is_initialized());
    }

    #[test]
    fn once_cell_set_and_get() {
        let cell = OnceCell::new();
        let val = cell.set(42).expect("first set should succeed");
        assert_eq!(*val, 42);
        assert!(cell.is_initialized());
        assert_eq!(cell.get(), Some(&42));
    }

    #[test]
    fn once_cell_double_set_fails() {
        let cell = OnceCell::new();
        cell.set(1).unwrap();
        let result = cell.set(2);
        assert!(result.is_err());
        assert_eq!(cell.get(), Some(&1)); // original value preserved
    }

    #[test]
    fn once_cell_get_or_init() {
        let cell = OnceCell::new();
        let val = cell.get_or_init(|| 99);
        assert_eq!(*val, 99);
        // Second call doesn't re-init
        let val2 = cell.get_or_init(|| 100);
        assert_eq!(*val2, 99);
    }

    #[test]
    fn once_cell_default() {
        let cell: OnceCell<String> = OnceCell::default();
        assert!(cell.get().is_none());
    }

    // -- OnceFlag -----------------------------------------------------------

    #[test]
    fn once_flag_fires_once() {
        let flag = OnceFlag::new();
        assert!(!flag.has_fired());
        assert!(flag.fire()); // first time
        assert!(flag.has_fired());
        assert!(!flag.fire()); // second time
        assert!(!flag.fire()); // third time
    }

    #[test]
    fn once_flag_reset() {
        let flag = OnceFlag::new();
        assert!(flag.fire());
        flag.reset();
        assert!(!flag.has_fired());
        assert!(flag.fire()); // fires again after reset
    }

    #[test]
    fn once_flag_default() {
        let flag = OnceFlag::default();
        assert!(!flag.has_fired());
    }
}
