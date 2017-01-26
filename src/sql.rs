use meta;

pub fn sql_create_table(entity_meta: &meta::EntityMeta) -> String {
    let fields = entity_meta.fields
        .iter()
        .map(|field| field.db_ty.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    format!("CREATE TABLE IF NOT EXISTS `{}`({})",
            entity_meta.table_name,
            fields)
}
pub fn sql_drop_table(entity_meta: &meta::EntityMeta) -> String {
    format!("DROP TABLE IF EXISTS `{}`", entity_meta.table_name)
}