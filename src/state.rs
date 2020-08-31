
type ResourceVec<T> = smallvec::SmallVec<[T; 2]>;

#[derive(Debug)]
struct StateData {
    parent: Option<std::rc::Rc<StateData>>,
    slots: type_map::TypeMap,
}

impl StateData {

    fn root() -> Self {
        Self {
            parent: None,
            slots: type_map::TypeMap::new(),
        }
    }

    fn get<R>(&self) -> &[R]
    where
        R: 'static,
    {
        if let Some(values) = self.slots.get::<ResourceVec<R>>() {
            values
        } else if let Some(parent) = &self.parent {
            parent.get()
        } else {
            &[]
        }
    }

    fn push<R>(&mut self, value: R)
    where
        R: 'static,
    {
        self.slots.entry::<ResourceVec<R>>().or_insert_with(ResourceVec::new).push(value);
    }

    fn child_with_override<R>(self: &std::rc::Rc<Self>, values: ResourceVec<R>) -> Self
    where
        R: 'static,
    {
        let mut slots = type_map::TypeMap::new();
        slots.insert(values);
        Self {
            parent: Some(self.clone()),
            slots,
        }
    }
}

#[derive(Debug)]
pub struct RootState {
    data: StateData,
}

impl RootState {

    pub fn new() -> Self {
        Self {
            data: StateData::root(),
        }
    }

    pub fn add<R>(&mut self, resource: R)
    where
        R: 'static,
    {
        self.data.push(resource);
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
            data: std::rc::Rc::new(self.data),
        }
    }
}

#[derive(Debug, Clone)]
pub struct State {
    data: std::rc::Rc<StateData>,
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
        let new_data = self.data
            .child_with_override(smallvec::smallvec![std::rc::Rc::new(current)]);
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
        self.data.get().contains(resource)
    }

    pub fn get<R>(&self) -> &[R]
    where
        R: 'static,
    {
        self.data.get()
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
        let original = self.data.get::<R>();
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
                callback(&new_state, &resources[resources.len()-1]);
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
        let original = self.data.get::<R>();
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
        let mut new_resources = self.data.get::<R>().iter().cloned().collect::<ResourceVec<R>>();
        new_resources.extend(source_iter);
        let new_data = self.data.child_with_override(new_resources);
        State {
            data: std::rc::Rc::new(new_data),
        }
    }
}

