/// Trait that application state machines must implement.
///
/// The Raft engine calls [`apply`] for every committed log entry (in order).
/// Application-specific command serialization is the caller's responsibility;
/// the core library treats commands as opaque bytes.
pub trait StateMachine: Send + 'static {
    /// Apply a committed command and return a result payload.
    ///
    /// * `command` may be `None` for protocol-level no-op entries -- the
    ///   implementation should simply ignore those (return `None`).
    /// * For real commands, the `Vec<u8>` contains whatever the application
    ///   serialized when it submitted the write via the client RPC.
    fn apply(&mut self, command: &Option<Vec<u8>>) -> Option<Vec<u8>>;

    /// Handle a read-only query without writing to the log.
    ///
    /// This is used for client `Get`-style operations that do not need to
    /// be replicated.  The `query` payload is application-defined.
    fn query(&self, query: &[u8]) -> Option<Vec<u8>>;
}
