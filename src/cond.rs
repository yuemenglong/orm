use mysql::Value;

#[derive(Debug, Clone)]
pub struct Cond {
    items: Vec<Item>,
}

impl Cond {
    pub fn new() -> Self {
        Cond { items: Vec::new() }
    }

    pub fn by_id<V>(id: V) -> Self
        where Value: From<V>
    {
        let mut cond = Cond::new();
        cond.id(id);
        cond
    }
    pub fn by_eq<V>(field: &str, value: V) -> Self
        where Value: From<V>
    {
        let mut cond = Cond::new();
        cond.eq(field, value);
        cond
    }

    pub fn id<V>(&mut self, id: V) -> &mut Self
        where Value: From<V>
    {
        self.items.push(Item::Id(Value::from(id)));
        self
    }
    pub fn eq<V>(&mut self, field: &str, value: V) -> &mut Self
        where Value: From<V>
    {
        self.items.push(Item::Eq(field.to_string(), Value::from(value)));
        self
    }
    pub fn gt<V>(&mut self, field: &str, value: V) -> &mut Self
        where Value: From<V>
    {
        self.items.push(Item::Gt(field.to_string(), Value::from(value)));
        self
    }

    pub fn to_sql(&self, alias: &str) -> String {
        self.items
            .iter()
            .map(|item| item.to_sql(alias))
            .collect::<Vec<_>>()
            .join(" AND ")
    }
    pub fn to_params(&self, alias: &str) -> Vec<(String, Value)> {
        self.items
            .iter()
            .map(|item| item.to_params(alias))
            .collect::<Vec<_>>()
    }
}

#[derive(Debug, Clone)]
enum Item {
    Id(Value),
    Eq(String, Value),
    Gt(String, Value),
}

fn concat(alias: &str, field: &str) -> String {
    format!("{}_{}", alias.to_lowercase(), field)
}

impl Item {
    fn to_sql(&self, alias: &str) -> String {
        match self {
            &Item::Id(..) => format!("{}.id = :{}", alias, concat(alias, "id")),
            &Item::Eq(ref field, ..) => format!("{}.{} = :{}", alias, field, concat(alias, field)),
            &Item::Gt(ref field, ..) => format!("{}.{} > :{}", alias, field, concat(alias, field)),
        }
    }
    fn to_params(&self, alias: &str) -> (String, Value) {
        match self {
            &Item::Id(ref value) => (concat(alias, "id"), value.clone()),
            &Item::Eq(ref field, ref value) => (concat(alias, field), value.clone()),
            &Item::Gt(ref field, ref value) => (concat(alias, field), value.clone()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct JoinCond {
    items: Vec<JoinItem>,
}

impl JoinCond {
    pub fn new() -> Self {
        JoinCond { items: Vec::new() }
    }
    pub fn by_eq(f1: &str, f2: &str) -> Self
    {
        let mut cond = JoinCond::new();
        cond.eq(f1, f2);
        cond
    }

    pub fn eq(&mut self, f1:&str, f2:&str) -> &mut Self
    {
        self.items.push(JoinItem::Eq(f1.to_string(), f2.to_string()));
        self
    }

    pub fn to_sql(&self, a1: &str, a2:&str) -> String {
        self.items
            .iter()
            .map(|item| item.to_sql(a1, a2))
            .collect::<Vec<_>>()
            .join(" AND ")
    }
}

#[derive(Debug, Clone)]
enum JoinItem {
    Eq(String, String),
    Ne(String, String),
    Gt(String, String),
    Lt(String, String),
}

impl JoinItem {
    fn to_sql(&self, a1: &str, a2: &str) -> String {
        match self {
            &JoinItem::Eq(ref f1, ref f2) => format!("{}.{} = {}.{}", a1, f1, a2, f2),
            &JoinItem::Ne(ref f1, ref f2) => format!("{}.{} <> {}.{}", a1, f1, a2, f2),
            &JoinItem::Gt(ref f1, ref f2) => format!("{}.{} > {}.{}", a1, f1, a2, f2),
            &JoinItem::Lt(ref f1, ref f2) => format!("{}.{} < {}.{}", a1, f1, a2, f2),
        }
    }
}
