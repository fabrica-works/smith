# Incremental byte line decoder with bounded recovery

The line decoder currently treats each `push` chunk as a complete set of
lines. Make it truly incremental while retaining the public API.
Lines terminate at LF. Remove exactly one CR immediately before LF;
all other CR bytes are payload. Output `Line(Vec<u8>)`, including empty
lines, only when LF arrives. `finish` emits a final nonempty unterminated
payload (without stripping a final CR), then resets; repeated finish is
empty and a decoder can be reused afterward.
The maximum is the payload byte count, excluding LF and excluding a CR
only when that CR is immediately before LF. A payload exactly at the
maximum is accepted. On overflow emit exactly one `TooLong` for that
line, then discard through its LF (or finish); subsequent lines recover.
An overflow may be emitted as soon as it is certain, but must not depend
on chunk boundaries. Keep retained buffering at most max_payload + 1
bytes; in particular never buffer an entire arbitrarily oversized chunk.
Handle max_payload = 0, arbitrary byte values, CRLF split across chunks,
and mixed empty/normal/oversized lines. Add tests and run the suite.
