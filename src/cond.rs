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
    pub fn by_ne<V>(field: &str, value: V) -> Self
        where Value: From<V>
    {
        let mut cond = Cond::new();
        cond.ne(field, value);
        cond
    }
    pub fn by_gt<V>(field: &str, value: V) -> Self
        where Value: From<V>
    {
        let mut cond = Cond::new();
        cond.gt(field, value);
        cond
    }
    pub fn by_lt<V>(field: &str, value: V) -> Self
        where Value: From<V>
    {
        let mut cond = Cond::new();
        cond.lt(field, value);
        cond
    }
    pub fn by_is_null(field: &str) -> Self {
        let mut cond = Cond::new();
        cond.is_null(field);
        cond
    }
    pub fn by_not_null(field: &str) -> Self {
    let mut cond = Cond::new();
        cond.not_null(field);
        cond
    }
}

impl Cond {
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
    pub fn ne<V>(&mut self, field: &str, value: V) -> &mut Self
        where Value: From<V>
    {
        self.items.push(Item::Ne(field.to_string(), Value::from(value)));
        self
    }
    pub fn gt<V>(&mut self, field: &str, value: V) -> &mut Self
        where Value: From<V>
    {
        self.items.push(Item::Gt(field.to_string(), Value::from(value)));
        self
    }
    pub fn lt<V>(&mut self, field: &str, value: V) -> &mut Self
        where Value: From<V>
    {
        self.items.push(Item::Lt(field.to_string(), Value::from(value)));
        self
    }
    pub fn is_null(&mut self, field: &str) -> &mut Self {
        self.items.push(Item::Null(field.to_string()));
        self
    }
    pub fn not_null(&mut self, field: &str) -> &mut Self {
        self.items.push(Item::NotNull(field.to_string()));
        self
    }
}

impl Cond{
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
            .flat_map(|item| item.to_params(alias))
            .collect::<Vec<_>>()
    }
}

#[derive(Debug, Clone)]
enum Item {
    Id(Value),
    Eq(String, Value),
    Ne(String, Value),
    Gt(String, Value),
    Lt(String, Value),
    Null(String),
    NotNull(String),
}

fn concat(alias: &str, field: &str) -> String {
    format!("{}_{}", alias, field)
}

impl Item {
    fn to_sql(&self, alias: &str) -> String {
        match self {
            &Item::Id(..) => format!("{}.id = :{}", alias, concat(alias, "id")),
            &Item::Eq(ref field, ..) => format!("{}.{} = :{}", alias, field, concat(alias, field)),
            &Item::Ne(ref field, ..) => format!("{}.{} <> :{}", alias, field, concat(alias, field)),
            &Item::Gt(ref field, ..) => format!("{}.{} > :{}", alias, field, concat(alias, field)),
            &Item::Lt(ref field, ..) => format!("{}.{} < :{}", alias, field, concat(alias, field)),
            &Item::Null(ref field) => format!("{}.{} IS NULL", alias, field),
            &Item::NotNull(ref field) => format!("{}.{} IS NOT NULL", alias, field),
        }
    }
    fn to_params(&self, alias: &str) -> Vec<(String, Value)> {
        match self {
            &Item::Id(ref value) => vec![(concat(alias, "id"), value.clone())],
            &Item::Eq(ref field, ref value) |
            &Item::Ne(ref field, ref value) |
            &Item::Gt(ref field, ref value) |
            &Item::Lt(ref field, ref value) => vec![(concat(alias, field), value.clone())],
            &Item::Null(..) |
            &Item::NotNull(..) => Vec::new(),
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
    pub fn by_eq(f1: &str, f2: &str) -> Self {
        let mut cond = JoinCond::new();
        cond.eq(f1, f2);
        cond
    }
    pub fn by_ne(f1: &str, f2: &str) -> Self {
        let mut cond = JoinCond::new();
        cond.ne(f1, f2);
        cond
    }
    pub fn by_gt(f1: &str, f2: &str) -> Self {
        let mut cond = JoinCond::new();
        cond.gt(f1, f2);
        cond
    }
    pub fn by_lt(f1: &str, f2: &str) -> Self {
        let mut cond = JoinCond::new();
        cond.lt(f1, f2);
        cond
    }
}

impl JoinCond {
    pub fn eq(&mut self, f1: &str, f2: &str) -> &mut Self {
        self.items.push(JoinItem::Eq(f1.to_string(), f2.to_string()));
        self
    }
    pub fn ne(&mut self, f1: &str, f2: &str) -> &mut Self {
        self.items.push(JoinItem::Ne(f1.to_string(), f2.to_string()));
        self
    }
    pub fn gt(&mut self, f1: &str, f2: &str) -> &mut Self {
        self.items.push(JoinItem::Gt(f1.to_string(), f2.to_string()));
        self
    }
    pub fn lt(&mut self, f1: &str, f2: &str) -> &mut Self {
        self.items.push(JoinItem::Lt(f1.to_string(), f2.to_string()));
        self
    }
    // pub fn eqv<V>(&mut self, f1: &str, value: V) -> &mut Self
    //     where Value: From<V>
    // {
    //     self.items.push(JoinItem::EqV(f1.to_string(), Value::from(value)));
    //     self
    // }
    // pub fn nev<V>(&mut self, f1: &str, value: V) -> &mut Self
    //     where Value: From<V>
    // {
    //     self.items.push(JoinItem::NeV(f1.to_string(), Value::from(value)));
    //     self
    // }
    // pub fn gtv<V>(&mut self, f1: &str, value: V) -> &mut Self
    //     where Value: From<V>
    // {
    //     self.items.push(JoinItem::GtV(f1.to_string(), Value::from(value)));
    //     self
    // }
    // pub fn ltv<V>(&mut self, f1: &str, value: V) -> &mut Self
    //     where Value: From<V>
    // {
    //     self.items.push(JoinItem::LtV(f1.to_string(), Value::from(value)));
    //     self
    // }

    pub fn to_sql(&self, a1: &str, a2: &str) -> String {
        self.items
            .iter()
            .map(|item| item.to_sql(a1, a2))
            .collect::<Vec<_>>()
            .join(" AND ")
    }
    // pub fn to_params(&self, a1: &str, a2: &str) -> Vec<(String, Value)> {
    //     self.items
    //         .iter()
    //         .flat_map(|item| item.to_params(a1, a2))
    //         .collect::<Vec<_>>()
    // }
}

#[derive(Debug, Clone)]
enum JoinItem {
    Eq(String, String),
    Ne(String, String),
    Gt(String, String),
    Lt(String, String), /* EqV(String, Value),
                         * NeV(String, Value),
                         * GtV(String, Value),
                         * LtV(String, Value), */
}

impl JoinItem {
    fn to_sql(&self, a1: &str, a2: &str) -> String {
        match self {
            &JoinItem::Eq(ref f1, ref f2) => format!("{}.{} = {}.{}", a1, f1, a2, f2),
            &JoinItem::Ne(ref f1, ref f2) => format!("{}.{} <> {}.{}", a1, f1, a2, f2),
            &JoinItem::Gt(ref f1, ref f2) => format!("{}.{} > {}.{}", a1, f1, a2, f2),
            &JoinItem::Lt(ref f1, ref f2) => format!("{}.{} < {}.{}", a1, f1, a2, f2),
            // &JoinItem::EqV(ref f, ..) => format!("{}.{} = :{}", a1, f, concat(a1, f)),
            // &JoinItem::NeV(ref f, ..) => format!("{}.{} <> :{}", a1, f, concat(a1, f)),
            // &JoinItem::GtV(ref f, ..) => format!("{}.{} > :{}", a1, f, concat(a1, f)),
            // &JoinItem::LtV(ref f, ..) => format!("{}.{} < :{}", a1, f, concat(a1, f)),
        }
    }
    // fn to_params(&self, a1: &str, a2: &str) -> Vec<(String, Value)> {
    //     match self {
    //         &JoinItem::Eq(..) |
    //         &JoinItem::Ne(..) |
    //         &JoinItem::Gt(..) |
    //         &JoinItem::Lt(..) => Vec::new(),
    //         &JoinItem::EqV(ref f, ref v) |
    //         &JoinItem::NeV(ref f, ref v) |
    //         &JoinItem::GtV(ref f, ref v) |
    //         &JoinItem::LtV(ref f, ref v) => vec![(concat(a1, f), v.clone())],
    //     }
    // }
}
