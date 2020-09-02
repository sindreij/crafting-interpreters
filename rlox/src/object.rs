use crate::chunk::Chunk;
use std::collections::HashMap;

#[derive(Clone)]
pub struct ObjHeap {
    heap: Vec<Obj>,
    strings: HashMap<String, ObjPointer>,
}

#[derive(Copy, Clone, Debug, PartialEq, Hash, Eq)]
pub struct ObjPointer(usize);

#[derive(Clone, PartialEq)]
pub struct Obj {
    pub kind: ObjKind,
}

#[derive(Clone, PartialEq)]
pub enum ObjKind {
    String(String),
    Function(ObjFunction),
}

#[derive(Clone, PartialEq)]
pub struct ObjFunction {
    pub arity: usize,
    pub chunk: Chunk,
    pub name: Option<String>,
}

impl ObjHeap {
    pub fn new() -> ObjHeap {
        ObjHeap {
            heap: Vec::with_capacity(256),
            strings: HashMap::new(),
        }
    }

    pub fn copy_string(&mut self, str: &str) -> ObjPointer {
        if let Some(interned) = self.strings.get(str) {
            return *interned;
        }
        self.allocate_string(str.to_owned())
    }

    pub fn take_string(&mut self, str: String) -> ObjPointer {
        if let Some(interned) = self.strings.get(&str) {
            return *interned;
        }
        self.allocate_string(str)
    }

    fn allocate_string(&mut self, str: String) -> ObjPointer {
        let ptr = self.allocate_obj(ObjKind::String(str.clone()));
        self.strings.insert(str, ptr);
        ptr
    }

    pub fn allocate_obj(&mut self, kind: ObjKind) -> ObjPointer {
        self.heap.push(Obj { kind });
        ObjPointer(self.heap.len() - 1)
    }
}

impl ObjPointer {
    pub fn borrow<'a>(&self, heap: &'a ObjHeap) -> &'a Obj {
        heap.heap.get(self.0).expect("Dangling pointer")
    }

    pub fn to_string(&self, heap: &ObjHeap) -> String {
        format!("{} ({})", self.borrow(heap).to_string(), self.0)
    }
}

impl Obj {
    pub fn to_string(&self) -> String {
        match &self.kind {
            ObjKind::String(inner) => inner.clone(),
            ObjKind::Function(inner) => {
                format!("<fn {}>", inner.name.as_deref().unwrap_or("<script>"))
            }
        }
    }

    pub fn new_function(&self) -> Obj {
        Obj {
            kind: ObjKind::Function(ObjFunction::new()),
        }
    }

    pub fn as_function(&self) -> &ObjFunction {
        match &self.kind {
            ObjKind::Function(inner) => inner,
            _ => panic!("Ran as_function on something that is not a function"),
        }
    }
}

impl ObjFunction {
    pub fn new() -> ObjFunction {
        ObjFunction {
            arity: 0,
            name: None,
            chunk: Chunk::new(),
        }
    }
}
