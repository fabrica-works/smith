Completed and children are trimmed archived JSON lines from codex-cli 0.160.0. The manifest pins each original file and lists its retained lines. Only records were removed; retained content is unchanged. The children run used three threads according to its archived result, but its collaboration wait has no receiver ids. Its JSON-line usage is root-only. Child, compaction and helper usage cannot be recovered from this stream.

Increment 00.10 adds actual isolated-home records from 2026-10-09. The former
synthetic failed SDK terminal is replaced by the native unknown-model failure;
the synthetic repeated SDK terminal is replaced by an actual rollout with
per-response, turn and thread usage fields. `live-children.jsonl` and its
root/child rollouts come from the completed `child-03` control. Metadata
projections omit user/account identifiers and base instructions; retained
usage and stdout lines are unchanged. Full originals remain in the private
run archives named in manifest.json. Credential assignment, JWT, bearer,
API-token and private-key scans found no matches in committed records.

`repository-04-time.jsonl` projects timing fields from the root rollout of
`repository-a64f0523-20261013T230741Z/04`. It retains timestamps, record types,
response and call identities, tool names and command durations. Prompts,
commands and outputs are omitted. The sibling `../smith-time.jsonl` retains
matched event spans from attempt 01 of that run, omitting optional call input,
request prompts and response completion bodies, with the events version projected
to the current codec.

The three `repository-04-{root,child-one,child-two}.jsonl` projections retain
native parent metadata, per-response usage and cumulative `token_count` usage
from that repository attempt. Repeated cumulative usage describes the same
response; the 62 root and 28 second-child notifications contain 55 and 27
responses. The older live root/child projections also retain their native
token-count totals for the same count check.
