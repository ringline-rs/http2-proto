//! HTTP/2 stream state tracking.

pub use crate::frame::StreamId;

/// Stream state (RFC 7540 Section 5.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamState {
    /// Stream is idle (not yet used).
    Idle,
    /// Reserved for server push (we don't use this as a client).
    ReservedRemote,
    /// Stream is open (can send and receive).
    Open,
    /// Half-closed (local) - we've sent END_STREAM.
    HalfClosedLocal,
    /// Half-closed (remote) - peer sent END_STREAM.
    HalfClosedRemote,
    /// Stream is closed.
    Closed,
}

/// An HTTP/2 stream.
#[derive(Debug)]
pub struct Stream {
    /// Stream identifier.
    id: StreamId,
    /// Current state.
    state: StreamState,
    /// Send-side flow control window.
    send_window: i32,
    /// Receive-side flow control window.
    recv_window: i32,
    /// Bytes received (for window updates).
    bytes_received: u32,
}

impl Stream {
    /// Create a new stream.
    pub fn new(id: StreamId, initial_window_size: u32) -> Self {
        Self {
            id,
            state: StreamState::Open,
            send_window: initial_window_size as i32,
            recv_window: initial_window_size as i32,
            bytes_received: 0,
        }
    }

    /// Get the stream ID.
    pub fn id(&self) -> StreamId {
        self.id
    }

    /// Get the stream state.
    pub fn state(&self) -> StreamState {
        self.state
    }

    /// Check if the stream is open for sending.
    pub fn can_send(&self) -> bool {
        matches!(
            self.state,
            StreamState::Open | StreamState::HalfClosedRemote
        )
    }

    /// Check if the stream is open for receiving.
    pub fn can_recv(&self) -> bool {
        matches!(self.state, StreamState::Open | StreamState::HalfClosedLocal)
    }

    /// Get the send window size.
    pub fn send_window(&self) -> i32 {
        self.send_window
    }

    /// Get the receive window size.
    pub fn recv_window(&self) -> i32 {
        self.recv_window
    }

    /// Record that we sent data.
    pub fn send_data(&mut self, size: u32) {
        self.send_window -= size as i32;
    }

    /// Record that we sent END_STREAM.
    pub fn send_end_stream(&mut self) {
        self.state = match self.state {
            StreamState::Open => StreamState::HalfClosedLocal,
            StreamState::HalfClosedRemote => StreamState::Closed,
            other => other,
        };
    }

    /// Record that we received data.
    pub fn recv_data(&mut self, size: u32) {
        self.recv_window -= size as i32;
        self.bytes_received += size;
    }

    /// Record that we received END_STREAM.
    pub fn recv_end_stream(&mut self) {
        self.state = match self.state {
            StreamState::Open => StreamState::HalfClosedRemote,
            StreamState::HalfClosedLocal => StreamState::Closed,
            other => other,
        };
    }

    /// Increase the send window (from WINDOW_UPDATE).
    /// Increase the send window by a WINDOW_UPDATE increment.
    ///
    /// Returns `false` if the increment would take the window above
    /// `MAX_WINDOW_SIZE` (2^31 - 1). RFC 7540 section 6.9.1 requires that case
    /// to be a `FLOW_CONTROL_ERROR`; the window is left unchanged so the caller
    /// can signal it. The previous unchecked `+=` overflowed instead: three
    /// well-formed WINDOW_UPDATE frames of 2^30 panicked a debug build and
    /// wrapped the window negative in release.
    #[must_use = "an unapplied increment is a FLOW_CONTROL_ERROR and must be signalled"]
    pub fn increase_send_window(&mut self, increment: u32) -> bool {
        match self.send_window.checked_add_unsigned(increment) {
            Some(w) => {
                self.send_window = w;
                true
            }
            None => false,
        }
    }

    /// Adjust the send window (from SETTINGS change).
    pub fn adjust_send_window(&mut self, delta: i32) {
        self.send_window += delta;
    }

    /// Mark the stream as reset.
    pub fn reset(&mut self) {
        self.state = StreamState::Closed;
    }

    /// Get bytes received since last window update.
    pub fn bytes_received(&self) -> u32 {
        self.bytes_received
    }

    /// Reset bytes received counter.
    pub fn reset_bytes_received(&mut self) {
        self.bytes_received = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn send_window_rejects_overflowing_increment() {
        // WINDOW_UPDATE increments are peer-controlled and each fits the 31-bit
        // field, so they accumulate. The unchecked `+=` panicked in a debug
        // build and wrapped the window to -1073676289 in release.
        let mut stream = Stream::new(StreamId::new(1), 65_535);
        let mut applied = 0;
        loop {
            let before = stream.send_window();
            if !stream.increase_send_window(1 << 30) {
                assert_eq!(
                    stream.send_window(),
                    before,
                    "a rejected increment must leave the window unchanged"
                );
                break;
            }
            applied += 1;
            assert!(
                stream.send_window() > 0,
                "window wrapped negative after {applied} increments"
            );
            assert!(applied < 10, "increment was never rejected");
        }
        assert!(
            applied >= 1,
            "a legitimate increment must still be accepted"
        );
    }

    #[test]
    fn test_stream_new() {
        let stream = Stream::new(StreamId::new(5), 65535);
        assert_eq!(stream.id().value(), 5);
        assert_eq!(stream.state(), StreamState::Open);
        assert_eq!(stream.send_window(), 65535);
        assert_eq!(stream.recv_window(), 65535);
        assert_eq!(stream.bytes_received(), 0);
    }

    #[test]
    fn test_stream_lifecycle() {
        let mut stream = Stream::new(StreamId::new(1), 65535);

        assert_eq!(stream.state(), StreamState::Open);
        assert!(stream.can_send());
        assert!(stream.can_recv());

        // Send END_STREAM
        stream.send_end_stream();
        assert_eq!(stream.state(), StreamState::HalfClosedLocal);
        assert!(!stream.can_send());
        assert!(stream.can_recv());

        // Receive END_STREAM
        stream.recv_end_stream();
        assert_eq!(stream.state(), StreamState::Closed);
        assert!(!stream.can_send());
        assert!(!stream.can_recv());
    }

    #[test]
    fn test_stream_lifecycle_recv_first() {
        let mut stream = Stream::new(StreamId::new(1), 65535);

        // Receive END_STREAM first
        stream.recv_end_stream();
        assert_eq!(stream.state(), StreamState::HalfClosedRemote);
        assert!(stream.can_send());
        assert!(!stream.can_recv());

        // Then send END_STREAM
        stream.send_end_stream();
        assert_eq!(stream.state(), StreamState::Closed);
        assert!(!stream.can_send());
        assert!(!stream.can_recv());
    }

    #[test]
    fn test_stream_send_end_stream_already_closed() {
        let mut stream = Stream::new(StreamId::new(1), 65535);
        stream.reset();
        assert_eq!(stream.state(), StreamState::Closed);

        // Sending END_STREAM on closed stream doesn't change state
        stream.send_end_stream();
        assert_eq!(stream.state(), StreamState::Closed);
    }

    #[test]
    fn test_stream_recv_end_stream_already_closed() {
        let mut stream = Stream::new(StreamId::new(1), 65535);
        stream.reset();
        assert_eq!(stream.state(), StreamState::Closed);

        // Receiving END_STREAM on closed stream doesn't change state
        stream.recv_end_stream();
        assert_eq!(stream.state(), StreamState::Closed);
    }

    #[test]
    fn test_flow_control() {
        let mut stream = Stream::new(StreamId::new(1), 65535);

        assert_eq!(stream.send_window(), 65535);

        stream.send_data(1000);
        assert_eq!(stream.send_window(), 64535);

        assert!(stream.increase_send_window(500));
        assert_eq!(stream.send_window(), 65035);
    }

    #[test]
    fn test_recv_flow_control() {
        let mut stream = Stream::new(StreamId::new(1), 65535);

        assert_eq!(stream.recv_window(), 65535);
        assert_eq!(stream.bytes_received(), 0);

        stream.recv_data(1000);
        assert_eq!(stream.recv_window(), 64535);
        assert_eq!(stream.bytes_received(), 1000);

        stream.recv_data(500);
        assert_eq!(stream.recv_window(), 64035);
        assert_eq!(stream.bytes_received(), 1500);
    }

    #[test]
    fn test_reset_bytes_received() {
        let mut stream = Stream::new(StreamId::new(1), 65535);

        stream.recv_data(1000);
        assert_eq!(stream.bytes_received(), 1000);

        stream.reset_bytes_received();
        assert_eq!(stream.bytes_received(), 0);
    }

    #[test]
    fn test_adjust_send_window() {
        let mut stream = Stream::new(StreamId::new(1), 65535);

        stream.adjust_send_window(1000);
        assert_eq!(stream.send_window(), 66535);

        stream.adjust_send_window(-2000);
        assert_eq!(stream.send_window(), 64535);
    }

    #[test]
    fn test_stream_reset() {
        let mut stream = Stream::new(StreamId::new(1), 65535);
        assert_eq!(stream.state(), StreamState::Open);

        stream.reset();
        assert_eq!(stream.state(), StreamState::Closed);
        assert!(!stream.can_send());
        assert!(!stream.can_recv());
    }

    #[test]
    fn test_stream_state_debug() {
        let states = [
            StreamState::Idle,
            StreamState::ReservedRemote,
            StreamState::Open,
            StreamState::HalfClosedLocal,
            StreamState::HalfClosedRemote,
            StreamState::Closed,
        ];

        for state in states {
            let debug = format!("{:?}", state);
            assert!(!debug.is_empty());
        }
    }

    #[test]
    fn test_stream_state_eq() {
        assert_eq!(StreamState::Open, StreamState::Open);
        assert_ne!(StreamState::Open, StreamState::Closed);
    }

    #[test]
    fn test_stream_debug() {
        let stream = Stream::new(StreamId::new(1), 65535);
        let debug = format!("{:?}", stream);
        assert!(debug.contains("Stream"));
    }

    #[test]
    fn test_can_send_states() {
        let mut stream = Stream::new(StreamId::new(1), 65535);

        // Open - can send
        assert!(stream.can_send());

        // HalfClosedRemote - can send
        stream.recv_end_stream();
        assert!(stream.can_send());

        // HalfClosedLocal - cannot send
        let mut stream2 = Stream::new(StreamId::new(3), 65535);
        stream2.send_end_stream();
        assert!(!stream2.can_send());
    }

    #[test]
    fn test_can_recv_states() {
        let mut stream = Stream::new(StreamId::new(1), 65535);

        // Open - can recv
        assert!(stream.can_recv());

        // HalfClosedLocal - can recv
        stream.send_end_stream();
        assert!(stream.can_recv());

        // HalfClosedRemote - cannot recv
        let mut stream2 = Stream::new(StreamId::new(3), 65535);
        stream2.recv_end_stream();
        assert!(!stream2.can_recv());
    }
}
