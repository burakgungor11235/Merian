use std::collections::HashMap;

use logos::Span;

use super::solve::MathValue;

pub type ScopeId = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Inferred,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Symbol {
    pub name: String,
    pub value: MathValue,
    pub kind: SymbolKind,
    pub declaration_order: u32,
    pub origin: Span,
    pub scope: ScopeId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Scope {
    pub parent: Option<ScopeId>,
    pub origin: Span,
    pub bindings: HashMap<String, Symbol>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScopeArena {
    scopes: Vec<Scope>,
    next_order: u32,
}

impl ScopeArena {
    pub fn document() -> Self {
        Self {
            scopes: vec![Scope {
                parent: None,
                origin: 0..0,
                bindings: HashMap::new(),
            }],
            next_order: 0,
        }
    }

    pub fn root(&self) -> ScopeId {
        0
    }

    pub fn get(&self, scope: ScopeId, name: &str) -> Option<&Symbol> {
        let mut current = Some(scope);
        while let Some(id) = current {
            let scope = self.scopes.get(id as usize)?;
            if let Some(symbol) = scope.bindings.get(name) {
                return Some(symbol);
            }
            current = scope.parent;
        }
        None
    }

    /// Last write wins, the order still advances so queries can tell them apart.
    pub fn bind(&mut self, scope: ScopeId, name: &str, value: MathValue, origin: Span) {
        let symbol = Symbol {
            name: name.to_owned(),
            value,
            kind: SymbolKind::Inferred,
            declaration_order: self.next_order,
            origin,
            scope,
        };
        self.next_order += 1;
        self.scopes[scope as usize]
            .bindings
            .insert(name.to_owned(), symbol);
    }

    pub fn scopes(&self) -> &[Scope] {
        &self.scopes
    }

    pub fn dump(&self) -> String {
        let mut out = format!("scopes: {}\n", self.scopes.len());
        for (id, scope) in self.scopes.iter().enumerate() {
            out.push_str(&format!(
                "scope {id} parent={:?} origin={:?} bindings={}\n",
                scope.parent,
                scope.origin,
                scope.bindings.len(),
            ));
            let mut symbols: Vec<&Symbol> = scope.bindings.values().collect();
            symbols.sort_by_key(|symbol| symbol.declaration_order);
            for symbol in symbols {
                out.push_str(&format!(
                    "  {} = {} ({:?}, order {}, span {:?})\n",
                    symbol.name,
                    symbol.value.to_plain_string(),
                    symbol.kind,
                    symbol.declaration_order,
                    symbol.origin,
                ));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value(text: &str) -> MathValue {
        MathValue::from_decimal_str(text).unwrap()
    }

    #[test]
    fn document_starts_with_one_empty_root_scope() {
        let tree = ScopeArena::document();
        assert_eq!(tree.scopes().len(), 1);
        assert_eq!(tree.root(), 0);
        assert!(tree.scopes()[0].parent.is_none());
        assert!(tree.get(0, "x").is_none());
    }

    #[test]
    fn bindings_are_read_back_in_declaration_order() {
        let mut tree = ScopeArena::document();
        tree.bind(0, "x", value("5"), 10..16);
        tree.bind(0, "y", value("7"), 30..36);

        let x = tree.get(0, "x").unwrap();
        assert_eq!(x.value.to_plain_string(), "5");
        assert_eq!(x.declaration_order, 0);
        assert_eq!(x.origin, 10..16);

        assert_eq!(tree.get(0, "y").unwrap().declaration_order, 1);
        assert!(tree.get(0, "z").is_none());
    }

    #[test]
    fn redeclaring_a_name_silently_overwrites() {
        let mut tree = ScopeArena::document();
        tree.bind(0, "x", value("5"), 0..6);
        tree.bind(0, "x", value("7"), 20..26);

        let x = tree.get(0, "x").unwrap();
        assert_eq!(x.value.to_plain_string(), "7");
        assert_eq!(x.declaration_order, 1);
        assert_eq!(tree.scopes()[0].bindings.len(), 1);
    }

    #[test]
    fn lookup_walks_the_parent_chain() {
        let mut tree = ScopeArena::document();
        tree.bind(0, "x", value("5"), 0..6);
        tree.scopes.push(Scope {
            parent: Some(0),
            origin: 40..50,
            bindings: HashMap::new(),
        });
        tree.scopes.push(Scope {
            parent: Some(1),
            origin: 60..70,
            bindings: HashMap::new(),
        });

        assert_eq!(tree.get(2, "x").unwrap().value.to_plain_string(), "5");

        tree.bind(1, "x", value("9"), 44..50);
        assert_eq!(tree.get(1, "x").unwrap().value.to_plain_string(), "9");
        assert_eq!(tree.get(2, "x").unwrap().value.to_plain_string(), "9");
        assert_eq!(tree.get(0, "x").unwrap().value.to_plain_string(), "5");
        assert!(tree.get(0, "child_only").is_none());
    }
}
