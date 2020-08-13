pub struct ObjectHeap {
    heap: Vec<Obj>,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ObjPointer(usize);

pub struct Obj {
    kind: ObjKind,
}

pub enum ObjKind {
    String(String),
}

impl ObjectHeap {
    pub fn new() -> ObjectHeap {
        ObjectHeap {
            heap: Vec::with_capacity(256),
        }
    }

    pub fn copy_string(&mut self, str: &str) -> ObjPointer {
        self.allocate_string(str.to_owned())
    }

    fn allocate_string(&mut self, str: String) -> ObjPointer {
        self.allocate_obj(ObjKind::String(str))
    }

    fn allocate_obj(&mut self, kind: ObjKind) -> ObjPointer {
        self.heap.push(Obj { kind });
        ObjPointer(self.heap.len() - 1)
    }
}
