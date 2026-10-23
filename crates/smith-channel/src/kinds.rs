//! Smith's one-version body table. It keeps no state and knows no domain;
//! [`schema`] projects the generated body bounds into Skein's channel table.
//! Contract: `protocol/channel.md`, sections 2-4.

use skein_channel::{Direction, Kind, Role, Schema, Term, Version};
use skein_lib::List;

use crate::v5;

/// A version-five body's number, sender, admission obligation and codec.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[expect(
    clippy::partial_pub_fields,
    reason = "the codec selector is internal; number, direction and obligation are public"
)]
pub struct KindRule {
    pub kind: u16,
    pub direction: Direction,
    pub required: bool,
    body: Body,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Body {
    Start,
    Message,
    MessageRefused,
    HostAnswer,
    Acknowledge,
    GrantRefresh,
    Cancel,
    Admitted,
    Call,
    Withdraw,
    Turn,
    Waiting,
    Long,
    LongDone,
    Unauthorized,
    Limited,
    Fact,
    Answer,
    Live,
    AccountRefused,
}

/// Why a body maximum cannot be represented in the channel's frame length.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Overflow {
    /// Generated bound arithmetic exceeded its checked range.
    Body,
    /// The body exceeds a frame's `u32` length.
    Frame,
}

/// The 20 kinds required by channel version five, in wire number order.
pub const V5: &[KindRule] = &[
    KindRule { kind: 0x0100, direction: Direction::FromInitiator, required: true, body: Body::Start },
    KindRule { kind: 0x0101, direction: Direction::FromInitiator, required: true, body: Body::Message },
    KindRule { kind: 0x0102, direction: Direction::FromInitiator, required: true, body: Body::HostAnswer },
    KindRule { kind: 0x0103, direction: Direction::FromInitiator, required: true, body: Body::Acknowledge },
    KindRule { kind: 0x0104, direction: Direction::FromInitiator, required: true, body: Body::GrantRefresh },
    KindRule { kind: 0x0105, direction: Direction::FromInitiator, required: true, body: Body::Cancel },
    KindRule { kind: 0x0106, direction: Direction::FromResponder, required: true, body: Body::Admitted },
    KindRule { kind: 0x0107, direction: Direction::FromResponder, required: true, body: Body::Call },
    KindRule { kind: 0x0108, direction: Direction::FromResponder, required: true, body: Body::Withdraw },
    KindRule { kind: 0x0109, direction: Direction::FromResponder, required: true, body: Body::Turn },
    KindRule { kind: 0x010a, direction: Direction::FromResponder, required: true, body: Body::Waiting },
    KindRule { kind: 0x010b, direction: Direction::FromResponder, required: true, body: Body::Long },
    KindRule { kind: 0x010c, direction: Direction::FromResponder, required: true, body: Body::LongDone },
    KindRule { kind: 0x010d, direction: Direction::FromResponder, required: true, body: Body::Unauthorized },
    KindRule { kind: 0x010e, direction: Direction::FromResponder, required: true, body: Body::Limited },
    KindRule { kind: 0x010f, direction: Direction::FromResponder, required: true, body: Body::Fact },
    KindRule { kind: 0x0110, direction: Direction::FromResponder, required: true, body: Body::Answer },
    KindRule { kind: 0x0111, direction: Direction::FromResponder, required: true, body: Body::MessageRefused },
    KindRule { kind: 0x0112, direction: Direction::FromResponder, required: true, body: Body::Live },
    KindRule { kind: 0x0113, direction: Direction::FromResponder, required: true, body: Body::AccountRefused },
];

/// Build the version-five channel schema from receiving codec limits.
#[expect(clippy::manual_let_else, reason = "the strict step subset uses exhaustive matches")]
pub fn schema(limits: &v5::Limits) -> Result<Schema, Overflow> {
    let mut kinds = List::with_capacity(20);
    for rule in V5 {
        let largest = match rule.body {
            Body::Start => v5::Start::worst_case_bytes(limits),
            Body::Message => v5::Message::worst_case_bytes(limits),
            Body::MessageRefused => v5::MessageRefused::worst_case_bytes(limits),
            Body::HostAnswer => v5::HostAnswer::worst_case_bytes(limits),
            Body::Acknowledge => v5::Acknowledge::worst_case_bytes(limits),
            Body::GrantRefresh => v5::GrantRefresh::worst_case_bytes(limits),
            Body::Cancel => v5::Cancel::worst_case_bytes(limits),
            Body::Admitted => v5::Admitted::worst_case_bytes(limits),
            Body::Call => v5::Call::worst_case_bytes(limits),
            Body::Withdraw => v5::Withdraw::worst_case_bytes(limits),
            Body::Turn => v5::Turn::worst_case_bytes(limits),
            Body::Waiting => v5::Waiting::worst_case_bytes(limits),
            Body::Long => v5::Long::worst_case_bytes(limits),
            Body::LongDone => v5::LongDone::worst_case_bytes(limits),
            Body::Unauthorized => v5::Unauthorized::worst_case_bytes(limits),
            Body::Limited => v5::Limited::worst_case_bytes(limits),
            Body::Fact => v5::Fact::worst_case_bytes(limits),
            Body::Answer => v5::Answer::worst_case_bytes(limits),
            Body::AccountRefused => v5::AccountRefused::worst_case_bytes(limits),
            Body::Live => v5::Live::worst_case_bytes(limits),
        };
        let largest = largest.ok_or(Overflow::Body)?;
        let largest = match u32::try_from(largest) {
            Ok(largest) => largest,
            Err(_) => return Err(Overflow::Frame),
        };
        kinds.push(Kind { kind: rule.kind, direction: rule.direction, largest }).expect("V5 has exactly 20 kinds");
    }
    let mut versions = List::with_capacity(1);
    versions.push(Version { version: 5, kinds }).expect("one version has one slot");
    Ok(Schema { magic: *b"smth", versions })
}

/// Find the first required kind that the peer cannot receive at our sending bound.
/// Contract: `protocol/channel.md`, section 2.
#[must_use]
pub fn peer_terms_gap(schema: &Schema, role: Role, version: u16, terms: &List<Term>) -> Option<u16> {
    required_terms_gap(schema, role, version, terms, V5)
}

fn required_terms_gap(
    schema: &Schema,
    role: Role,
    version: u16,
    terms: &List<Term>,
    rules: &[KindRule],
) -> Option<u16> {
    let sending = match role {
        Role::Initiator => Direction::FromInitiator,
        Role::Responder => Direction::FromResponder,
    };
    let table = schema.version(version)?;
    for rule in rules {
        if rule.direction == sending && rule.required {
            let own = table.kind(rule.kind)?;
            let mut accepted = false;
            for term in terms {
                if term.kind == rule.kind && term.largest >= own.largest {
                    accepted = true;
                }
            }
            if !accepted {
                return Some(rule.kind);
            }
        }
    }
    None
}

impl KindRule {
    /// The wire format version this codec writes and reads.
    pub const FORMAT_VERSION: u64 = 5;
}

#[cfg(test)]
mod optional_tests {
    use super::{Body, KindRule, required_terms_gap};
    use skein_channel::{Direction, Kind, Role, Schema, Term, Version};
    use skein_lib::List;

    #[test]
    fn an_older_peer_may_leave_a_test_only_optional_kind_out() {
        let rules = [
            KindRule { kind: 0x0100, direction: Direction::FromInitiator, required: true, body: Body::Start },
            KindRule { kind: 0x0101, direction: Direction::FromInitiator, required: false, body: Body::Message },
            KindRule { kind: 0x0102, direction: Direction::FromResponder, required: true, body: Body::Answer },
            KindRule { kind: 0x0103, direction: Direction::FromResponder, required: false, body: Body::Fact },
        ];
        let mut kinds = List::with_capacity(4);
        for rule in rules {
            kinds.push(Kind { kind: rule.kind, direction: rule.direction, largest: 32 }).expect("four test kinds");
        }
        let mut versions = List::with_capacity(1);
        versions.push(Version { version: 5, kinds }).expect("one test version");
        let schema = Schema { magic: *b"smth", versions };
        let mut host_terms = List::with_capacity(1);
        host_terms.push(Term { kind: 0x0100, largest: 32 }).expect("one required kind");
        assert_eq!(required_terms_gap(&schema, Role::Initiator, 5, &host_terms, &rules), None);
        assert_eq!(required_terms_gap(&schema, Role::Initiator, 5, &List::with_capacity(0), &rules), Some(0x0100));
        let mut agent_terms = List::with_capacity(1);
        agent_terms.push(Term { kind: 0x0102, largest: 32 }).expect("one required kind");
        assert_eq!(required_terms_gap(&schema, Role::Responder, 5, &agent_terms, &rules), None);
        assert_eq!(required_terms_gap(&schema, Role::Responder, 5, &List::with_capacity(0), &rules), Some(0x0102));
    }
}
