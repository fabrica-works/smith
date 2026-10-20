//! Check the full bound fixture without allocating a JSON value tree per message.

use serde::Serialize;
use skein_llm as shared;

#[derive(Serialize)]
struct Message<'a> {
    role: &'a str,
    blocks: [Block<'a>; 1],
}

#[derive(Serialize)]
struct Block<'a> {
    #[serde(rename = "type")]
    kind: &'a str,
    text: &'a str,
}

pub(super) fn assert_native_prompt_line(
    line: &[u8],
    native: &shared::Prompt,
    text: &[u8],
    credential: &shared::Credential,
) {
    // Every source message is identical. Encode its expected role, block kind
    // and complete text once with an independent JSON implementation, then copy
    // those bytes for every admitted message. Exact comparison checks all the
    // same fields and counts without decoding repeated payloads or retaining
    // a tree of JSON objects at the maximum history size.
    let text = std::str::from_utf8(text).expect("native admitted UTF8");
    let message = serde_json::to_vec(&Message { role: "user", blocks: [Block { kind: "text", text }] })
        .expect("independent expected message");
    let mut expected = br#""prompt":{"system":"native system","tools":[],"messages":["#.to_vec();
    for index in 0..native.messages.len() {
        if index != 0 {
            expected.push(b',');
        }
        expected.extend_from_slice(&message);
    }
    expected.extend_from_slice(b"]}}\n");
    let metadata = line
        .strip_suffix(expected.as_slice())
        .expect("native system, every source message, user role, single text block and every escaped payload survive");
    // The matched suffix is valid JSON by construction. Parse the remaining
    // metadata with a null prompt to check the entire enclosing document too.
    let mut document = metadata.to_vec();
    document.extend_from_slice(b"\"prompt\":null}\n");
    let decoded: serde_json::Value = serde_json::from_slice(&document).expect("independent whole-line JSON");
    assert_eq!(decoded.get("prompt"), Some(&serde_json::Value::Null), "prompt belongs to the root record");
    let encoded = std::str::from_utf8(line).expect("JSON UTF8");
    assert!(
        !encoded.contains(std::str::from_utf8(&credential.value).expect("fixture access UTF8")),
        "grant access bytes never enter content"
    );
    assert!(
        !encoded.contains(std::str::from_utf8(&credential.account_id).expect("fixture account UTF8")),
        "grant account bytes never enter content"
    );
}
