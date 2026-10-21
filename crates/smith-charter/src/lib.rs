//! Bounded charter and result records; this crate retains no runtime state and
//! knows no domain, endpoint configuration or host policy. Constructors,
//! encoders and decoders are generated from `protocol/charter.md`, sections 2-6.

#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]

extern crate alloc;

use alloc::boxed::Box;

#[rustfmt::skip]
#[path = "generated/v4.rs"]
pub mod v4;

pub use v4::{
    Budget, BudgetParts, CEILINGS, ChangeRule, ChangeRuleParts, Charter, CharterParts, Contract, ContractParts,
    Conventions, ConventionsParts, Effect, Families, FamiliesParts, Field, FieldParts, FieldRule, FieldRuleParts, Form,
    HostTool, HostToolParts, Item, ItemKind, ItemKindParts, ItemParts, Llm, LlmParts, Mode, Prices, PricesParts,
    Reserve, ReserveParts, RunResult, RunResultParts, Section, SectionParts, TextRule, TextRuleParts, Tools,
    ToolsParts, VerdictRule, VerdictRuleParts,
};

impl Charter {
    /// The wire format version this codec writes and reads.
    pub const FORMAT_VERSION: u64 = 4;
}

/// Provider effort name, bounded to 32 UTF-8 bytes by the charter codec.
pub type Effort = Box<[u8]>;
