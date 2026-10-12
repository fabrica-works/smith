use crate::graph::{Error, Node};
use std::collections::{BTreeMap, BTreeSet};

pub fn plan(nodes: &[Node], targets: &[String]) -> Result<Vec<String>, Error> {
    let mut by_name = BTreeMap::new();
    for node in nodes {
        if by_name.insert(node.name.clone(), node).is_some() {
            return Err(Error::Duplicate(node.name.clone()));
        }
    }
    for node in nodes {
        for dependency in &node.dependencies {
            if !by_name.contains_key(dependency) {
                return Err(Error::MissingDependency {
                    node: node.name.clone(),
                    dependency: dependency.clone(),
                });
            }
        }
    }
    for target in targets {
        if !by_name.contains_key(target) {
            return Err(Error::UnknownTarget(target.clone()));
        }
    }
    let mut selected = BTreeSet::new();
    let mut stack = targets.to_vec();
    while let Some(name) = stack.pop() {
        if selected.insert(name.clone()) {
            stack.extend(by_name[&name].dependencies.iter().cloned());
        }
    }
    let mut indegree = BTreeMap::new();
    let mut dependents: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut ready = BTreeSet::new();
    for name in &selected {
        let dependencies: BTreeSet<_> = by_name[name].dependencies.iter().cloned().collect();
        indegree.insert(name.clone(), dependencies.len());
        if dependencies.is_empty() {
            ready.insert(name.clone());
        }
        for dep in dependencies {
            dependents.entry(dep).or_default().insert(name.clone());
        }
    }
    let mut result = Vec::new();
    while let Some(name) = ready.pop_first() {
        result.push(name.clone());
        if let Some(children) = dependents.get(&name) {
            for child in children {
                let count = indegree.get_mut(child).unwrap();
                *count -= 1;
                if *count == 0 {
                    ready.insert(child.clone());
                }
            }
        }
    }
    if result.len() != selected.len() {
        let scheduled: BTreeSet<_> = result.iter().cloned().collect();
        return Err(Error::Cycle(
            selected.difference(&scheduled).cloned().collect(),
        ));
    }
    Ok(result)
}
