use syntax;
use syntax::ast::MetaItemKind;
use syntax::ast::LitKind;
use syntax::ast::NestedMetaItemKind;

#[derive(Debug)]
pub struct Attr {
    pub name: String,
    pub values: Option<Vec<Attr>>,
}

impl Attr {
    fn new(name: String) -> Attr {
        Attr {
            name: name,
            values: None,
        }
    }
    pub fn has(&self, name: &str) -> bool {
        self.values.as_ref().map_or(false, |vec| {
            for item in vec {
                if item.name == name {
                    return true;
                }
            }
            return false;
        })
    }
    pub fn get(&self, name: &str) -> Option<&str> {
        self.values.as_ref().map_or(None, |vec| {
            for item in vec {
                if item.name != name {
                    continue;
                }
                return item.values.as_ref().map_or(None, |vec| {
                    match vec.len() {
                        0 => None,
                        1 => Some(&vec[0].name),
                        _ => unreachable!(),
                    }
                });
            }
            None

        })
    }
    pub fn get_attr(&self, name: &str) -> Option<&Attr> {
        self.values.as_ref().map_or(None, |vec| {
            for item in vec {
                if item.name != name {
                    continue;
                }
                return Some(&item);
            }
            None
        })
    }
}

pub fn visit_attrs(attrs: &Vec<syntax::ast::Attribute>) -> Attr {
    let mut ret = Attr::new(String::new());
    ret.values = Some(attrs.iter().map(|attr| visit_meta(&attr.value)).collect());
    ret
}

fn visit_meta(item: &syntax::ast::MetaItem) -> Attr {
    let name = item.name.as_str().to_string();
    let mut attr = Attr::new(name);
    match item.node {
        MetaItemKind::Word => attr,
        MetaItemKind::List(ref vec) => {
            attr.values = Some(vec.iter().map(|nest_item| visit_nest(&nest_item.node)).collect());
            attr
        }
        MetaItemKind::NameValue(ref lit) => {
            attr.values = Some(vec![visit_literal(&lit.node)]);
            attr
        }
    }
}

fn visit_nest(nest_item: &syntax::ast::NestedMetaItemKind) -> Attr {
    match nest_item {
        &NestedMetaItemKind::MetaItem(ref sub_item) => visit_meta(&sub_item),
        &NestedMetaItemKind::Literal(ref lit) => visit_literal(&lit.node),
    }
}

fn visit_literal(lit: &syntax::ast::LitKind) -> Attr {
    match lit {
        &LitKind::Str(symbol, _) => Attr::new(symbol.as_str().to_string()),
        // LitKind::ByteStr(Rc<Vec<u8>>),
        // LitKind::Byte(u8),
        // LitKind::Char(char),
        &LitKind::Int(value, _) => Attr::new(value.to_string()),
        // LitKind::Float(Symbol, FloatTy),
        // LitKind::FloatUnsuffixed(Symbol),
        &LitKind::Bool(value) => Attr::new(value.to_string()),
        _ => unreachable!(),
    }
}
