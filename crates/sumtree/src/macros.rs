#[allow(unused_macros)]
macro_rules! branch {
    ($colour:expr, $left:expr, $right:expr $(,)?) => {{
        crate::SumTree::new_branch($colour, $left, $right)
    }};
}

#[allow(unused_macros)]
macro_rules! branch_r {
    ($left:expr, $right:expr $(,)?) => {{
        branch!(crate::Colour::Red, $left, $right)
    }};
}

#[allow(unused_macros)]
macro_rules! branch_b {
    ($left:expr, $right:expr $(,)?) => {{
        branch!(crate::Colour::Black, $left, $right)
    }};
}

#[allow(unused_macros)]
macro_rules! leaf {
    ($item:expr) => {{
        crate::SumTree::new_leaf($item)
    }};
}

#[allow(unused_imports)]
pub(super) use {branch, branch_b, branch_r, leaf};
