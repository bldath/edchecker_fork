
use std::collections::HashMap;

use crate::model::Argument;


struct Guesser {
    target: (Argument, Argument, Argument),
    state: i32,
    rest: Option<Box<Guesser>>,

}


impl Guesser {
    fn next(&mut self) -> Option<(Argument, Argument, Argument)> {
        if self.state == 0 {
            self.state = 1;
            return Some((self.target.0, self.target.1, self.target.2));
        }

        if self.state == 1 {
            if let Some(r) = &mut self.rest {
                if let Some(q) = r.next() {
                    return Some(q);
                }
            }
            return Some((self.target.0, self.target.1, self.target.2));
        }
    }

    fn reset(&mut self) {
        self.state = 0;
        if let Some(r) = &mut self.rest {
            r.reset();
        }
    }
}
