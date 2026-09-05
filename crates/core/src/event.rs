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
/// ```rust,ignore
/// use account_decoder_core::{DecodedEvent, EventKind};
///
/// #[derive(Debug)]
/// struct TokenMint {
///     pub supply: u64,
///     pub decimals: u8,
///     // ... other fields
/// }
///
/// impl DecodedEvent for TokenMint {
///     fn event_kind(&self) -> EventKind {
///         EventKind::Account
///     }
///
///     fn event_type(&self) -> &'static str {
///         "TokenMint"
///     }
///
///     fn program_name(&self) -> &'static str {
///         "spl-token"
///     }
///
///     fn as_any(&self) -> &dyn Any {
///         self
///     }
/// }
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
    /// ```rust,ignore
    /// let event: Box<dyn DecodedEvent> = decoder.decode_account(data)?;
    ///
    /// if let Some(mint) = event.as_any().downcast_ref::<TokenMint>() {
    ///     println!("Supply: {}", mint.supply);
    /// }
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
