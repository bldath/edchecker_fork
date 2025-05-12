pub trait Solution {
    pub fn get_eo() -> Vec<(Idx, Idx)>;
}

pub struct EmptySolution;
impl Solution for EmptySolution {
    fn get_eo() -> Vec<(Idx, Idx)> {
        vec![]
    }
}

pub trait Checker {
    fn new(rr: &ReadResult) -> Self;
    fn check(&self) -> Result<dyn Solution, Box<dyn Error>>;
}