use fixture::{Error, Node, plan};

#[test]
fn independent_targets_are_sorted() {
    let nodes = vec![Node { name: "b".into(), dependencies: vec![] }, Node { name: "a".into(), dependencies: vec![] }];
    assert_eq!(plan(&nodes, &["b".into(), "a".into(), "b".into()]), Ok(vec!["a".into(), "b".into()]));
}

#[test]
fn unknown_target() {
    assert_eq!(plan(&[], &["x".into()]), Err(Error::UnknownTarget("x".into())));
}
