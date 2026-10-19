//! Bounded channel records and the version-three kind table. This crate keeps no
//! runtime state and knows no domain, credentials or host policy. Generated
//! constructors, encoders and decoders handle bodies; [`schema`] advertises
//! their largest permitted encodings. Contracts: `protocol/channel.md`,
//! sections 2-4 and 6-8.

#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]

extern crate alloc;

#[rustfmt::skip]
#[path = "generated/v3.rs"]
pub mod v3;

mod kinds;

pub use kinds::{KindRule, Overflow, V3, peer_terms_gap, schema};
pub use v3::{
    Cap, CompletionLimit, CompletionPhase, LimitFailure, LimitFailureParts, PathCap, PathCapParts, PoolLimit,
    PoolLimitParts, ResponseEvidence, ResponseEvidenceParts, TimeoutFailure, TimeoutFailureParts,
};

pub use v3::{
    Accepted, AcceptedParts, Acknowledge, AcknowledgeParts, Admitted, AdmittedParts, Answer, AnswerParts, AnsweredCall,
    AnsweredCallParts, Ask, BudgetCeiling, BudgetCeilingValue, BudgetCeilingValueParts, BudgetChecks,
    BudgetChecksParts, BudgetCompletion, BudgetCompletionParts, BudgetFailure, BudgetFailureValue,
    BudgetFailureValueParts, BudgetPart, BudgetPartValue, BudgetPartValueParts, BudgetRefusal, BudgetRefusalValue,
    BudgetRefusalValueParts, BudgetSpend, BudgetSpendParts, BudgetTime, BudgetTimeParts, BudgetTurns, BudgetTurnsParts,
    CEILINGS, Call, CallName, CallNameParts, CallParts, Cancel, CancelParts, CompletionEvidence, CompletionFailure,
    CompletionFault, CompletionFaultParts, DeliverAsk, DeliverAskParts, Delivered, DeliveredParts, Delivery,
    DeliveryFailure, DeliveryFailureParts, DeliveryReason, DeliveryRefusal, DeliveryRefusalParts, DeliveryReply,
    DeliveryReplyParts, Directory, DirectoryParts, Effect, Exhausted, ExhaustedParts, Fact, FactKind, FactParts,
    Failed, FailedParts, Field, FieldParts, Grant, GrantParts, GrantRefresh, GrantRefreshParts, GrantValue,
    GrantValueParts, HostAnswer, HostAnswerParts, HostAsk, HostAskParts, HostReply, HostReplyParts, InvalidStart,
    InvalidStartValue, InvalidStartValueParts, Limits, Live, LiveAsk, LiveAskParts, LiveCall, LiveCallParts, LiveKind,
    LiveParts, Long, LongDone, LongDoneParts, LongParts, Marker, MarkerParts, Message, MessageParts, MessageRefusal,
    MessageRefused, MessageRefusedParts, ModelFault, ModelFaultValue, ModelFaultValueParts, Operation,
    Overflow as BudgetOverflow, OverflowValue, OverflowValueParts, PolicyFailure, PolicyFailureParts, PriceRate,
    PriceRateValue, PriceRateValueParts, Receipt, ReceiptParts, ReceivingLimit, ReceivingLimitValue,
    ReceivingLimitValueParts, Refused, RefusedParts, Rejected, RejectedParts, Reply, RunFailure, RunResult, SavedReply,
    Start, StartParts, StartRefusal, TranscriptRefusal, TranscriptRefusalValue, TranscriptRefusalValueParts, Turn,
    TurnParts, Waiting, WaitingParts, Window, WindowBound, WindowBoundParts, WindowParts, Withdraw, WithdrawParts,
    Workspace, WorkspaceParts,
};
