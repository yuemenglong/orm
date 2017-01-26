use std::collections::HashMap;
use mysql::Value;
use mysql::Row;

use meta::EntityMeta;

pub trait Entity {
    fn get_meta() -> &'static EntityMeta;
    fn set_id(&mut self, id: u64);
    fn get_id(&self) -> u64;
    fn has_id(&self) -> bool;

    fn get_table_name() -> String {
        Self::get_meta().table_name.clone()
    }
    fn get_columns() -> Vec<String> {
        let entity_meta = Self::get_meta();
        entity_meta.fields
            .iter()
            .filter(|field| !field.pkey)
            .map(|field| field.field_name.clone())
            .collect::<Vec<_>>()
    }
    fn get_values(&self) -> Vec<Value>;
    fn get_params(&self) -> Vec<(String, Value)> {
        Self::get_columns().into_iter().zip(self.get_values().into_iter()).collect::<Vec<_>>()
    }


    fn sql_insert() -> String {
        let table_name = Self::get_table_name();
        let fields = Self::get_columns().join(", ");
        let values =
            Self::get_columns().iter().map(|column| format!(":{}", column)).collect::<Vec<_>>().join(", ");
        format!("INSERT INTO `{}`({}) VALUES ({})", &table_name, &fields, &values)
    }
    // fn get_name() -> String;
    // // fn get_field_meta() -> Vec<FieldMeta>;
    // fn get_params(&self) -> Vec<(String, Value)>;
    // fn from_row(row: Row) -> Self;
    // fn from_row_ex(row: Row, nameMap: &HashMap<String, String>) -> Self;

    // fn get_create_table() -> String;
    // fn get_drop_table() -> String;

    // fn get_field_list() -> String;
    // fn get_prepare() -> String;
    // fn get_params_id(&self) -> Vec<(String, Value)>;
    //  {
    //     vec![("id".to_string(), Value::from(self.get_id()))]
    // }
}
