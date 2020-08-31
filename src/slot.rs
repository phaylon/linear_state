
#[derive(Debug)]
pub enum SlotNode {
    Base(SlotMap),
    Node(std::rc::Rc<dyn std::any::Any>, std::rc::Rc<SlotNode>),
}

impl SlotNode {

    pub fn flatten(&self) -> Self {
        let mut current = self;
        let mut flat = fxhash::FxHashMap::default();
        loop {
            match *current {
                SlotNode::Base(ref basemap) => {
                    for (id, value) in &basemap.map {
                        if !flat.contains_key(id) {
                            flat.insert(*id, value.clone());
                        }
                    }
                    return SlotNode::Base(SlotMap { map: flat });
                },
                SlotNode::Node(ref value, ref parent) => {
                    if !flat.contains_key(&value.type_id()) {
                        flat.insert(value.type_id(), value.clone());
                    }
                    current = parent;
                },
            }
        }
    }

    pub fn child_with_override<T>(self: &std::rc::Rc<Self>, value: T) -> Self
    where
        T: 'static,
    {
        SlotNode::Node(std::rc::Rc::new(value), self.clone())
    }

    pub fn get<T>(&self) -> Option<&T>
    where
        T: 'static,
    {
        let id = std::any::TypeId::of::<T>();
        let mut current = self;
        loop {
            match *current {
                SlotNode::Base(ref basemap) => {
                    return basemap
                        .map
                        .get(&id)
                        .map(|value| value.downcast_ref().expect("correct associated value type"));
                },
                SlotNode::Node(ref value, ref parent) => {
                    if let Some(value) = value.downcast_ref() {
                        return Some(value);
                    } else {
                        current = parent;
                    }
                },
            }
        }
    }
}

#[derive(Debug)]
pub struct SlotMap {
    map: fxhash::FxHashMap<std::any::TypeId, std::rc::Rc<dyn std::any::Any>>,
}

#[derive(Debug)]
pub struct RootMap {
    map: fxhash::FxHashMap<std::any::TypeId, Box<dyn std::any::Any>>,
}

impl RootMap {

    pub fn new() -> Self {
        Self {
            map: fxhash::FxHashMap::default(),
        }
    }

    pub fn get_mut_or_default<T>(&mut self) -> &mut T
    where
        T: 'static + Default,
    {
        self.map
            .entry(std::any::TypeId::of::<T>())
            .or_insert_with(|| Box::new(T::default()))
            .downcast_mut::<T>()
            .expect("existing or defauled slot value")
    }

    pub fn into_node(self) -> SlotNode {
        let mut map = fxhash::FxHashMap::default();
        for (key, value) in self.map {
            map.insert(key, value.into());
        }
        SlotNode::Base(SlotMap { map })
    }
}
