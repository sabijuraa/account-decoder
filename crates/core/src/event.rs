//! Decoded event types and traits.
//!
//! Events represent successfully decoded account or instruction data.
//! They provide a uniform interface while supporting type-safe downcasting
//! to program-specific types.

use std::any::Any;
use std::fmt::{self, Debug};

/// The kind of decoded event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventKind {
    /// Decoded from account data.
    Account,
    /// Decoded from instruction data.
    Instruction,
}

impl fmt::Display for EventKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Account => write!(f, "account"),
            Self::Instruction => write!(f, "instruction"),
        }
    }
}

/// Trait for decoded events.
///
/// All decoded account/instruction data implements this trait, enabling:
/// - Uniform handling in registries and pipelines
/// - Type-safe downcasting to concrete types
/// - Debug and display formatting
///
/// # Implementing
///
/// ```rust
/// use account_decoder_core::{DecodedEvent, EventKind, TypedEvent};
/// use std::any::Any;
///
/// #[derive(Debug)]
/// struct TokenMint {
///     supply: u64,
///     decimals: u8,
/// }
///
/// impl DecodedEvent for TokenMint {
///     fn event_kind(&self) -> EventKind { EventKind::Account }
///     fn event_type(&self) -> &'static str { "TokenMint" }
///     fn program_name(&self) -> &'static str { "spl-token" }
///     fn as_any(&self) -> &dyn Any { self }
/// }
///
/// // Once boxed, the concrete type is recovered by downcasting.
/// let event: Box<dyn DecodedEvent> = Box::new(TokenMint { supply: 100, decimals: 6 });
/// assert_eq!(event.event_type(), "TokenMint");
///
/// let mint = event.downcast_ref::<TokenMint>().expect("it is a TokenMint");
/// assert_eq!(mint.decimals, 6);
/// ```
pub trait DecodedEvent: Debug + Send + Sync {
    /// Returns whether this is an account or instruction event.
    fn event_kind(&self) -> EventKind;

    /// Returns the specific type name within the program.
    ///
    /// For example: "TokenAccount", "Mint", "Transfer", "Initialize".
    fn event_type(&self) -> &'static str;

    /// Returns the program name this event belongs to.
    ///
    /// For example: "spl-token", "system", "token-2022".
    fn program_name(&self) -> &'static str;

    /// Allows downcasting to the concrete type.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use account_decoder_core::{DecodedEvent, EventKind, TypedEvent};
    /// # use std::any::Any;
    /// # #[derive(Debug)]
    /// # struct Mint { decimals: u8 }
    /// # impl DecodedEvent for Mint {
    /// #     fn event_kind(&self) -> EventKind { EventKind::Account }
    /// #     fn event_type(&self) -> &'static str { "Mint" }
    /// #     fn program_name(&self) -> &'static str { "spl-token" }
    /// #     fn as_any(&self) -> &dyn Any { self }
    /// # }
    /// let event: Box<dyn DecodedEvent> = Box::new(Mint { decimals: 9 });
    ///
    /// // The right type comes back; the wrong one is None rather than a panic.
    /// assert_eq!(event.downcast_ref::<Mint>().map(|m| m.decimals), Some(9));
    /// assert!(event.downcast_ref::<String>().is_none());
    /// ```
    fn as_any(&self) -> &dyn Any;

    /// Returns a fully qualified event name.
    ///
    /// Default: "{program_name}::{event_type}"
    fn full_name(&self) -> String {
        format!("{}::{}", self.program_name(), self.event_type())
    }

    /// Returns the size of the original encoded data, if known.
    ///
    /// This is useful for metrics and debugging.
    fn encoded_size(&self) -> Option<usize> {
        None
    }
}

/// Extension trait for type-safe downcasting of decoded events.
///
/// This provides ergonomic methods for extracting concrete types from
/// `Box<dyn DecodedEvent>` values.
pub trait TypedEvent: DecodedEvent {
    /// Attempt to downcast to a specific type.
    ///
    /// Returns `Some(&T)` if this event is of type `T`, `None` otherwise.
    fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.as_any().downcast_ref()
    }

    /// Check if this event is of a specific type.
    fn is<T: 'static>(&self) -> bool {
        self.as_any().is::<T>()
    }
}

// Blanket implementation for all DecodedEvent types
impl<T: DecodedEvent + ?Sized> TypedEvent for T {}

/// A wrapper that adds context to decoded events.
///
/// Useful when you need to associate additional metadata with an event
/// without modifying the underlying type.
#[derive(Debug)]
pub struct ContextualEvent<T: DecodedEvent> {
    /// The underlying event.
    pub event: T,
    /// Optional slot number where this was observed.
    pub slot: Option<u64>,
    /// Optional account/transaction signature.
    pub signature: Option<String>,
}

impl<T: DecodedEvent + 'static> DecodedEvent for ContextualEvent<T> {
    fn event_kind(&self) -> EventKind {
        self.event.event_kind()
    }

    fn event_type(&self) -> &'static str {
        self.event.event_type()
    }

    fn program_name(&self) -> &'static str {
        self.event.program_name()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn encoded_size(&self) -> Option<usize> {
        self.event.encoded_size()
    }
}

/// An event that couldn't be fully decoded but contains partial information.
///
/// This is useful for graceful degradation - when full decoding fails,
/// we can still provide raw data and any successfully parsed fields.
#[derive(Debug)]
pub struct PartialEvent {
    /// The event kind (account/instruction).
    pub kind: EventKind,
    /// The program name.
    pub program: &'static str,
    /// Raw data that couldn't be decoded.
    pub raw_data: Vec<u8>,
    /// Any successfully parsed discriminator.
    pub discriminator: Option<Vec<u8>>,
    /// Human-readable description of what went wrong.
    pub error_context: String,
}

impl DecodedEvent for PartialEvent {
    fn event_kind(&self) -> EventKind {
        self.kind
    }

    fn event_type(&self) -> &'static str {
        "Partial"
    }

    fn program_name(&self) -> &'static str {
        self.program
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn encoded_size(&self) -> Option<usize> {
        Some(self.raw_data.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct Simple;

    impl DecodedEvent for Simple {
        fn event_kind(&self) -> EventKind {
            EventKind::Account
        }
        fn event_type(&self) -> &'static str {
            "Simple"
        }
        fn program_name(&self) -> &'static str {
            "test"
        }
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    #[test]
    fn contextual_event_carries_provenance_without_changing_the_event() {
        // An indexer needs to know which slot an account was read at. Wrapping
        // rather than widening every event type keeps that out of the decoders.
        let wrapped = ContextualEvent {
            event: Simple,
            slot: Some(250_000_000),
            signature: Some("5xy".to_string()),
        };

        assert_eq!(wrapped.event_type(), "Simple", "the inner type shows through");
        assert_eq!(wrapped.event_kind(), EventKind::Account);
        assert_eq!(wrapped.program_name(), "test");
        assert_eq!(wrapped.slot, Some(250_000_000));
        assert_eq!(wrapped.signature.as_deref(), Some("5xy"));
    }

    #[test]
    fn a_partial_event_keeps_the_bytes_it_could_not_decode() {
        // Graceful degradation: a caller that cannot decode an account should
        // still be able to say which program it belonged to and hand the raw
        // bytes on, rather than dropping it.
        let raw = vec![9u8; 40];
        let partial = PartialEvent {
            kind: EventKind::Instruction,
            program: "unknown-program",
            raw_data: raw.clone(),
            discriminator: Some(raw[..8].to_vec()),
            error_context: "no decoder matched the discriminator".to_string(),
        };

        assert_eq!(partial.event_type(), "Partial");
        assert_eq!(partial.event_kind(), EventKind::Instruction);
        assert_eq!(partial.encoded_size(), Some(40));
        assert_eq!(partial.discriminator.as_deref(), Some(&raw[..8]));
        assert!(partial.error_context.contains("no decoder"));
    }

    #[derive(Debug)]
    struct TestEvent {
        pub value: u64,
    }

    impl DecodedEvent for TestEvent {
        fn event_kind(&self) -> EventKind {
            EventKind::Account
        }

        fn event_type(&self) -> &'static str {
            "TestEvent"
        }

        fn program_name(&self) -> &'static str {
            "test-program"
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    #[test]
    fn test_event_basic() {
        let event = TestEvent { value: 42 };
        assert_eq!(event.event_kind(), EventKind::Account);
        assert_eq!(event.event_type(), "TestEvent");
        assert_eq!(event.full_name(), "test-program::TestEvent");
    }

    #[test]
    fn test_downcast() {
        let event: Box<dyn DecodedEvent> = Box::new(TestEvent { value: 42 });

        assert!(event.is::<TestEvent>());
        assert!(!event.is::<PartialEvent>());

        let downcasted = event.downcast_ref::<TestEvent>().unwrap();
        assert_eq!(downcasted.value, 42);
    }

    #[test]
    fn test_event_kind_display() {
        assert_eq!(EventKind::Account.to_string(), "account");
        assert_eq!(EventKind::Instruction.to_string(), "instruction");
    }
}
