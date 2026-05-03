pub enum LawTag {
    Storage,
    Hunting,
    Sport,
}


pub struct Law {
    id: i64,
    name: String,
    region: String,
    text: String,
    version: i32,
    tags: Vec<LawTag>
}

impl Law {
    fn new() -> Self {
        todo!()
    }

    
}