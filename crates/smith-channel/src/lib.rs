//! Bounded channel records and the version-five kind table. This crate keeps no
//! runtime state and knows no domain, credentials or host policy. Generated
//! constructors, encoders and decoders handle bodies; [`schema`] advertises
//! their largest permitted encodings. Contracts: `protocol/channel.md`,
//! sections 2-4 and 6-8.

#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]

extern crate alloc;

#[rustfmt::skip]
#[path = "generated/v5.rs"]
pub mod v5;

mod kinds;

pub use kinds::{KindRule, Overflow, V5, peer_terms_gap, schema};
pub use v5::{
    Cap, CompletionLimit, CompletionPhase, LimitFailure, LimitFailureParts, PathCap, PathCapParts, PoolLimit,
    PoolLimitParts, ResponseEvidence, ResponseEvidenceParts, TimeoutFailure, TimeoutFailureParts,
};

pub use v5::{
    Accepted, AcceptedParts, AccountRefused, AccountRefusedParts, Acknowledge, AcknowledgeParts, Admitted,
    AdmittedParts, Answer, AnswerParts, AnsweredCall, AnsweredCallParts, Ask, BudgetCeiling, BudgetCeilingValue,
    BudgetCeilingValueParts, BudgetChecks, BudgetChecksParts, BudgetCompletion, BudgetCompletionParts, BudgetFailure,
    BudgetFailureValue, BudgetFailureValueParts, BudgetPart, BudgetPartValue, BudgetPartValueParts, BudgetRefusal,
    BudgetRefusalValue, BudgetRefusalValueParts, BudgetSpend, BudgetSpendParts, BudgetTime, BudgetTimeParts,
    BudgetTurns, BudgetTurnsParts, CEILINGS, Call, CallName, CallNameParts, CallParts, Cancel, CancelParts,
    CompletionEvidence, CompletionFailure, CompletionFault, CompletionFaultParts, CredentialValue,
    CredentialValueParts, DeliverAsk, DeliverAskParts, Delivered, DeliveredParts, Delivery, DeliveryFailure,
    DeliveryFailureParts, DeliveryReason, DeliveryRefusal, DeliveryRefusalParts, DeliveryReply, DeliveryReplyParts,
    Directory, DirectoryParts, Effect, Fact, FactKind, FactParts, Failed, FailedParts, Field, FieldParts, Grant,
    GrantParts, GrantRefresh, GrantRefreshParts, GrantValue, HostAnswer, HostAnswerParts, HostAsk, HostAskParts,
    HostReply, HostReplyParts, InvalidStart, InvalidStartValue, InvalidStartValueParts, Limited, LimitedFault,
    LimitedFaultParts, LimitedParts, Limits, Live, LiveAsk, LiveAskParts, LiveCall, LiveCallParts, LiveKind, LiveParts,
    Long, LongDone, LongDoneParts, LongParts, Marker, MarkerParts, Message, MessageParts, MessageRefusal,
    MessageRefused, MessageRefusedParts, ModelFault, ModelFaultValue, ModelFaultValueParts, Operation,
    Overflow as BudgetOverflow, OverflowValue, OverflowValueParts, PolicyFailure, PolicyFailureParts, PriceRate,
    PriceRateValue, PriceRateValueParts, ProgramFault, ProgramFaultParts, ProgramValue, ProgramValueParts, Receipt,
    ReceiptParts, ReceivingLimit, ReceivingLimitValue, ReceivingLimitValueParts, Refused, RefusedFault,
    RefusedFaultParts, RefusedParts, Reply, RunFailure, RunResult, SavedReply, Scheme, Start, StartParts, StartRefusal,
    TranscriptRefusal, TranscriptRefusalValue, TranscriptRefusalValueParts, Turn, TurnParts, Unauthorized,
    UnauthorizedFault, UnauthorizedFaultParts, UnauthorizedParts, Waiting, WaitingParts, Window, WindowBound,
    WindowBoundParts, WindowParts, Withdraw, WithdrawParts, WithdrawnValue, WithdrawnValueParts, Workspace,
    WorkspaceParts,
};
