use std::collections::HashMap;
use mysql::Value;
use mysql::Row;

use meta::EntityMeta;
use sql;

pub trait Entity {
    fn get_meta() -> &'static EntityMeta;
    fn set_id(&mut self, id: u64);
    fn get_id(&self) -> u64;
    fn has_id(&self) -> bool;

    fn get_columns() -> Vec<String> {
        sql::entity_get_columns(Self::get_meta())
    }
    fn get_values(&self) -> Vec<Value>;
    fn get_params(&self) -> Vec<(String, Value)> {
        Self::get_columns().into_iter().zip(self.get_values().into_iter()).collect::<Vec<_>>()
    }
    fn set_values(&mut self, row: &mut Row, prefix: &str);

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
