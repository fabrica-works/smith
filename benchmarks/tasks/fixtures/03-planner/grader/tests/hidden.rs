use fixture::{Error, Node, plan};

fn n(name: &str, dependencies: &[&str]) -> Node {
    Node { name: name.into(), dependencies: dependencies.iter().map(|s| (*s).into()).collect() }
}
fn strings(names: &[&str]) -> Vec<String> {
    names.iter().map(|s| (*s).into()).collect()
}

#[test]
fn closure_diamond_and_duplicates() {
    let nodes = vec![n("z", &["b", "c", "b"]), n("c", &["a"]), n("b", &["a"]), n("a", &[]), n("unused", &[])];
    assert_eq!(plan(&nodes, &strings(&["z", "z"])), Ok(strings(&["a", "b", "c", "z"])));
}

#[test]
fn newly_ready_node_precedes_old_ready_node() {
    let nodes = vec![n("a", &["b"]), n("b", &[]), n("z", &[])];
    assert_eq!(plan(&nodes, &strings(&["a", "z"])), Ok(strings(&["b", "a", "z"])));
}

#[test]
fn global_validation_and_priority() {
    assert_eq!(plan(&[n("a", &["missing"]), n("a", &[])], &[]), Err(Error::Duplicate("a".into())));
    assert_eq!(
        plan(&[n("a", &[]), n("unused", &["missing"])], &strings(&["bad"])),
        Err(Error::MissingDependency { node: "unused".into(), dependency: "missing".into() })
    );
}

#[test]
fn selected_cycle_reports_downstream_blockage() {
    let nodes = vec![n("z", &["b"]), n("b", &["a"]), n("a", &["b"]), n("ok", &[])];
    assert_eq!(plan(&nodes, &strings(&["z", "ok"])), Err(Error::Cycle(strings(&["a", "b", "z"]))));
    assert_eq!(plan(&nodes, &strings(&["ok"])), Ok(strings(&["ok"])));
    assert_eq!(plan(&nodes, &[]), Ok(vec![]));
}

#[test]
fn self_cycle() {
    assert_eq!(plan(&[n("a", &["a"])], &strings(&["a"])), Err(Error::Cycle(strings(&["a"]))));
}

#[test]
fn deep_chain_and_input_permutations() {
    let nodes: Vec<Node> = (0..10000)
        .map(|i| Node {
            name: format!("n{i:05}"),
            dependencies: if i == 0 { vec![] } else { vec![format!("n{:05}", i - 1)] },
        })
        .collect();
    let got = plan(&nodes, &["n09999".into()]).unwrap();
    assert_eq!(got.len(), 10000);
    for (i, name) in got.iter().enumerate() {
        assert_eq!(name, &format!("n{i:05}"));
    }
    let mut reversed = nodes.clone();
    reversed.reverse();
    assert_eq!(plan(&reversed, &["n09999".into()]).unwrap(), got);
}

// Extended adversarial validation

#[test]
fn validation_keeps_input_order() {
    let nodes = vec![n("z", &["second", "first"]), n("a", &["other"])];
    assert_eq!(plan(&nodes, &[]), Err(Error::MissingDependency { node: "z".into(), dependency: "second".into() }));
    assert_eq!(plan(&[n("a", &[])], &strings(&["z", "y"])), Err(Error::UnknownTarget("z".into())));
    assert_eq!(plan(&[n("z", &[]), n("a", &[]), n("z", &[]), n("a", &[])], &[]), Err(Error::Duplicate("z".into())));
}

fn advance(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

fn random_dag(state: &mut u64) -> Vec<Vec<usize>> {
    let mut dependencies = vec![Vec::new(); 16];
    for (node, list) in dependencies.iter_mut().enumerate() {
        for dependency in 0..node {
            if advance(state) % 4 == 0 {
                list.push(dependency);
            }
        }
    }
    dependencies
}

fn ready_scan(dependencies: &[Vec<usize>], labels: &[usize], targets: &[usize]) -> Vec<String> {
    let mut selected = [false; 16];
    let mut remaining = targets.to_vec();
    while let Some(node) = remaining.pop() {
        if !selected[node] {
            selected[node] = true;
            remaining.extend(&dependencies[node]);
        }
    }
    let mut done = [false; 16];
    let mut expected = Vec::new();
    loop {
        let next = (0..16)
            .filter(|&node| selected[node] && !done[node] && dependencies[node].iter().all(|&dep| done[dep]))
            .min_by_key(|&node| labels[node]);
        match next {
            None => break,
            Some(node) => {
                done[node] = true;
                expected.push(format!("n{:02}", labels[node]));
            }
        }
    }
    expected
}

#[test]
fn random_dags_match_independent_ready_scan() {
    for seed in 1..129u64 {
        let mut state = seed;
        let dependencies = random_dag(&mut state);
        let mut labels: Vec<_> = (0..16).collect();
        for index in (1..16).rev() {
            labels.swap(index, (advance(&mut state) % (index as u64 + 1)) as usize);
        }
        let targets = [15, 7];
        let expected = ready_scan(&dependencies, &labels, &targets);
        let mut nodes: Vec<_> = (0..16)
            .map(|node| Node {
                name: format!("n{:02}", labels[node]),
                dependencies: dependencies[node].iter().map(|&dep| format!("n{:02}", labels[dep])).collect(),
            })
            .collect();
        nodes.reverse();
        assert_eq!(plan(&nodes, &targets.map(|node| format!("n{:02}", labels[node]))), Ok(expected), "seed {seed}");
    }
}
