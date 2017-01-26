use std::collections::HashMap;
use mysql::Value;
use mysql::Row;

use meta::EntityMeta;

pub trait Entity {
    fn get_meta() -> &'static EntityMeta;
    fn get_create_table() -> String {
        let entity_meta = Self::get_meta();
        let fields = entity_meta.fields
            .iter()
            .map(|field| field.db_ty.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        format!("CREATE TABLE IF NOT EXISTS `{}`({})",
                entity_meta.table_name,
                fields)
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

    // fn set_id(&mut self, id: u64);
    // fn get_id(&self) -> Option<u64>;
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
