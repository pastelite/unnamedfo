// Grammar
// field: a b | c => field(a and (b or c))
// field: a field2: d =>

enum Operation {
    Field(String, Vec<Operation>),
    Value(String),
    And(Vec<Operation>),
    Or(Vec<Operation>),
}
