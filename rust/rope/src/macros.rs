#[allow(unused_macros)]
macro_rules! branch {
    ($colour:expr, $left:expr, $right:expr $(,)?) => {{
        std::sync::Arc::new(crate::rope::tree::Node::new_branch($colour, $left, $right))
    }};
}

#[allow(unused_macros)]
macro_rules! branch_r {
    ($left:expr, $right:expr $(,)?) => {{
        branch!(crate::rope::tree::NodeColour::Red, $left, $right)
    }};
}

#[allow(unused_macros)]
macro_rules! branch_b {
    ($left:expr, $right:expr $(,)?) => {{
        branch!(crate::rope::tree::NodeColour::Black, $left, $right)
    }};
}

#[allow(unused_macros)]
macro_rules! leaf_e {
    ($buffer:ident, $size:literal) => {{
        let mut i = 0;
        loop {
            i += 1;
            let (block, written) = $buffer.append(&[0; $size]).expect("block append");
            if written == $size {
                break std::sync::Arc::new(crate::rope::tree::Node::new_leaf(block));
            }
            if i == 10 {
                unreachable!()
            }
        }
    }};
}

#[allow(unused_macros)]
macro_rules! leaf {
    ($buffer:ident, $val:expr) => {{
        let mut i = 0;
        loop {
            i += 1;
            let (block, written) = $buffer.append($val).expect("block append");
            if written == $val.len() {
                break std::sync::Arc::new(crate::rope::tree::Node::new_leaf(block));
            }
            if i == 10 {
                unreachable!()
            }
        }
    }};
}

#[allow(unused_imports)]
pub(super) use {branch, branch_b, branch_r, leaf, leaf_e};
