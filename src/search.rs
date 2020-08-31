
pub fn search<F, G, C>(
    mut current: Vec<crate::State>,
    mut advance: F,
    mut goal: G,
    mut control: C,
) -> Vec<crate::State>
where
    F: FnMut(&crate::State, &mut SearchCollector<'_>),
    G: FnMut(&crate::State) -> bool,
    C: FnMut(&mut SearchState<'_>),
{
    let mut solutions = Vec::new();
    let mut next = Vec::new();
    let mut depth = 0;
    loop {
        depth += 1;
        next.clear();
        if current.is_empty() {
            return Vec::new();
        }
        for state in &current {
            advance(state, &mut SearchCollector::wrap(&mut next));
        }
        for state in &next {
            if goal(state) {
                solutions.push(state.clone());
            }
        }
        if !solutions.is_empty() {
            return solutions;
        }
        control(&mut SearchState::wrap(&mut next, depth));
        std::mem::swap(&mut next, &mut current);
    }
}

#[derive(Debug)]
pub struct SearchState<'a> {
    depth: u64,
    container: &'a mut Vec<crate::State>,
}

impl<'a> SearchState<'a> {

    fn wrap(
        container: &'a mut Vec<crate::State>,
        depth: u64,
    ) -> Self {
        Self { container, depth }
    }

    pub fn count(&self) -> usize {
        self.container.len()
    }

    pub fn states(&self) -> &[crate::State] {
        self.container
    }

    pub fn depth(&self) -> u64 {
        self.depth
    }

    pub fn clear(&mut self) {
        self.container.clear();
    }

    pub fn truncate(&mut self, max: usize) {
        self.container.truncate(max);
    }

    pub fn shuffle(&mut self) {
        use rand::seq::{SliceRandom};

        self.container.shuffle(&mut rand::thread_rng());
    }

    pub fn retain<F>(&mut self, condition: F)
    where
        F: FnMut(&crate::State) -> bool,
    {
        self.container.retain(condition);
    }
}

#[derive(Debug)]
pub struct SearchCollector<'a> {
    container: &'a mut Vec<crate::State>,
}

impl<'a> SearchCollector<'a> {

    fn wrap(container: &'a mut Vec<crate::State>) -> Self {
        Self { container }
    }

    pub fn push(&mut self, state: crate::State) {
        self.container.push(state);
    }
}
