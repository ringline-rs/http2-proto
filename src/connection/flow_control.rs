//! HTTP/2 flow control.

/// Flow control state for connection or stream level.
///
/// HTTP/2 uses a credit-based flow control scheme. Each side maintains
/// a send window that limits how much data can be in flight.
#[derive(Debug)]
pub struct FlowControl {
    /// Current window size.
    window: i32,
    /// Initial window size (for calculating updates).
    initial_window: u32,
    /// Bytes consumed since last window update.
    consumed: u32,
    /// Threshold for sending window updates (fraction of initial window).
    update_threshold: u32,
}

impl FlowControl {
    /// Create new flow control state.
    pub fn new(initial_window_size: u32) -> Self {
        Self {
            window: initial_window_size as i32,
            initial_window: initial_window_size,
            consumed: 0,
            // Send update when we've consumed half the window
            update_threshold: initial_window_size / 2,
        }
    }

    /// Get current available window.
    pub fn available(&self) -> i32 {
        self.window
    }

    /// Increase the window (from WINDOW_UPDATE).
    /// Increase the window by a WINDOW_UPDATE increment.
    ///
    /// Returns `false` if the increment would take the window above
    /// `MAX_WINDOW_SIZE` (2^31 - 1), which RFC 7540 section 6.9.1 requires to
    /// be a `FLOW_CONTROL_ERROR`. The window is left unchanged so the caller
    /// can signal it. This previously saturated silently, which was memory-safe
    /// but hid a condition the spec requires reporting.
    #[must_use = "an unapplied increment is a FLOW_CONTROL_ERROR and must be signalled"]
    pub fn increase_window(&mut self, increment: u32) -> bool {
        match self.window.checked_add_unsigned(increment) {
            Some(w) => {
                self.window = w;
                true
            }
            None => false,
        }
    }

    /// Consume window capacity (data sent or received).
    pub fn consume(&mut self, amount: u32) {
        self.window -= amount as i32;
        self.consumed += amount;
    }

    /// Check if we should send a WINDOW_UPDATE.
    pub fn should_update(&self) -> bool {
        self.consumed >= self.update_threshold
    }

    /// Get the pending window update amount.
    pub fn pending_update(&self) -> u32 {
        self.consumed
    }

    /// Reset consumed counter after sending WINDOW_UPDATE.
    pub fn reset_pending(&mut self) {
        self.window += self.consumed as i32;
        self.consumed = 0;
    }

    /// Adjust window for settings change.
    pub fn adjust_window(&mut self, delta: i32) {
        self.window = self.window.saturating_add(delta);
    }

    /// Get the initial window size.
    pub fn initial_window(&self) -> u32 {
        self.initial_window
    }

    /// Set a new initial window size.
    pub fn set_initial_window(&mut self, new_initial: u32) {
        let delta = new_initial as i32 - self.initial_window as i32;
        self.initial_window = new_initial;
        self.window = self.window.saturating_add(delta);
        self.update_threshold = new_initial / 2;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_rejects_overflowing_increment() {
        let mut fc = FlowControl::new(65_535);
        let mut applied = 0;
        loop {
            let before = fc.available();
            if !fc.increase_window(1 << 30) {
                assert_eq!(fc.available(), before, "must leave the window unchanged");
                break;
            }
            applied += 1;
            assert!(fc.available() > 0, "window wrapped negative");
            assert!(applied < 10, "increment was never rejected");
        }
        assert!(
            applied >= 1,
            "a legitimate increment must still be accepted"
        );
    }

    #[test]
    fn test_initial_state() {
        let fc = FlowControl::new(65535);
        assert_eq!(fc.available(), 65535);
        assert!(!fc.should_update());
    }

    #[test]
    fn test_consume_and_update() {
        let mut fc = FlowControl::new(65535);

        // Consume some data
        fc.consume(30000);
        assert_eq!(fc.available(), 35535);
        assert!(!fc.should_update()); // Haven't hit threshold yet

        // Consume more to hit threshold
        fc.consume(10000);
        assert_eq!(fc.available(), 25535);
        assert!(fc.should_update()); // Now should update

        // Get pending and reset
        assert_eq!(fc.pending_update(), 40000);
        fc.reset_pending();
        assert_eq!(fc.available(), 65535); // Window restored
        assert!(!fc.should_update());
    }

    #[test]
    fn test_window_increase() {
        let mut fc = FlowControl::new(65535);

        fc.consume(30000);
        assert_eq!(fc.available(), 35535);

        assert!(fc.increase_window(20000));
        assert_eq!(fc.available(), 55535);
    }

    #[test]
    fn test_settings_adjustment() {
        let mut fc = FlowControl::new(65535);

        // Consume some data
        fc.consume(10000);
        assert_eq!(fc.available(), 55535);

        // Adjust for new settings (increase by 10000)
        fc.adjust_window(10000);
        assert_eq!(fc.available(), 65535);
    }
}
