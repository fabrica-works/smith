#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub name: String,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Duplicate(String),
    MissingDependency { node: String, dependency: String },
    UnknownTarget(String),
    Cycle(Vec<String>),
}
