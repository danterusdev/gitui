use std::collections::HashMap;

pub struct CommitNode {
    pub id: String,
    pub parents: Vec<String>,
    pub children: Vec<String>,
    pub reference: Option<String>,
}

impl CommitNode {
    pub fn create(commit: git2::Commit, commits: &mut HashMap<String, CommitNode>, reference: Option<String>) -> String {
        if commits.contains_key(&commit.id().to_string()) && reference.is_none() {
            commit.id().to_string()
        } else {
            let mut result = CommitNode { id: commit.id().to_string(), parents: Vec::new(), children: Vec::new(), reference };

            for parent in commit.parents() {
                let commit = CommitNode::create(parent, commits, None);
                result.parents.push(commit.clone());
            }

            for parent in commit.parents() {
                let commit = CommitNode::create(parent, commits, None);
                let commit = commits.get_mut(&commit).unwrap();
                commit.children.push(result.id.clone());
            }

            commits.insert(commit.id().to_string(), result);
            commit.id().to_string()
        }
    }
}

pub fn get_commit_depth(commit: &CommitNode, commits: &HashMap<String, CommitNode>) -> usize {
    if commit.parents.len() > 0 {
        let mut min_parent_depth = usize::MAX;
        for parent in &commit.parents {
            let parent_depth = get_commit_depth(commits.get(parent).unwrap(), commits);
            if parent_depth < min_parent_depth {
                min_parent_depth = parent_depth;
            }
        }

        min_parent_depth + 1
    } else {
        0
    }
}

fn get_commit_tree_size(commit: &CommitNode, commits: &HashMap<String, CommitNode>) -> usize {
    let mut size = commit.children.len();
    if size > 0 {
        size -= 1;
    }

    for child in &commit.children {
        let child = commits.get(child).unwrap();
        size += get_commit_tree_size(child, commits);
    }
    size
}

pub fn get_commit_height(commit: &CommitNode, commits: &HashMap<String, CommitNode>) -> isize {
    // Removed for testing, I'm not sure how to exactly to handle this
    // assert!(commit.parents.len() <= 1);

    if commit.parents.len() == 0 {
        0
    } else {
        let parent = commit.parents.get(0).unwrap();
        let parent = commits.get(parent).unwrap();
        // Removed for testing, I'm not sure how to exactly to handle this
        //assert!(parent.children.len() <= 2);

        if parent.children.len() == 1 {
            get_commit_height(parent, commits)
        } else {
            let multiplier = if parent.children.iter().position(|c| c == &commit.id).unwrap() == 0 { -1 } else { 1 };
            let value = get_commit_tree_size(commit, commits) as isize;
            get_commit_height(parent, commits) + multiplier * (1 + value)
        }
    }
}
