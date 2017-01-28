use syntax;
use syntax::ast::MetaItemKind;
use syntax::ast::LitKind;
use syntax::ast::NestedMetaItemKind;

#[derive(Debug)]
pub enum Annotation {
    Id(bool),
    Len(u64),
    Nullable(bool),
    Pointer,
    HasOne(String),
    HasMany(String),
    ManyToMany(String),
}

pub fn visit_struct_field_attr(attr: &syntax::ast::Attribute) -> Annotation {
    match attr.value.name.as_str().to_string().as_ref() {
        "id" => visit_anno_id(attr),
        "len" => visit_anno_len(attr),
        "nullable" => visit_anno_nullable(attr),
        "pointer" => visit_anno_pointer(attr),
        _ => unreachable!(),
    }
}

fn visit_anno_id(attr: &syntax::ast::Attribute) -> Annotation {
    if let MetaItemKind::Word = attr.value.node {
        return Annotation::Id(false);
    }
    if let MetaItemKind::List(ref vec) = attr.value.node {
        if vec.len() != 1 {
            return unreachable!();
        }
        if let NestedMetaItemKind::MetaItem(ref meta_item) = vec[0].node {
            if let MetaItemKind::Word = meta_item.node {
                if meta_item.name.as_str().to_string() == String::from("auto") {
                    return Annotation::Id(true);
                }
            }
        }
    }
    return unreachable!();
}

fn visit_anno_len(attr: &syntax::ast::Attribute) -> Annotation {
    if let MetaItemKind::List(ref vec) = attr.value.node {
        if vec.len() == 1 {
            if let NestedMetaItemKind::Literal(ref lit) = vec[0].node {
                if let LitKind::Int(u, _) = lit.node {
                    return Annotation::Len(u);
                }
            }
        }
    }
    unreachable!()
}

fn visit_anno_nullable(attr: &syntax::ast::Attribute) -> Annotation {
    if let MetaItemKind::List(ref vec) = attr.value.node {
        if vec.len() == 1 {
            if let NestedMetaItemKind::Literal(ref lit) = vec[0].node {
                if let LitKind::Bool(b) = lit.node {
                    return Annotation::Nullable(b);
                }
            }
        }
    }
    unreachable!()
}

fn visit_anno_pointer(attr: &syntax::ast::Attribute) -> Annotation {
    if let MetaItemKind::Word = attr.value.node{
        return Annotation::Pointer;
    }
    unreachable!()
}


// fn visit_struct_field_attr(&mut self, attr: &syntax::ast::Attribute) {
//         self.visit_meta_item(&attr.value);
//     }
//     fn visit_meta_item(&mut self, item: &syntax::ast::MetaItem) {
//         println!("MetaItem Name: {:?}", item.name);
//         match item.node {
//             MetaItemKind::Word => {
//                 println!("MetaItemKind::Word");
//             }
//             MetaItemKind::List(ref vec) => {
//                 println!("MetaItemKind::List");
//                 for item in vec {
//                     self.visit_nest_meta_item(&item);
//                 }
//             }
//             MetaItemKind::NameValue(ref lit) => {
//                 println!("MetaItemKind::NameValue");
//                 self.visit_lit_meta_item(lit);
//             }
//         }
//     }
//     fn visit_nest_meta_item(&mut self, item: &syntax::ast::NestedMetaItem) {
//         match item.node {
//             NestedMetaItemKind::MetaItem(ref item) => {
//                 self.visit_meta_item(&item);
//             }
//             _ => {}

//         }
//     }
//     fn visit_lit_meta_item(&mut self, lit: &syntax::ast::Lit) {
//         match lit.node {
//             LitKind::Str(ref symbol, ref _str_style) => {
//                 println!("Lit Value: {:?}", symbol.as_str());
//             }
//             _ => {}
//         }
//     }
