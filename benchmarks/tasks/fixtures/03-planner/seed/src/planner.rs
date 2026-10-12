use crate::graph::{Error, Node};

pub fn plan(nodes: &[Node], targets: &[String]) -> Result<Vec<String>, Error> {
    let mut result = Vec::new();
    for target in targets {
        if !nodes.iter().any(|node| &node.name == target) {
            return Err(Error::UnknownTarget(target.clone()));
        }
        if !result.contains(target) {
            result.push(target.clone());
        }
    }
    result.sort();
    Ok(result)
}
