#[macro_use]
use macros;
use meta::EntityMeta;

use mysql::conn::GenericConnection;
use mysql::Error;

pub fn create<C>(conn: &mut C, meta: &EntityMeta) -> Result<u64, Error>
    where C: GenericConnection
{
    let fields = meta.get_non_refer_fields()
        .iter()
        .map(|field| field.get_db_type())
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!("CREATE TABLE IF NOT EXISTS `{}`({})", meta.table, fields);
    log!("{}", sql);
    conn.prep_exec(sql, ()).map(|res| res.affected_rows())
}

pub fn drop<C>(conn: &mut C, meta: &EntityMeta) -> Result<u64, Error>
    where C: GenericConnection
{
    let fields = meta.get_non_refer_fields()
        .iter()
        .map(|field| field.get_db_type())
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!("DROP TABLE IF EXISTS `{}`", meta.table);
    log!("{}", sql);
    conn.prep_exec(sql, ()).map(|res| res.affected_rows())
}
