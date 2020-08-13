#[derive(Clone)]
pub struct ObjHeap {
    heap: Vec<Obj>,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ObjPointer(usize);

#[derive(Clone, Eq, PartialEq)]
pub struct Obj {
    pub kind: ObjKind,
}

#[derive(Clone, Eq, PartialEq)]
pub enum ObjKind {
    String(String),
}

impl ObjHeap {
    pub fn new() -> ObjHeap {
        ObjHeap {
            heap: Vec::with_capacity(256),
        }
    }

    pub fn copy_string(&mut self, str: &str) -> ObjPointer {
        self.allocate_string(str.to_owned())
    }

    pub fn take_string(&mut self, str: String) -> ObjPointer {
        self.allocate_string(str)
    }

    fn allocate_string(&mut self, str: String) -> ObjPointer {
        self.allocate_obj(ObjKind::String(str))
    }

    fn allocate_obj(&mut self, kind: ObjKind) -> ObjPointer {
        self.heap.push(Obj { kind });
        ObjPointer(self.heap.len() - 1)
    }
}

impl ObjPointer {
    pub fn borrow<'a>(&self, heap: &'a ObjHeap) -> &'a Obj {
        heap.heap.get(self.0).expect("Dangling pointer")
    }
}

impl Obj {
    pub fn to_string(&self) -> String {
        match &self.kind {
            ObjKind::String(inner) => inner.clone(),
        }
    }
}
