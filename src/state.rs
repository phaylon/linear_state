
type ResourceVec<T> = smallvec::SmallVec<[T; 2]>;

#[derive(Debug)]
pub struct RootState {
    data: crate::slot::RootMap,
}

impl RootState {

    pub fn new() -> Self {
        Self {
            data: crate::slot::RootMap::new(),
        }
    }

    pub fn add<R>(&mut self, resource: R)
    where
        R: 'static,
    {
        self.data.get_mut_or_default::<ResourceVec<R>>().push(resource);
    }

    pub fn add_many<R, I>(&mut self, resource_iter: I)
    where
        R: 'static,
        I: IntoIterator<Item = R>,
    {
        for resource in resource_iter.into_iter() {
            self.add(resource);
        }
    }

    pub fn with<R>(mut self, resource: R) -> Self
    where
        R: 'static,
    {
        self.add(resource);
        self
    }

    pub fn with_many<R, I>(mut self, resource_iter: I) -> Self
    where
        R: 'static,
        I: IntoIterator<Item = R>,
    {
        self.add_many(resource_iter);
        self
    }

    pub fn finalize(self) -> State {
        State {
            data: std::rc::Rc::new(self.data.into_node()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct State {
    data: std::rc::Rc<crate::slot::SlotNode>,
}

#[derive(Debug)]
struct Trace<T> {
    value: T,
    previous: Option<std::rc::Rc<Trace<T>>>,
}

#[derive(Debug)]
pub struct TraceIter<'s, T> {
    current: Option<&'s std::rc::Rc<Trace<T>>>,
}

impl<'s, T> Iterator for TraceIter<'s, T> {
    type Item = &'s T;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.current.map(|trace| &trace.value);
        self.current = self.current.and_then(|trace| trace.previous.as_ref());
        current
    }
}

impl State {

    pub fn flatten(&self) -> Self {
        Self {
            data: std::rc::Rc::new(self.data.flatten()),
        }
    }

    pub fn trace<T>(&self) -> TraceIter<'_, T>
    where
        T: 'static,
    {
        TraceIter {
            current: self.first::<std::rc::Rc<Trace<T>>>(),
        }
    }

    pub fn with_trace<T>(&self, value: T) -> Self
    where
        T: 'static,
    {
        let current =
            if let Some(prev) = self.first::<std::rc::Rc<Trace<T>>>() {
                Trace {
                    value,
                    previous: Some(prev.clone()),
                }
            } else {
                Trace {
                    value,
                    previous: None,
                }
            };
        let trace_resource: ResourceVec<std::rc::Rc<Trace<T>>> =
            smallvec::smallvec![std::rc::Rc::new(current)];
        let new_data = self.data.child_with_override(trace_resource);
        State {
            data: std::rc::Rc::new(new_data),
        }
    }

    pub fn first<R>(&self) -> Option<&R>
    where
        R: 'static,
    {
        self.get().first()
    }

    pub fn has<R>(&self, resource: &R) -> bool
    where
        R: Eq + 'static,
    {
        self.get().contains(resource)
    }

    pub fn get<R>(&self) -> &[R]
    where
        R: 'static,
    {
        self.data.get::<ResourceVec<R>>()
            .map(|values| values.as_slice())
            .unwrap_or_else(|| &[])
    }

    pub fn descend_mapped_filtered<R, RF, MF, F>(
        &self,
        mut include: RF,
        mut mapper: MF,
        mut callback: F,
    )
    where
        R: Clone + 'static,
        RF: FnMut(&R) -> bool,
        MF: FnMut(R) -> R,
        F: FnMut(&State, &R),
    {
        let original = self.get::<R>();
        for index in 0..original.len() {
            if include(&original[index]) {
                let mut new_resources = original.iter().cloned().collect::<ResourceVec<R>>();
                let removed = new_resources.swap_remove(index);
                let mapped = mapper(removed);
                new_resources.push(mapped);
                let new_data = self.data.child_with_override(new_resources);
                let new_state = State {
                    data: std::rc::Rc::new(new_data),
                };
                let resources = new_state.get::<R>();
                callback(&new_state, resources.last().expect("just inserted element"));
            }
        }
    }

    pub fn descend_mapped<R, MF, F>(
        &self,
        mapper: MF,
        callback: F,
    )
    where
        R: Clone + 'static,
        MF: FnMut(R) -> R,
        F: FnMut(&State, &R),
    {
        self.descend_mapped_filtered(|_| true, mapper, callback);
    }

    pub fn descend_consumed_filtered<R, RF, F>(
        &self,
        mut include: RF,
        mut callback: F,
    )
    where
        R: Clone + 'static,
        RF: FnMut(&R) -> bool,
        F: FnMut(&State, R),
    {
        let original = self.get::<R>();
        for index in 0..original.len() {
            if include(&original[index]) {
                let mut new_resources = original.iter().cloned().collect::<ResourceVec<R>>();
                let removed = new_resources.swap_remove(index);
                let new_data = self.data.child_with_override(new_resources);
                let new_state = State {
                    data: std::rc::Rc::new(new_data),
                };
                callback(&new_state, removed);
            }
        }
    }

    pub fn descend_consumed<R, F>(&self, callback: F)
    where
        R: Clone + 'static,
        F: FnMut(&State, R),
    {
        self.descend_consumed_filtered(|_| true, callback);
    }

    pub fn with_produced<R, I>(&self, source: I) -> Self
    where
        R: Clone + 'static,
        I: IntoIterator<Item = R>,
    {
        let source_iter = source.into_iter();
        let mut new_resources = self.get::<R>().iter().cloned().collect::<ResourceVec<R>>();
        new_resources.extend(source_iter);
        let new_data = self.data.child_with_override(new_resources);
        State {
            data: std::rc::Rc::new(new_data),
        }
    }
}
