use std::rc::Rc;

pub struct CommitNode {
    pub id: String,
    pub parents: Vec<Rc<CommitNode>>,
}

impl CommitNode {
    pub fn create(commit: git2::Commit, commits: &mut Vec<Rc<CommitNode>>) -> Rc<Self> {
        let mut result = CommitNode { id: commit.id().to_string(), parents: Vec::new() };
        for parent in commit.parents() {
            let commit = CommitNode::create(parent, commits);
            result.parents.push(commit);
        }

        let reference = Rc::new(result);
        commits.push(Rc::clone(&reference));
        reference
    }
}
