use std::fmt::{self, Display};

#[derive(Debug, Clone)]
pub enum Type {
    Address,
    FixedBytes(usize),
    Bytes,
    Int(usize),
    Uint(usize),
    Bool,
    String,
    FixedArray(Box<Type>, usize),
    Array(Box<Type>),
    Tuple(Vec<Type>),
}

pub type SchemaPart = Vec<(String, Type)>;

pub struct Schema {
    parts: SchemaPart,
}

pub struct SchemaBuilder {
    parts: SchemaPart,
}

impl Default for SchemaBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SchemaBuilder {
    pub fn new() -> Self {
        SchemaBuilder { parts: vec![] }
    }

    pub fn add(mut self, name: &str, token_type: Type) -> Self {
        self.parts.push((name.to_owned(), token_type));
        self
    }

    pub fn build(self) -> Schema {
        Schema { parts: self.parts }
    }
}

fn token_type_to_abi_type(token_type: &Type) -> String {
    match token_type {
        Type::Address => "address".into(),
        Type::FixedBytes(size) => format!("bytes{}", size),
        Type::Bytes => "bytes".into(),
        Type::Int(size) => format!("int{}", size),
        Type::Uint(size) => format!("uint{}", size),
        Type::Bool => "bool".into(),
        Type::String => "string".into(),
        Type::FixedArray(token_type, size) => {
            format!("{}[{}]", token_type_to_abi_type(token_type), size)
        }
        Type::Array(token_type) => {
            format!("{}[]", token_type_to_abi_type(token_type))
        }
        Type::Tuple(types) => {
            let inner_types = types
                .iter()
                .map(token_type_to_abi_type)
                .collect::<Vec<String>>()
                .join(",");
            format!("({})", inner_types)
        }
    }
}

impl Display for Schema {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let parts = self
            .parts
            .iter()
            .map(|(name, token_type)| format!("{} {}", token_type_to_abi_type(token_type), name))
            .collect::<Vec<String>>()
            .join(", ");
        write!(f, "{}", parts)
    }
}

impl From<Schema> for String {
    fn from(schema: Schema) -> Self {
        schema.to_string()
    }
}

#[cfg(test)]
mod tests {
    use crate::schema::{SchemaBuilder, Type};

    #[test]
    fn schema_test() {
        let schema = SchemaBuilder::new()
            .add("name", Type::String)
            .add("age", Type::Uint(8))
            .add("address", Type::Address)
            .add("data", Type::FixedArray(Box::new(Type::Uint(8)), 3))
            .add("data2", Type::Array(Box::new(Type::Uint(8))))
            .add("data3", Type::Tuple(vec![Type::Uint(8), Type::Uint(8)]))
            .build();
        assert_eq!(schema.to_string(), "string name, uint8 age, address address, uint8[3] data, uint8[] data2, (uint8,uint8) data3");
    }
}
